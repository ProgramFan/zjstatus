# status-bar

Zellij's built-in `status-bar` plugin, taken from
[zellij v0.45.1](https://github.com/zellij-org/zellij/tree/v0.45.1/default-plugins/status-bar)
(MIT, see `LICENSE.md`), with fixes for themes whose bar background is not the
terminal's own background. Everything else is unchanged; the changes are marked
with `zjstatus:` comments in the source.

- The ` ... ` marker shown when the key hints do not fit is painted with the
  bar colours. Upstream prints it unstyled, which leaves a block in the
  terminal's default colours.
- The separator after the active mode ribbon is drawn on the bar background
  (`text_unselected.background`). Upstream draws it on `ribbon_selected.base`,
  the text colour of the active ribbon, so themes with light text on the
  active ribbon get a light block.
- The plugin requests its permissions, as it is no longer built in.

## Usage

```sh
cargo build --release
cp target/wasm32-wasip1/release/status-bar.wasm ~/.config/zellij/plugins/
```

Point the `status-bar` alias in `config.kdl` at it, so every layout picks it up:

```kdl
plugins {
    status-bar location="file:~/.config/zellij/plugins/status-bar.wasm"
}
```
