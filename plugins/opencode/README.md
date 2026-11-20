# OpenCode

Use OpenCode 1.x with the v1 plugin API. Latch was tested with 1.18.31
and 1.19.6.

Run `./install opencode` from the macOS or Linux Latch bundle. The installer
links this plugin into `${XDG_CONFIG_HOME:-$HOME/.config}/opencode/plugins`.
Keep the bundle where you extracted it and restart OpenCode. To uninstall,
remove the `latch.js` link from that directory.

For a source or npm install, put `latch` on OpenCode's `PATH` and add the
absolute path to `plugins/opencode/index.js` to the `plugin` list in your
OpenCode v1 config.

Latch adds edit diagnostics to results from `edit`, `write`, and
`apply_patch`. At turn end, OpenCode shows a toast in the TUI and emits
the report on stderr for headless runs. A failed check is included in the
next prompt in the same session. Latch does not start another turn.
The plugin registers the local Latch MCP server with `run_turn_end` and
`search_output`.
