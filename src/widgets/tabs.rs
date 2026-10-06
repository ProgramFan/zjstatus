use std::{cmp, collections::BTreeMap, ops::Range};

use zellij_tile::{
    prelude::{InputMode, ModeInfo, PaneInfo, PaneManifest, TabInfo},
    shim::{new_tab, switch_tab_to},
};

use crate::{config::ZellijState, render::FormattedPart};

use super::widget::Widget;

pub struct TabsWidget {
    active_tab_format: Vec<FormattedPart>,
    active_tab_fullscreen_format: Vec<FormattedPart>,
    active_tab_sync_format: Vec<FormattedPart>,
    normal_tab_format: Vec<FormattedPart>,
    normal_tab_fullscreen_format: Vec<FormattedPart>,
    normal_tab_sync_format: Vec<FormattedPart>,
    normal_tab_bell_format: Option<Vec<FormattedPart>>,
    normal_tab_flashing_bell_format: Option<Vec<FormattedPart>>,
    rename_tab_format: Vec<FormattedPart>,
    separator: Option<FormattedPart>,
    fullscreen_indicator: Option<String>,
    floating_indicator: Option<String>,
    sync_indicator: Option<String>,
    bell_indicator: Option<String>,
    flashing_bell_indicator: Option<String>,
    tab_display_count: Option<usize>,
    tab_truncate_start_format: Vec<FormattedPart>,
    tab_truncate_end_format: Vec<FormattedPart>,
    tab_zero_based_index: bool,
    new_tab_button: Vec<FormattedPart>,
}

impl TabsWidget {
    pub fn new(config: &BTreeMap<String, String>) -> Self {
        let mut normal_tab_format: Vec<FormattedPart> = Vec::new();
        if let Some(form) = config.get("tab_normal") {
            normal_tab_format = FormattedPart::multiple_from_format_string(form, config);
        }

        let normal_tab_fullscreen_format = match config.get("tab_normal_fullscreen") {
            Some(form) => FormattedPart::multiple_from_format_string(form, config),
            None => normal_tab_format.clone(),
        };

        let normal_tab_sync_format = match config.get("tab_normal_sync") {
            Some(form) => FormattedPart::multiple_from_format_string(form, config),
            None => normal_tab_format.clone(),
        };

        let normal_tab_bell_format = config
            .get("tab_normal_bell")
            .map(|form| FormattedPart::multiple_from_format_string(form, config));

        let normal_tab_flashing_bell_format = config
            .get("tab_normal_flashing_bell")
            .map(|form| FormattedPart::multiple_from_format_string(form, config));

        let mut active_tab_format = normal_tab_format.clone();
        if let Some(form) = config.get("tab_active") {
            active_tab_format = FormattedPart::multiple_from_format_string(form, config);
        }

        let active_tab_fullscreen_format = match config.get("tab_active_fullscreen") {
            Some(form) => FormattedPart::multiple_from_format_string(form, config),
            None => active_tab_format.clone(),
        };

        let active_tab_sync_format = match config.get("tab_active_sync") {
            Some(form) => FormattedPart::multiple_from_format_string(form, config),
            None => active_tab_format.clone(),
        };

        let rename_tab_format = match config.get("tab_rename") {
            Some(form) => FormattedPart::multiple_from_format_string(form, config),
            None => active_tab_format.clone(),
        };

        let tab_display_count = match config.get("tab_display_count") {
            Some(count) => count.parse::<usize>().ok(),
            None => None,
        };

        let tab_truncate_start_format = config
            .get("tab_truncate_start_format")
            .map(|form| FormattedPart::multiple_from_format_string(form, config))
            .unwrap_or_default();

        let tab_truncate_end_format = config
            .get("tab_truncate_end_format")
            .map(|form| FormattedPart::multiple_from_format_string(form, config))
            .unwrap_or_default();

        let tab_zero_based_index = match config.get("tab_zero_based_index") {
            Some(e) => matches!(e.as_str(), "true"),
            None => false,
        };

        let new_tab_button = config
            .get("tab_new_button")
            .map(|form| FormattedPart::multiple_from_format_string(form, config))
            .unwrap_or_default();

        let separator = config
            .get("tab_separator")
            .map(|s| FormattedPart::from_format_string(s, config));

        let bell_indicator = config.get("tab_bell_indicator").cloned();
        let flashing_bell_indicator = config
            .get("tab_flashing_bell_indicator")
            .cloned()
            .or_else(|| bell_indicator.clone());

        Self {
            normal_tab_format,
            normal_tab_fullscreen_format,
            normal_tab_sync_format,
            normal_tab_bell_format,
            normal_tab_flashing_bell_format,
            active_tab_format,
            active_tab_fullscreen_format,
            active_tab_sync_format,
            rename_tab_format,
            separator,
            floating_indicator: config.get("tab_floating_indicator").cloned(),
            sync_indicator: config.get("tab_sync_indicator").cloned(),
            fullscreen_indicator: config.get("tab_fullscreen_indicator").cloned(),
            bell_indicator,
            flashing_bell_indicator,
            tab_display_count,
            tab_truncate_start_format,
            tab_truncate_end_format,
            tab_zero_based_index,
            new_tab_button,
        }
    }
}

impl Widget for TabsWidget {
    fn process(&self, _name: &str, state: &ZellijState) -> String {
        format!(
            "{}{}",
            self.render_tabs(state),
            self.render_new_tab_button()
        )
    }

    fn process_click(&self, _name: &str, state: &ZellijState, pos: usize) {
        if self.new_tab_button_range(state).contains(&pos) {
            new_tab::<&str>(None, None);

            return;
        }

        let mut offset = 0;
        let mut counter = 0;

        let (truncated_start, truncated_end, tabs) =
            get_tab_window(&state.tabs, self.tab_display_count);

        let active_pos = &state
            .tabs
            .iter()
            .find(|t| t.active)
            .expect("no active tab")
            .position
            + 1;

        if truncated_start > 0 {
            for f in &self.tab_truncate_start_format {
                let mut content = f.content.clone();

                if content.contains("{count}") {
                    content = content.replace("{count}", (truncated_end).to_string().as_str());
                }

                offset += console::measure_text_width(&f.format_string(&content));

                if pos <= offset {
                    switch_tab_to(active_pos.saturating_sub(1) as u32);
                }
            }
        }

        for tab in &tabs {
            counter += 1;

            let mut rendered_content = self.render_tab(tab, &state.panes, &state.mode);

            if counter < tabs.len()
                && let Some(sep) = &self.separator
            {
                rendered_content =
                    format!("{}{}", rendered_content, sep.format_string(&sep.content));
            }

            let content_len = console::measure_text_width(&rendered_content);

            if pos > offset && pos < offset + content_len {
                switch_tab_to(tab.position as u32 + 1);

                break;
            }

            offset += content_len;
        }

        if truncated_end > 0 {
            for f in &self.tab_truncate_end_format {
                let mut content = f.content.clone();

                if content.contains("{count}") {
                    content = content.replace("{count}", (truncated_end).to_string().as_str());
                }

                offset += console::measure_text_width(&f.format_string(&content));

                if pos <= offset {
                    switch_tab_to(cmp::min(active_pos + 1, state.tabs.len()) as u32);
                }
            }
        }
    }
}

impl TabsWidget {
    fn render_tabs(&self, state: &ZellijState) -> String {
        let mut output = "".to_owned();
        let mut counter = 0;

        let (truncated_start, truncated_end, tabs) =
            get_tab_window(&state.tabs, self.tab_display_count);

        if truncated_start > 0 {
            for f in &self.tab_truncate_start_format {
                let mut content = f.content.clone();

                if content.contains("{count}") {
                    content = content.replace("{count}", (truncated_start).to_string().as_str());
                }

                output = format!("{output}{}", f.format_string(&content));
            }
        }

        for tab in &tabs {
            let content = self.render_tab(tab, &state.panes, &state.mode);
            counter += 1;

            output = format!("{}{}", output, content);

            if counter < tabs.len()
                && let Some(sep) = &self.separator
            {
                output = format!("{}{}", output, sep.format_string(&sep.content));
            }
        }

        if truncated_end > 0 {
            for f in &self.tab_truncate_end_format {
                let mut content = f.content.clone();

                if content.contains("{count}") {
                    content = content.replace("{count}", (truncated_end).to_string().as_str());
                }

                output = format!("{output}{}", f.format_string(&content));
            }
        }

        output
    }

    fn render_new_tab_button(&self) -> String {
        self.new_tab_button.iter().fold("".to_owned(), |output, f| {
            format!("{output}{}", f.format_string(&f.content))
        })
    }

    // columns covered by the new tab button, relative to the start of the widget
    fn new_tab_button_range(&self, state: &ZellijState) -> Range<usize> {
        let start = console::measure_text_width(&self.render_tabs(state));

        start..start + console::measure_text_width(&self.render_new_tab_button())
    }

    fn select_format(&self, info: &TabInfo, mode: &ModeInfo) -> &Vec<FormattedPart> {
        if info.active && mode.mode == InputMode::RenameTab {
            return &self.rename_tab_format;
        }

        if !info.active && info.is_flashing_bell {
            let fmt = self
                .normal_tab_flashing_bell_format
                .as_ref()
                .or(self.normal_tab_bell_format.as_ref());
            if let Some(fmt) = fmt {
                return fmt;
            }
        }

        if !info.active
            && info.has_bell_notification
            && let Some(fmt) = self.normal_tab_bell_format.as_ref()
        {
            return fmt;
        }

        if info.active && info.is_fullscreen_active {
            return &self.active_tab_fullscreen_format;
        }

        if info.active && info.is_sync_panes_active {
            return &self.active_tab_sync_format;
        }

        if info.active {
            return &self.active_tab_format;
        }

        if info.is_fullscreen_active {
            return &self.normal_tab_fullscreen_format;
        }

        if info.is_sync_panes_active {
            return &self.normal_tab_sync_format;
        }

        &self.normal_tab_format
    }

    fn render_tab(&self, tab: &TabInfo, panes: &PaneManifest, mode: &ModeInfo) -> String {
        let formatters = self.select_format(tab, mode);
        let mut output = "".to_owned();

        for f in formatters.iter() {
            let mut content = f.content.clone();

            let tab_name = match mode.mode {
                InputMode::RenameTab => match tab.name.is_empty() {
                    true => "Enter name...",
                    false => tab.name.as_str(),
                },
                _name => tab.name.as_str(),
            };

            if content.contains("{name}") {
                content = content.replace("{name}", tab_name);
            }

            if content.contains("{index}") {
                let index = match self.tab_zero_based_index {
                    true => tab.position,
                    false => tab.position + 1,
                };
                content = content.replace("{index}", index.to_string().as_str());
            }

            if content.contains("{floating_total_count}") {
                let panes_for_tab: Vec<PaneInfo> =
                    panes.panes.get(&tab.position).cloned().unwrap_or_default();

                content = content.replace(
                    "{floating_total_count}",
                    &format!("{}", panes_for_tab.iter().filter(|p| p.is_floating).count()),
                );
            }

            if content.contains("{focused_pane_title}") {
                let panes_for_tab: Vec<PaneInfo> =
                    panes.panes.get(&tab.position).cloned().unwrap_or_default();

                let focused_pane_title = panes_for_tab
                    .iter()
                    .find(|pane| pane.is_focused)
                    .map(|pane| pane.title.clone())
                    .unwrap_or_default();

                content = content.replace("{focused_pane_title}", &focused_pane_title);
            }

            content = self.replace_indicators(content, tab, panes);

            output = format!("{}{}", output, f.format_string(&content));
        }

        output.to_owned()
    }

    fn replace_indicators(&self, content: String, tab: &TabInfo, panes: &PaneManifest) -> String {
        let mut content = content;
        if content.contains("{fullscreen_indicator}")
            && let Some(fullscreen_indicator) = self.fullscreen_indicator.clone()
        {
            content = content.replace(
                "{fullscreen_indicator}",
                if tab.is_fullscreen_active {
                    fullscreen_indicator.as_ref()
                } else {
                    ""
                },
            );
        }

        if content.contains("{sync_indicator}")
            && let Some(sync_indicator) = self.sync_indicator.clone()
        {
            content = content.replace(
                "{sync_indicator}",
                if tab.is_sync_panes_active {
                    sync_indicator.as_ref()
                } else {
                    ""
                },
            );
        }

        if content.contains("{floating_indicator}")
            && let Some(floating_indicator) = self.floating_indicator.clone()
        {
            let panes_for_tab: Vec<PaneInfo> =
                panes.panes.get(&tab.position).cloned().unwrap_or_default();

            let is_floating = panes_for_tab.iter().any(|p| p.is_floating);

            content = content.replace(
                "{floating_indicator}",
                if is_floating {
                    floating_indicator.as_ref()
                } else {
                    ""
                },
            );
        }

        if content.contains("{bell_indicator}")
            && (self.bell_indicator.is_some() || self.flashing_bell_indicator.is_some())
        {
            let indicator = if tab.is_flashing_bell {
                self.flashing_bell_indicator.as_deref().unwrap_or("")
            } else if tab.has_bell_notification {
                self.bell_indicator.as_deref().unwrap_or("")
            } else {
                ""
            };

            content = content.replace("{bell_indicator}", indicator);
        }

        content
    }
}

/// Tells whether the name is the one Zellij gives to tabs the user did not name.
pub fn is_default_tab_name(name: &str) -> bool {
    name.strip_prefix("Tab #")
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
}

/// Returns the tab that holds the plugin pane with the given id.
pub fn tab_of_plugin<'a>(
    plugin_id: u32,
    tabs: &'a [TabInfo],
    panes: &PaneManifest,
) -> Option<&'a TabInfo> {
    tabs.iter().find(|tab| {
        panes
            .panes
            .get(&tab.position)
            .is_some_and(|panes| panes.iter().any(|p| p.is_plugin && p.id == plugin_id))
    })
}

/// Returns the terminal pane that has the focus within the given tab.
pub fn focused_terminal_pane<'a>(tab: &TabInfo, panes: &'a PaneManifest) -> Option<&'a PaneInfo> {
    let focused =
        |floating: bool| {
            panes.panes.get(&tab.position)?.iter().find(|p| {
                p.is_focused && p.is_floating == floating && !p.is_plugin && !p.is_suppressed
            })
        };

    // is_focused is tracked per layer, so the floating layer wins while it is shown
    if tab.are_floating_panes_visible
        && let Some(pane) = focused(true)
    {
        return Some(pane);
    }

    focused(false)
}

/// Reduces a command line to the name of its executable, like tmux does it for window names.
pub fn command_name(command_line: &[String]) -> Option<String> {
    let executable = command_line.first()?;
    let name = executable.rsplit('/').next()?.trim_start_matches('-');

    match name.is_empty() {
        true => None,
        false => Some(name.to_owned()),
    }
}

pub fn get_tab_window(
    tabs: &Vec<TabInfo>,
    max_count: Option<usize>,
) -> (usize, usize, Vec<TabInfo>) {
    let max_count = match max_count {
        Some(count) => count,
        None => return (0, 0, tabs.to_vec()),
    };

    if tabs.len() <= max_count {
        return (0, 0, tabs.to_vec());
    }

    let active_index = tabs.iter().position(|t| t.active).expect("no active tab");

    // active tab is in the last #max_count tabs, so return the last #max_count
    if active_index > tabs.len().saturating_sub(max_count) {
        return (
            tabs.len().saturating_sub(max_count),
            0,
            tabs.iter()
                .cloned()
                .rev()
                .take(max_count)
                .rev()
                .collect::<Vec<TabInfo>>(),
        );
    }

    // tabs must be truncated
    let first_index = active_index.saturating_sub(1);
    let last_index = cmp::min(first_index + max_count, tabs.len());

    (
        first_index,
        tabs.len().saturating_sub(last_index),
        tabs.as_slice()[first_index..last_index].to_vec(),
    )
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, HashMap};

    use zellij_tile::prelude::{PaneInfo, PaneManifest, TabInfo};

    use super::{
        TabsWidget, command_name, focused_terminal_pane, get_tab_window, is_default_tab_name,
        tab_of_plugin,
    };
    use crate::{config::ZellijState, widgets::widget::Widget};
    use rstest::rstest;

    fn tabs_state(count: usize) -> ZellijState {
        ZellijState {
            tabs: (0..count)
                .map(|i| TabInfo {
                    position: i,
                    name: format!("t{i}"),
                    active: i == 0,
                    ..TabInfo::default()
                })
                .collect(),
            ..ZellijState::default()
        }
    }

    #[rstest]
    // no button configured
    #[case(&[], 2, 8..8)]
    // button directly follows the last tab
    #[case(&[("tab_new_button", "#[fg=green,bold] + ")], 2, 8..11)]
    // multiple format sections count as one button
    #[case(&[("tab_new_button", "#[fg=black]<#[fg=green] + #[fg=black]>")], 2, 8..13)]
    // button follows the truncation indicator
    #[case(
        &[
            ("tab_new_button", "#[fg=green,bold] + "),
            ("tab_display_count", "2"),
            ("tab_truncate_end_format", "#[fg=red] +{count} >"),
        ],
        5,
        13..16
    )]
    // button is rendered without any tabs
    #[case(&[("tab_new_button", "#[fg=green,bold] + ")], 0, 0..3)]
    pub fn test_new_tab_button(
        #[case] extra_config: &[(&str, &str)],
        #[case] tab_count: usize,
        #[case] expected_range: std::ops::Range<usize>,
    ) {
        let mut config =
            BTreeMap::from([("tab_normal".to_owned(), "#[fg=blue] {name} ".to_owned())]);
        for (key, value) in extra_config {
            config.insert((*key).to_owned(), (*value).to_owned());
        }

        let widget = TabsWidget::new(&config);
        let state = tabs_state(tab_count);

        assert_eq!(widget.new_tab_button_range(&state), expected_range);
        assert_eq!(
            console::measure_text_width(&widget.process("tabs", &state)),
            expected_range.end
        );
    }

    #[rstest]
    #[case(&["/usr/bin/zsh"], Some("zsh"))]
    #[case(&["-zsh"], Some("zsh"))]
    #[case(&["nvim", "src/main.rs"], Some("nvim"))]
    #[case(&["./target/debug/zjstatus", "--help"], Some("zjstatus"))]
    #[case(&[], None)]
    #[case(&[""], None)]
    #[case(&["/usr/bin/"], None)]
    pub fn test_command_name(#[case] command_line: &[&str], #[case] expected: Option<&str>) {
        let command_line: Vec<String> = command_line.iter().map(|s| (*s).to_owned()).collect();

        assert_eq!(command_name(&command_line).as_deref(), expected);
    }

    #[rstest]
    #[case("Tab #1", true)]
    #[case("Tab #12", true)]
    #[case("Tab #", false)]
    #[case("Tab #1a", false)]
    #[case("zsh", false)]
    #[case("", false)]
    pub fn test_is_default_tab_name(#[case] name: &str, #[case] expected: bool) {
        assert_eq!(is_default_tab_name(name), expected);
    }

    fn pane(id: u32, is_plugin: bool, is_floating: bool, is_focused: bool) -> PaneInfo {
        PaneInfo {
            id,
            is_plugin,
            is_floating,
            is_focused,
            ..PaneInfo::default()
        }
    }

    // two tabs, each with its own bar (plugin 0 and 1), terminal 0 shares the id with a plugin
    fn two_tabs(are_floating_panes_visible: bool) -> (Vec<TabInfo>, PaneManifest) {
        let tabs = (0..2)
            .map(|position| TabInfo {
                position,
                are_floating_panes_visible,
                ..TabInfo::default()
            })
            .collect();

        let panes = PaneManifest {
            panes: HashMap::from([
                (
                    0,
                    vec![pane(0, true, false, false), pane(0, false, false, true)],
                ),
                (
                    1,
                    vec![
                        pane(1, true, false, false),
                        pane(1, false, false, false),
                        pane(2, false, false, true),
                        pane(3, false, true, true),
                    ],
                ),
            ]),
        };

        (tabs, panes)
    }

    #[rstest]
    #[case(0, Some(0))]
    #[case(1, Some(1))]
    #[case(2, None)]
    pub fn test_tab_of_plugin(#[case] plugin_id: u32, #[case] expected_position: Option<usize>) {
        let (tabs, panes) = two_tabs(false);

        let tab = tab_of_plugin(plugin_id, &tabs, &panes);

        assert_eq!(tab.map(|t| t.position), expected_position);
    }

    #[rstest]
    #[case(0, false, Some(0))]
    // the focused tiled pane, as long as the floating panes are hidden
    #[case(1, false, Some(2))]
    #[case(1, true, Some(3))]
    pub fn test_focused_terminal_pane(
        #[case] position: usize,
        #[case] are_floating_panes_visible: bool,
        #[case] expected_id: Option<u32>,
    ) {
        let (tabs, panes) = two_tabs(are_floating_panes_visible);

        let pane = focused_terminal_pane(&tabs[position], &panes);

        assert_eq!(pane.map(|p| p.id), expected_id);
        assert!(pane.is_none_or(|p| !p.is_plugin));
    }

    #[rstest]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (1, 1, vec![
                TabInfo {
                    active: false,
                    name: "2".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: true,
                    name: "3".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "4".to_owned(),
                    ..TabInfo::default()
                },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: true,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (0, 2, vec![
                TabInfo {
                    active: true,
                    name: "1".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "2".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "3".to_owned(),
                    ..TabInfo::default()
                },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (0, 2, vec![
                TabInfo {
                    active: false,
                    name: "1".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: true,
                    name: "2".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "3".to_owned(),
                    ..TabInfo::default()
                },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (2, 0, vec![
                TabInfo {
                    active: false,
                    name: "3".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "4".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: true,
                    name: "5".to_owned(),
                    ..TabInfo::default()
                },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (2, 0, vec![
                TabInfo {
                    active: false,
                    name: "3".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: true,
                    name: "4".to_owned(),
                    ..TabInfo::default()
                },
                TabInfo {
                    active: false,
                    name: "5".to_owned(),
                    ..TabInfo::default()
                },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
        ],
        None,
        (0, 0, vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "4".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "5".to_owned(),
                ..TabInfo::default()
            },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (0, 0, vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            ]
        )
    )]
    #[case(
        vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
        ],
        Some(3),
        (0, 0, vec![
            TabInfo {
                active: false,
                name: "1".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: true,
                name: "2".to_owned(),
                ..TabInfo::default()
            },
            TabInfo {
                active: false,
                name: "3".to_owned(),
                ..TabInfo::default()
            },
            ]
        )
    )]
    pub fn test_get_tab_window(
        #[case] tabs: Vec<TabInfo>,
        #[case] max_count: Option<usize>,
        #[case] expected: (usize, usize, Vec<TabInfo>),
    ) {
        let res = get_tab_window(&tabs, max_count);

        assert_eq!(res, expected);
    }
}
