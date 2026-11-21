# Pi

Use Pi 1.5.2 or later with its built-in MCP support enabled.

After installing the released npm tarballs, register the extension:

```sh
pi install "$(npm root -g)/@barakchamo/latch/plugins/pi"
```

If you use the standalone archive, pass the absolute path to its
`plugins/pi` directory. Use the same path with `pi remove` to uninstall.
Restart Pi from a terminal where `latch --version` works.

Latch adds diagnostics to results from Pi's `edit` and `write` tools.
After the agent settles, Latch adds the turn-end report to session
history without starting another model call. It also registers the tools
`mcp__latch__run_turn_end` and `mcp__latch__search_output`.
