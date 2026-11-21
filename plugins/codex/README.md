# Latch for Codex

Install from the project’s GitHub marketplace:

```sh
codex plugin marketplace add BarakChamo/latch --ref codex/marketplace
codex plugin add latch@latch-tools
```

The plugin includes Latch for macOS and glibc Linux on ARM64 and x64.

Start Codex from the directory containing your project's
[Latch configuration or package scripts](https://github.com/BarakChamo/latch#configure-checks).
Review and trust the plugin hooks in Codex using `/hooks`. The
[hook configuration](hooks/hooks.json) can also be installed directly as a
project's `.codex/hooks.json`. Merge the event entries if that file already
exists, and use one installation method per project.

After updating Latch, refresh the installed plugin and review the new hook
definitions in `/hooks`.

| Codex event | Latch behavior |
| --- | --- |
| `UserPromptSubmit` | Snapshot the Git worktree by session and turn. Deliver a saved failure report once. |
| `PreToolUse` | Capture the Git baseline when any edit check uses `workspace_changed` or a path filter. |
| `PostToolUse` | When `after_edit` is configured, run its checks after `apply_patch` and attach the output as model context. |
| `Stop` | Compare against the same commit. Skip unchanged worktrees and run checks on changes or when a comparison is unavailable. |
| `Interrupt` | Discard that turn's baseline. |

Snapshots cover the entire Git worktree. Checks run from the Codex event's
working directory. Snapshots include tracked
paths reported by Git and untracked, nonignored files. They compare net
content changes, including preexisting dirty files that change again.
Projects without Git or without a commit still run their checks at stop.
Snapshots are stored under
`$CODEX_HOME/latch/baselines-v1` (default `~/.codex` for `CODEX_HOME`) and
normally removed on stop or interrupt.

Latch also runs validation if the Git index can hide edits, such as when
paths are marked assume-unchanged or skip-worktree.

Successful checks are quiet by default. On failure, Latch reports the diagnostics
and lets the turn finish. The next prompt in the same session and working
directory receives the saved failure once. The stop and post-tool hooks
allow up to 120 seconds for validation.

The bundled `run_turn_end` tool runs the same checks on demand. It takes the project’s absolute
path and returns failure diagnostics as a tool result visible to the agent.
Codex may ask for approval because the configured commands can modify files.
See the [configuration reference](https://github.com/BarakChamo/latch#configure-checks) for defaults,
per-hook overrides, and output policies.
