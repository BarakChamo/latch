# Latch for GitHub Copilot CLI

Install Latch from the GitHub marketplace:

```sh
copilot plugin marketplace add 'https://github.com/BarakChamo/latch.git#codex/marketplace'
copilot plugin install latch@latch-tools
```

Start a new session in your project. The plugin carries the Latch binary
for macOS and glibc Linux on ARM64 and x64.

Use a Copilot CLI version that supports `userPromptTransformed`. The
[hook configuration](hooks.json) can also be installed under
`.github/hooks/`. Use one installation method per project.

| Copilot event | Latch behavior |
| --- | --- |
| `userPromptSubmitted` | Snapshot the Git worktree. |
| `agentStop` | Run matching turn-end checks. Return diagnostics as a Copilot warning without requesting continuation. |
| `userPromptTransformed` | Append any saved failure to the next model-facing prompt in the same session and directory, once. |
| `preToolUse` | Capture the Git baseline when any edit check uses `workspace_changed` or a path filter. |
| `postToolUse` | Run opt-in `after_edit` checks after a successful native edit and append diagnostics to the tool result. |
| `sessionEnd` | Discard the baseline. A pending failure can still be delivered when the session is resumed. |

Copilot uses exit code 2 and stderr for a nonblocking stop-hook warning,
including success output if configured. The native edits matched are
`create`, `edit`, `str_replace_editor`, and `apply_patch`.

Copilot may omit a tool-call ID from command-hook payloads. Latch pairs
those calls by tool name and arguments and runs the check if pairing is
ambiguous.

The bundled `run_turn_end` tool takes the absolute `project_directory` from
the agent and runs all configured turn-end checks there. Copilot starts the
tool in the plugin directory. The
[configuration reference](https://github.com/BarakChamo/latch#configure-checks) covers shared
defaults, per-hook overrides, and output policies.
