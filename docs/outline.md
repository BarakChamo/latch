# Latch documentation outline

| Page | What it should answer |
| --- | --- |
| Introduction | What Latch is, which harnesses it supports, and when checks run. |
| Quickstart | How to install the CLI, enable the Codex, Claude Code, or Copilot plugin, and confirm the first check ran. |
| Package scripts | Which JavaScript and uv script names Latch discovers, how it chooses a runner, and where to find runnable examples. |
| Harness plugins | How to install and manage each plugin, call `run_turn_end`, and identify the commands a harness launches. |
| Configuration | How `latch.json` merges with discovery, how hook arrays and path filters work, and when to use `collect` or `stop`. |
| Hooks and feedback | Which edits trigger checks, how Git baselines handle dirty worktrees, and where each harness shows diagnostics. |
| CLI reference | How to run and inspect checks from a terminal and interpret exit codes. |
| Troubleshooting | How to diagnose missing runners, skipped hooks, missing tools, and missing reports. |
