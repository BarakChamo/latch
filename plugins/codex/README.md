# Latch for Codex

Install the CLI and make sure `latch` and `git` are on the Codex process's
`PATH`. From the Latch checkout, run:

```sh
cargo install --locked --path crates/latch
codex plugin marketplace add "$(pwd)"
codex plugin add latch@latch-local
```

The package uses `.codex-plugin/plugin.json` so Codex CLI 0.154.0 discovers
its bundled hooks. If you installed an earlier copy with the root
`plugin.json` manifest, remove and reinstall `latch@latch-local`.

Start Codex from the directory containing your project's
[Latch configuration or package scripts](../../README.md#configuration).
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

The bundled `run_turn_end` tool runs the same checks on demand. It takes no
arguments and returns failure diagnostics as a tool result visible to the agent.
Codex may ask for approval because the configured commands can modify files.
See the [configuration reference](../../README.md#configuration) for defaults,
per-hook overrides, and output policies.
