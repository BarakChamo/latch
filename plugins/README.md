# Harness plugins

Harness plugins invoke the local Latch CLI. The CLI owns project configuration,
command execution, and workspace snapshots. Each plugin wires its harness
events and translates validation results into the host's hook protocol.

- [Codex](codex/README.md)
- [Claude Code](claude-code/README.md)
- [GitHub Copilot CLI](copilot/README.md)
