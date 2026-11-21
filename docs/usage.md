# Latch usage guide

Start with the [README](../README.md) to install a plugin and run your first check.

## Install the plugin from a terminal

Download [`install-latch-plugin.sh`](https://github.com/BarakChamo/latch/releases/download/v0.1.2/install-latch-plugin.sh), then run it in your terminal.

```sh
sh install-latch-plugin.sh --install-plugin codex
```

Use `claude` or `copilot` for the corresponding CLI. The installer registers
Latch through the harness’s marketplace commands. It downloads the matching
release binary, checks its SHA-256, and keeps it in your user cache. The
marketplace plugin uses the same cache. Once the binary is cached, its hooks
and tools can run offline. In Codex, open `/hooks` and approve the hooks
after installation.

## Hooks and feedback

The plugin translates its agent’s native lifecycle events into Latch events. Latch uses the same project commands and settings regardless of which plugin calls it.

| Event | Comparison | Use |
| --- | --- | --- |
| `after_edit` | Before and after a supported native file edit. | Fast lint, formatting, or type checks. |
| `turn_end` | At the start and end of a turn. Includes changes made with shell commands. | Test suites and checks that only need to run once per changed turn. |

Latch compares Git-visible paths and contents. An existing uncommitted change only triggers a check if it changes during that event. Linked Git worktrees have their own baselines. If Latch can’t reliably compare the workspace, it runs configured checks without applying change-based filters.

| Agent | Edit feedback | Turn-end feedback |
| --- | --- | --- |
| Codex | Same-turn edit result. | App-server warning. Failed checks reach the next prompt in the same session. |
| Claude Code | Same-turn edit result. | Informational hook event. Failed checks reach the next prompt in the same session. |
| GitHub Copilot CLI | Same-turn edit result. | Hook warning. Failed checks reach the next prompt in the same session. |
| OpenCode | Same-turn edit result. | v1 toast or headless stderr, v2 server stderr. Failed checks also reach the next prompt. |
| Pi | Same-turn edit result. | Session history, visible to the next model request. |
| OpenHands CLI | Next prompt. | Next prompt. |
| Kimi Code CLI | Use `turn_end`. | Next prompt. |
| Grok Build | Same-turn edit result. | Hook stderr. |

A failed check does not start another turn or a repair pass. The terminal output of a headless run may omit hook context that the agent receives. See the [integration guides](../plugins/) for agent-specific hooks and setup.

## Package script discovery

Latch reads package scripts from the session or command working directory.

| Event | JavaScript and TypeScript in `package.json` | Python in `pyproject.toml` |
| --- | --- | --- |
| `after_edit` | `scripts["hook:afteredit"]` | `[project.scripts]` entry named `hook-afteredit` |
| `turn_end` | `scripts["hook:turnend"]` | `[project.scripts]` entry named `hook-turnend` |

For JavaScript, Latch chooses npm, pnpm, Yarn, or Bun using `packageManager`, then checks the local lockfile, then defaults to npm. For Python, define the names as console entry points in a packaged uv project. Latch calls them with `uv run`. See the runnable [TypeScript](../examples/typescript/) and [Python](../examples/python/) projects. If you already run a command such as `uv run pytest`, you can put that command directly in `latch.json`.

## Configuration

Run `latch config init` from the project directory to create a starter
`latch.json`. It leaves script discovery enabled and preserves any existing
file.

Use [`latch.schema.json`](../latch.schema.json) for editor completion and validation. Copy it next to your config and set `"$schema": "./latch.schema.json"`.

```json
{
  "defaults": { "failure_output": "tail", "tail_lines": 12 },
  "hooks": {
    "after_edit": [
      { "commands": [["npm", "run", "lint"]] },
      { "extensions": ["md", "mdx"], "commands": [["npm", "run", "lint:md"]] }
    ],
    "turn_end": { "commands": [["npm", "test"]] }
  }
}
```

- An event accepts one object or an array. Matching objects run in order. Command arrays start with the executable and list its arguments separately.
- `paths` matches Git-worktree-relative globs. `extensions` matches filename suffixes. When both are set, one changed file must satisfy both filters for the object to run.
- Global `defaults` apply unless an object overrides them. Discovered scripts run after explicit objects, with JavaScript before Python, and inherit the global defaults.
- By default, `when` is `workspace_changed`, `on_success` is `silent`, and `on_failure` is `collect`. Use `always` to run unfiltered checks without changes, `output` to show passing output, or `stop` to stop the current command list on its first failure.
- Set an event to `[]` to disable it, including discovery. Set `"hooks": {}` to disable both events.

## Install the CLI

Download your platform’s archive from [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest). The packages cover Apple Silicon and Intel Macs, plus glibc Linux on ARM64 and x64.

### Standalone download

For Apple Silicon, from your downloads directory:

```sh
tar -xzf latch-0.1.2-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 latch-0.1.2-aarch64-apple-darwin/bin/latch "$HOME/.local/bin/latch"
export PATH="$HOME/.local/bin:$PATH"
latch --version
```

Use the archive name for your platform in the first and third lines. Add the `export PATH` line to your shell startup file to keep it in new terminals.

### npm

Download the wrapper and the matching platform tarball from the same release. For Apple Silicon:

```sh
npm install --global --offline ./barakchamo-latch-0.1.2.tgz ./barakchamo-latch-darwin-arm64-0.1.2.tgz
latch --version
```

The other binary tarballs contain `darwin-x64`, `linux-arm64`, or `linux-x64` in their names.

### Cargo

With Rust 1.88 or later:

```sh
git clone --branch v0.1.2 https://github.com/BarakChamo/latch.git
cd latch
cargo install --locked --path crates/latch
latch --version
```

## Commands and saved logs

| Command | What it does |
| --- | --- |
| `latch run after_edit` | Run edit checks immediately. |
| `latch run turn_end` | Run turn-end checks immediately. |
| `latch explain after_edit` or `latch explain turn_end` | Show the commands, settings, and filters Latch resolved. |
| `latch output <log-or-id> [--grep <text>]` | Replay or search a saved failure from the same project. |
| `latch config init` | Create a starter `latch.json` in the current directory. |
| `latch --help` | List the available commands. |

Manual runs bypass the Git change gate and file filters. Successful runs exit `0`, check or configuration failures exit `1`, and invalid CLI arguments exit `2`.

Hooks and plugin tools can shorten failure output. Set `failure_output` to `excerpt` (default), `compact` (diagnostic lines), `tail` (last lines of each stream), or `rtk` (uses an independently installed [RTK](https://github.com/rtk-ai/rtk)). If the report includes a `Saved log` path, open the file directly or use `latch output` from the same project.

Where the plugin exposes Latch tools, ask your agent to call `run_turn_end` to run checks now or `search_output` with the saved log path and a literal query to recover omitted lines.

## Troubleshooting

- **The agent asks you to sign in.** Use `codex login`, `claude auth login`, or `copilot login` for the corresponding CLI and start a new session.
- **Nothing ran.** Make sure the plugin is enabled, start a new session in your project, and make a real change with the agent’s file-edit tool. In Codex, review hook trust in `/hooks`. An unchanged edit or read-only turn usually skips validation.
- **The check is still quiet.** Passing checks are silent by default. Use `latch explain <event>` with the standalone CLI to confirm the script or command is configured.
- **The command could not start.** Install the project dependencies and make sure the runner is on the agent’s `PATH`. For Copilot tool discovery, run `copilot mcp list` and look for `latch`.
- **Running in a hosted session.** Automatic checks depend on the host loading the plugin or command hooks. You can run the CLI directly from the checkout or in CI.
