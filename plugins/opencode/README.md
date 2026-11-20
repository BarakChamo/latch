# OpenCode

Latch supports OpenCode v1 and v2 from one plugin entry point. It was tested
with 1.18.31, 1.19.6, and 2.3.2. The v2 integration uses the idle hook from
OpenCode 2.3.2.

Run `./install opencode` from the macOS or Linux Latch bundle. The installer
links this plugin into `${XDG_CONFIG_HOME:-$HOME/.config}/opencode/plugins`.
Keep the bundle where you extracted it and restart OpenCode. To uninstall,
remove the `latch.js` link from that directory.

For a source or npm install, put `latch` on OpenCode's `PATH` and add the
absolute path to `plugins/opencode/index.js` to the `plugin` list in v1.
In v2, add the absolute path to the `plugins/opencode` directory to the
`plugins` list.

Latch adds diagnostics to results from `edit`, `write`, and the native
patch tool. A failed turn-end check is included in the next prompt in the
same session. In v1, the report also appears as a TUI toast or on headless
stderr. In v2, the report goes to the server's stderr. To see it immediately
in a headless run, use `opencode --standalone --print-logs run "..."`.
Latch does not start another turn.
The plugin registers the local Latch MCP server with `run_turn_end` and
`search_output`.
