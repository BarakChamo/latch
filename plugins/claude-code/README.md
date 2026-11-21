# Latch for Claude Code

Install Latch from the GitHub marketplace:

```sh
claude plugin marketplace add 'BarakChamo/latch#codex/marketplace'
claude plugin install latch@latch-tools
```

Start a new session in your project. The plugin downloads Latch on first
use for macOS and glibc Linux on ARM64 and x64.

The [hook configuration](hooks/hooks.json) can also be merged into the
project's `.claude/settings.json`. Use one installation method per project.

| Claude Code event | Latch behavior |
| --- | --- |
| `UserPromptSubmit` | Capture a Git baseline keyed by session ID. Deliver a saved failure report once. |
| `PreToolUse` | Capture the Git baseline when any edit check uses `workspace_changed` or a path filter. |
| `PostToolUse` | When `after_edit` is configured, run its checks after the matched native edit tools and attach the output as model context. |
| `Stop` | By default, compare the worktree against the starting commit. Run the commands for changed worktrees or when a comparison is unavailable. |
| `StopFailure` / `SessionEnd` | Discard any pending baseline. |

Latch compares Git-visible files across the worktree, including untracked,
nonignored files. It resolves [checks](https://github.com/BarakChamo/latch#configure-checks) from the
hook event's working directory. Baselines live in
`$CLAUDE_CONFIG_DIR/latch/baselines-v1`, with `~/.claude` as the default
config directory.

Latch also runs validation if the Git index can hide edits, such as when
paths are marked assume-unchanged or skip-worktree.

Successful checks are quiet by default. On failure, Latch reports the diagnostics
and lets the turn finish. The next prompt in the same session and working
directory receives the saved failure once. The stop and post-tool hooks
allow up to 120 seconds for validation.

The bundled `run_turn_end` tool runs the same checks on demand. It takes no
arguments and returns failure diagnostics as a tool result visible to the agent.
See the [configuration reference](https://github.com/BarakChamo/latch#configure-checks) for defaults,
per-hook overrides, and output policies.
