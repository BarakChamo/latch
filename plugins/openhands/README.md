# OpenHands

Use `PluginSource(source="github:BarakChamo/latch")` in your OpenHands SDK
conversation to load the native plugin with its hooks and tools.
[SDK and Canvas setup](https://barakchamo.github.io/latch/docs/plugins/#openhands)

## OpenHands CLI

From the Git project you want to validate, run
`sh /path/to/install-latch-plugin.sh install openhands` with the downloaded
[release installer](https://github.com/BarakChamo/latch/releases/latest/download/install-latch-plugin.sh).
If you already have the CLI on your path, run `latch install openhands`.

This adds workspace hooks to `.openhands/hooks.json` and registers Latch in
the OpenHands user MCP configuration. It preserves other hook and server
entries. If OpenHands already uses a user-level hook file and your project has
no hook file, installation updates the user-level file. The generated command paths point to your local Latch installation.
Keep these paths local or replace them with a shared executable path before
committing the configuration.

Latch runs `after_edit` after changes from the native file editor and
`turn_end` on changed turns. Reports appear on the next user prompt in the
same conversation. Failed checks leave the turn finished. The MCP server
exposes `run_turn_end` and `search_output`, both with an absolute project
directory.

Start a new conversation in the project after installation. To uninstall,
remove the Latch entries from the workspace hook file and remove the
`latch` entry from the `mcpServers` section of `~/.openhands/mcp.json`
(or the directory selected by `OPENHANDS_PERSISTENCE_DIR`).

[OpenHands hook configuration](https://docs.openhands.dev/openhands/usage/customization/hooks)
