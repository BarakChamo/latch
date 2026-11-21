# Latch

Latch automatically runs lint, tests, and other project checks and sends failures back to your coding agent.

[Install a plugin](#install-a-plugin) · [First check](#run-your-first-check) · [Configuration](#configure-checks) · [Standalone CLI](#install-the-standalone-cli) · [Troubleshooting](#troubleshoot) · [Releases](https://github.com/BarakChamo/latch/releases/latest)

## Install a plugin

The plugins bundle Latch for macOS and glibc Linux on ARM64 and x64.
Install your agent’s CLI and sign in if prompted.

### Codex

```sh
codex plugin marketplace add BarakChamo/latch --ref codex/marketplace
codex plugin add latch@latch-tools
```

Run `codex login` if needed. Start a new session in your project, open
`/hooks`, and approve the Latch hooks after reviewing them.

### Claude Code

```sh
claude plugin marketplace add 'BarakChamo/latch#codex/marketplace'
claude plugin install latch@latch-tools
```

Run `claude auth login` if needed, then start a new session in your project.

### GitHub Copilot CLI

```sh
copilot plugin marketplace add 'https://github.com/BarakChamo/latch.git#codex/marketplace'
copilot plugin install latch@latch-tools
```

Run `copilot login` if needed, then start a new session in your project.
This setup was tested with Copilot CLI 1.0.114.

If you previously installed Latch from a local archive or npm package,
remove that plugin in your harness before enabling the marketplace version.

## Run your first check

Add `latch.json` to your project root. This example runs your existing
`npm run lint` script after each source edit. Use whatever check you already
run locally, such as `uv run pytest`.

```json
{
  "hooks": {
    "after_edit": {
      "commands": [["npm", "run", "lint"]]
    }
  }
}
```

Start your agent in the project and ask it to change a file with its edit
tool. Latch sends any lint errors back to the agent. Passing checks are
quiet by default.

## Configure checks

- Use `after_edit` for quick checks that should run after a native file edit.
  Use `turn_end` for slower checks or to catch changes made through shell commands.
- Each event accepts one object or an array of objects. Matching objects
  run in order. `paths` filters by Git-worktree-relative globs and
  `extensions` filters by filename suffix. When both are present, the
  same changed file must match both.
- By default, Latch skips unchanged work, including no-op edits to dirty
  files. Set `when` to `"always"` to run on every matching event.
- By default, Latch runs the full command list and reports every failure.
  Use `"on_failure": "stop"` to stop a list at its first failure, or
  `"on_success": "output"` to show passing output.
- Put shared settings in `defaults` and override them inside any hook object.
  Copy [the JSON schema](latch.schema.json) next to `latch.json` and add
  `"$schema": "./latch.schema.json"` for editor completion.

For example, run Markdown lint only when a Markdown file changes and run
the full test suite after a changed turn:

```json
{
  "defaults": { "failure_output": "tail", "tail_lines": 12 },
  "hooks": {
    "after_edit": {
      "extensions": ["md", "mdx"],
      "commands": [["npm", "run", "lint:md"]]
    },
    "turn_end": {
      "commands": [["npm", "test"]]
    }
  }
}
```

You can also use scripts named `hook:afteredit` or `hook:turnend` in
`package.json`, or `hook-afteredit` and `hook-turnend` under
`[project.scripts]` in `pyproject.toml`. Latch uses the configured
JavaScript package manager or uv and runs discovered scripts after explicit
commands. See the runnable [TypeScript and Python examples](examples/).

## Read failures and saved logs

Codex, Claude Code, and Copilot CLI deliver edit failures during the turn.
A turn-end failure lets the turn finish and is available on the next prompt in
the same session.

Set `failure_output` to `excerpt` for the default preview, `compact` to
keep diagnostic lines, `tail` for the last lines of each stream, or
`rtk` to use a separately installed [RTK](https://github.com/rtk-ai/rtk) filter. When the report includes
a `Saved log` path, ask your agent to open it or call Latch’s
`search_output` tool. You can also ask it to call `run_turn_end`
to run the current project’s turn-end checks on demand.

## Install the standalone CLI

Install the CLI when you want to run checks from your terminal or CI.
Download your platform’s `latch-0.1.1-*.tar.gz` from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/tag/v0.1.1).
From your downloads directory, extract it and install the binary. For
Apple Silicon:

```sh
tar -xzf latch-0.1.1-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 latch-0.1.1-aarch64-apple-darwin/bin/latch "$HOME/.local/bin/latch"
export PATH="$HOME/.local/bin:$PATH"
latch --version
```

Use `x86_64-apple-darwin` for Intel Macs,
`x86_64-unknown-linux-gnu` for Linux x64, or
`aarch64-unknown-linux-gnu` for Linux ARM64. Add the `export PATH` line to
your shell startup file to keep `latch` available in new terminals.

From your project root, run the event you configured in `latch.json`.
For the [example above](#run-your-first-check):

```sh
latch run after_edit
```

This runs the check immediately. It exits `0` on success or prints the
failure and exits `1`. Run `latch explain after_edit` to see which commands
and settings Latch found.

To search a saved failure, copy the full path after `Saved log:` into
`latch output "/path/from-the-report.log" --grep "text to find"`.
Short output may fit in the report without a saved log.

If you prefer npm, download the wrapper tarball and the binary tarball
for your platform from the same release. For example, on Apple Silicon:

```sh
npm install --global --offline ./barakchamo-latch-0.1.1.tgz ./barakchamo-latch-darwin-arm64-0.1.1.tgz
```

For Intel Mac, Linux ARM64, or Linux x64, use the binary tarball with
`darwin-x64`, `linux-arm64`, or `linux-x64` in its name.

## Other coding agents

### OpenCode

Download and extract the [standalone archive](#install-the-standalone-cli).
In a terminal open to the extracted folder, run its installer:

```sh
./install opencode
```

Keep that folder in place. Restart OpenCode, connect a model provider if
needed, and run the [first check](#run-your-first-check). The
[OpenCode guide](https://github.com/BarakChamo/latch/tree/main/plugins/opencode)
also shows project-specific configuration for v1 and v2.

Latch also supports Pi, OpenHands CLI, Kimi Code, and Grok Build.
Cursor, Devin, and Kiro CLI V3 are in preview. See the [integration guides on GitHub](https://github.com/BarakChamo/latch/tree/main/plugins)
for those agents.

## Troubleshoot

- **The agent asks you to sign in.** Use the sign-in command in
  [your agent’s setup](#install-a-plugin), then restart the session.
- **The hook is quiet.** Start a new session from the directory containing
  `latch.json`. Review `/hooks` in Codex, or run `claude plugin list`
  or `copilot plugin list` to check that Latch is enabled. By default,
  unchanged files and passing checks are quiet.
- **Copilot can’t find the Latch tools.** Run `copilot mcp list` and look
  for `latch`. Update Copilot if the commands in this guide are unavailable.
- **Using managed Codex Cloud?** The plugin runs local command hooks.
  In a hosted checkout you can install the standalone CLI and run
  `latch run turn_end` from its terminal or CI.

## Develop Latch

Install Rust and Node.js, run `npm ci`, then run `npm run check`.
See the [release guide](https://github.com/BarakChamo/latch/blob/main/docs/releasing.md)
for packaging and verification.
