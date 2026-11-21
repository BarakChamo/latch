# Latch

## Install the plugin

On macOS or Linux, open a terminal in this folder and run the command for
your coding agent.

| Agent | Command |
| --- | --- |
| Codex | `./install codex` |
| Claude Code | `./install claude` |
| GitHub Copilot CLI | `./install copilot` |
| OpenCode | `./install opencode` |
| Pi | `./install pi` |
| OpenHands CLI | From your project, run `/path/to/bundle/install openhands`. |
| Kimi Code CLI | Run `./install kimi`, then paste the printed command into Kimi Code. |

From a Grok Build project, run `/path/to/bundle/install grok` and trust
the folder in Grok with `/hooks-trust`. The bundle also includes preview
adapters for Cursor, Devin, and Kiro CLI V3. From the project, run
`/path/to/bundle/install <cursor|devin|kiro> --preview`.
These adapters have protocol tests and are still undergoing real-session validation.

Start a new session in your project. In Codex, review and trust the Latch
hooks using `/hooks`. Keep this folder so the harness can find the local
marketplace. The plugin bundle includes Latch.

On Windows, add this archive's `bin` directory to `PATH` and add the extracted
folder as a local marketplace. See the [harness setup guides](plugins/).

## Run Latch in a terminal

Use the binary in this folder's `bin` directory or add it to your `PATH`.

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Use `latch explain after_edit` or `latch explain turn_end` in your project
to inspect the resolved checks.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
