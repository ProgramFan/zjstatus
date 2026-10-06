use std::collections::BTreeMap;

#[cfg(all(not(feature = "bench"), not(test)))]
use zellij_tile::shim::run_command;

use crate::render::FormattedPart;
use crate::{
    config::ZellijState,
    widgets::{command::lock, widget::Widget},
};

/// Key of the command result that holds the host name.
pub const RESULT_NAME: &str = "hostname";

pub struct HostnameWidget {
    format: Vec<FormattedPart>,
    short: bool,
}

impl HostnameWidget {
    pub fn new(config: &BTreeMap<String, String>) -> Self {
        let mut format: Vec<FormattedPart> = Vec::new();
        if let Some(form) = config.get("hostname_format") {
            format = FormattedPart::multiple_from_format_string(form, config);
        }

        let short = match config.get("hostname_short") {
            Some(short) => short == "true",
            None => false,
        };

        Self { format, short }
    }
}

impl Widget for HostnameWidget {
    fn process(&self, _name: &str, state: &ZellijState) -> String {
        let result = match state.command_results.get(RESULT_NAME) {
            Some(result) => result,
            None => {
                request_hostname(state);

                return "".to_owned();
            }
        };

        let mut name = result.stdout.trim();
        if self.short {
            name = name.split('.').next().unwrap_or(name);
        }

        if name.is_empty() {
            return "".to_owned();
        }

        if self.format.is_empty() {
            return name.to_owned();
        }

        let mut output = "".to_owned();

        for f in &self.format {
            let mut content = f.content.clone();

            if content.contains("{name}") {
                content = content.replace("{name}", name);
            }

            output = format!("{}{}", output, f.format_string(&content));
        }

        output
    }

    fn process_click(&self, _name: &str, _state: &ZellijState, _pos: usize) {}
}

// The plugin is sandboxed, so the host needs to be asked for its name. It does not
// change, which is why this happens only once.
fn request_hostname(state: &ZellijState) {
    if lock(RESULT_NAME, state.clone()) {
        return;
    }

    let context = BTreeMap::from([("name".to_owned(), RESULT_NAME.to_owned())]);

    tracing::debug!("Requesting host name {:?}", context);

    #[cfg(all(not(feature = "bench"), not(test)))]
    run_command(&["uname", "-n"], context);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::widgets::command::CommandResult;
    use rstest::rstest;

    #[rstest]
    // plain host name, as reported
    #[case(&[], Some("box.example.org\n"), "box.example.org")]
    #[case(&[("hostname_short", "true")], Some("box.example.org\n"), "box")]
    #[case(&[("hostname_short", "true")], Some("box\n"), "box")]
    #[case(&[("hostname_format", "#[fg=red] {name} ")], Some("box\n"), " box ")]
    #[case(&[("hostname_format", "#[fg=red]<#[fg=blue]{name}#[fg=red]>")], Some("box\n"), "<box>")]
    // nothing is rendered as long as the name is unknown
    #[case(&[("hostname_format", "#[fg=red] {name} ")], Some("\n"), "")]
    #[case(&[("hostname_format", "#[fg=red] {name} ")], None, "")]
    pub fn test_hostname(
        #[case] config: &[(&str, &str)],
        #[case] stdout: Option<&str>,
        #[case] expected: &str,
    ) {
        let config = config
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();

        let mut state = ZellijState {
            plugin_uuid: "test_hostname".to_owned(),
            ..ZellijState::default()
        };
        if let Some(stdout) = stdout {
            state.command_results.insert(
                RESULT_NAME.to_owned(),
                CommandResult {
                    exit_code: Some(0),
                    stdout: stdout.to_owned(),
                    ..CommandResult::default()
                },
            );
        }

        let output = HostnameWidget::new(&config).process("hostname", &state);

        assert_eq!(console::strip_ansi_codes(&output), expected);
    }
}
