# Latch

For the private preview, install this wrapper tarball and the binary
tarball for your platform together. See the
[install guide](https://github.com/BarakChamo/latch/blob/preview-0.1.0/apps/docs/content/docs/cli.mdx#install-the-cli)
for the command. Run `latch --version` to verify the install, or
`latch explain turn_end` in a project to inspect its checks.

The package also contains the Codex, Claude Code, Copilot CLI, OpenCode,
and Pi plugins. The marketplace root for Codex, Claude Code, and Copilot is
`$(npm root -g)/@barakchamo/latch` on shell-based installations. Add that
directory as a local marketplace in your harness. The plugins use the same
`latch` executable on `PATH` for hooks and the local `run_turn_end` tool.

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
