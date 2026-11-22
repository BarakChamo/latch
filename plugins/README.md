# Harness plugins

Install from GitHub through your agent’s plugin manager. The plugin
downloads and verifies the Latch CLI on first use. For offline setup,
the macOS and Linux archives include the CLI and plugins.
[Installation instructions](../README.md#install-a-plugin)
The CLI owns project configuration,
command execution, and workspace snapshots. Each plugin wires its harness
events and translates validation results into the host's hook protocol.

The guides below cover the source plugins and their hook mappings.

- [Codex](codex/README.md)
- [Claude Code](claude-code/README.md)
- [GitHub Copilot CLI](copilot/README.md)
- [OpenCode](opencode/README.md)
- [Pi](pi/README.md)
- [OpenHands CLI](openhands/README.md)
- [Kimi Code CLI](kimi/README.md)

## Local hook adapters

Run the command from the project with [the CLI installed](../docs/usage.md#install-the-cli).

| Agent | Setup |
| --- | --- |
| Grok Build | `latch install grok`, then review and trust the hooks with `/hooks-trust`. |
| Cursor CLI (preview) | `latch install cursor --preview` |
| Devin CLI (preview) | `latch install devin --preview` |
| Kiro CLI V3 (preview) | `latch install kiro --preview` |

The preview adapters have automated protocol tests. See [hooks and feedback](../docs/usage.md#hooks-and-feedback) for the common lifecycle and where reports appear.
