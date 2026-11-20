# Pi

Use Pi 1.5.2 or later with its built-in MCP support enabled.

Run `./install pi` from the macOS or Linux Latch bundle. It registers the
extension with `pi install`. Keep the extracted bundle and restart Pi.
To uninstall, run `pi remove /absolute/path/to/bundle/plugins/pi` with
the same path you installed.

For a source or npm install, put `latch` on Pi's `PATH` and run
`pi install /absolute/path/to/plugins/pi`.

Latch adds diagnostics to results from Pi's `edit` and `write` tools.
After the agent settles, Latch adds the turn-end report to session
history without starting another model call. It also registers the tools
`mcp__latch__run_turn_end` and `mcp__latch__search_output`.
