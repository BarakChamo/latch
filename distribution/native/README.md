# Latch

This archive contains a native CLI and plugins for Codex, Claude Code, and
GitHub Copilot CLI. Add this archive's `bin` directory to your `PATH` and
run `latch --version`. Add the extracted directory as a local marketplace
to install a plugin. See the [harness setup guides](plugins/).

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Use `latch explain after_edit` or `latch explain turn_end` in your project
to inspect the resolved checks.
