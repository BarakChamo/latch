# Latch

Run your project’s checks automatically when a coding agent edits files or
finishes a turn. Passing checks stay quiet. Failures come back with the
diagnostic and, when the output is shortened, a link to the saved log
when available.

- [Install the Codex plugin](#install-the-codex-plugin)
- [Run your first check](#run-your-first-check)
- [Configure checks](#configure-checks)
- [Read failures and saved logs](#read-failures-and-saved-logs)
- [Install the standalone CLI](#install-the-standalone-cli)
- [Other coding agents](#other-coding-agents)
- [Troubleshoot](#troubleshoot)
- [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest)

## Install the Codex plugin

In a terminal with Codex installed, run:

```sh
codex plugin marketplace add BarakChamo/latch --ref codex/marketplace
codex plugin add latch@latch-tools
```

The plugin includes Latch for macOS and glibc Linux on ARM64 and x64.
You’ll need GitHub access to this repository. In the Codex desktop app,
you can also add the marketplace with source `BarakChamo/latch` and Git
ref `codex/marketplace`, then install **Latch** from that marketplace.

Open a new Codex session in your project. Run `/hooks` and approve the
Latch hooks after reviewing them. If you installed the older
`latch@latch-local` plugin, remove it with `codex plugin remove latch@latch-local`
to avoid running both copies.

## Run your first check

Add `latch.json` to a Git project that already has a `typecheck` script:

```json
{
  "hooks": {
    "after_edit": {
      "commands": [["npm", "run", "typecheck"]]
    }
  }
}
```

Ask Codex to make an edit and leave it in place. If type checking fails,
Latch puts the diagnostic alongside the edit result. A clean result stays
quiet. Replace the command with any check you already run, such as
`["uv", "run", "pytest"]` or `["cargo", "fmt", "--all", "--", "--check"]`.

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

In Codex, edit failures reach the agent during the turn. A turn-end
failure lets the turn finish and is available on the next prompt in
the same session.

Set `failure_output` to `excerpt` for the default preview, `compact` to
keep diagnostic lines, `tail` for the last lines of each stream, or
`rtk` to use a separately installed RTK filter. When the report includes
a `Saved log` path, ask your agent to open it or call Latch’s
`search_output` tool. You can also ask it to call `run_turn_end`
to run the current project’s turn-end checks on demand.

## Install the standalone CLI

Install the CLI when you want to run checks from your terminal or CI.
Download your platform’s `latch-0.1.0-*.tar.gz` from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/tag/v0.1.0).
From your downloads directory, extract it and install the binary. For
Apple Silicon:

```sh
tar -xzf latch-0.1.0-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 latch-0.1.0-aarch64-apple-darwin/bin/latch "$HOME/.local/bin/latch"
"$HOME/.local/bin/latch" --version
```

Use `x86_64-apple-darwin` for Intel Macs,
`x86_64-unknown-linux-gnu` for Linux x64, or
`aarch64-unknown-linux-gnu` for Linux ARM64. Add `$HOME/.local/bin` to your
`PATH` to run `latch` by name.

- `latch run turn_end` runs checks immediately, even with no Git changes.
- `latch explain after_edit` shows the commands and filters for that event.
- `latch output <id> --grep <text>` searches a saved failure without rerunning the check.

## Other coding agents

Latch also has integrations for Claude Code, Copilot CLI, OpenCode, Pi,
OpenHands CLI, Kimi Code, and Grok Build. Cursor, Devin, and Kiro CLI V3
are in preview. See the [integration guides on GitHub](https://github.com/BarakChamo/latch/tree/main/plugins)
for those agents.

## Troubleshoot

- **GitHub can’t clone the marketplace.** Sign in with an account that
  has repository access. If you use GitHub CLI for Git credentials,
  run `gh auth login` followed by `gh auth setup-git`, then retry.
- **No checks run in Codex.** Start a new local session in the project,
  review `/hooks`, and confirm `latch.json` is in the directory where
  the agent works. Read-only turns and no-op edits are skipped by default.
- **Using managed Codex Cloud?** The plugin runs local command hooks.
  In a hosted checkout you can install the standalone CLI and run
  `latch run turn_end` from its terminal or CI.

## Develop Latch

Install Rust and Node.js, run `npm ci`, then run `npm run check`.
See the [release guide](https://github.com/BarakChamo/latch/blob/main/docs/releasing.md)
for packaging and verification.
