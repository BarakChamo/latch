# Latch

Install the CLI with `npm install -g @barakchamo/latch`, then run
`latch --version`. npm installs the native binary for your platform as an
optional dependency. Run `latch explain after_edit` or `latch explain turn_end`
in a project to inspect its checks.

The package also contains the Codex, Claude Code, and Copilot CLI plugins
and their marketplace catalogs. The marketplace root is
`$(npm root -g)/@barakchamo/latch` on shell-based installations. Add that
directory as a local marketplace in your harness. The plugins use the same
`latch` executable on `PATH` for hooks and the local `run_turn_end` tool.

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.
