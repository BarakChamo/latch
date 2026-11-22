<h1 align="center">Latch</h1>
<p align="center"><strong>Give your coding agent a feedback loop.</strong></p>
<p align="center">Latch runs your project’s linters, tests, and other checks as your agent works, and brings failures back into the session.</p>
<p align="center">
  <a href="#install-a-plugin">Install a plugin</a> ·
  <a href="#run-your-first-check">First check</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#compatibility">Compatibility</a> ·
  <a href="#install-the-standalone-cli">Standalone CLI</a> ·
  <a href="https://barakchamo.github.io/latch/">Docs</a>
</p>

## How it works

Use Latch to catch a lint error right after an edit or see a failing test at the end of a turn. Reuse commands you already run locally or in CI, and keep them in the project so teammates can use the same checks with different agents.

Plugins map their agents’ native hooks to two shared Latch events. Available events vary by agent, but your check commands stay in the project and work with any integration.

| Event | When checks run |
| --- | --- |
| `after_edit` | After a supported file-edit tool changes the worktree. Use it for fast checks such as linting. |
| `turn_end` | When the agent finishes a turn that changed the worktree. This catches shell edits and works well for tests. |

- **Skip unchanged work.** Latch compares Git-visible files before and after the edit or turn. No-op edits and read-only turns skip checks by default, even in a dirty worktree.
- **Keep passing checks quiet.** Success is silent by default, so failures are easier to spot in the agent session.
- **Run checks that match the change.** Filter by path or file extension so a Markdown-only edit can run Markdown lint and skip TypeScript checks.
- **Keep failure reports short.** Compact output surfaces useful lines such as errors and assertions. The agent can retrieve more detail from a saved log without rerunning the check.

## Install a plugin

Install Latch in your agent on macOS or glibc Linux (ARM64 and x64). On first use, the plugin downloads and verifies the matching Latch binary. Open a new session in your Git project after installing.

### Codex

```sh
codex plugin marketplace add BarakChamo/latch
codex plugin add latch@latch-tools
```

If Codex says `latch-tools` is already added from a different source, run
`codex plugin marketplace remove latch-tools`, then rerun the two commands above.

In a new Codex session in your project, review and approve the Latch hooks in `/hooks`.

### Claude Code

```sh
claude plugin marketplace add https://github.com/BarakChamo/latch.git
claude plugin install latch@latch-tools
```

### GitHub Copilot CLI

```sh
copilot plugin marketplace add BarakChamo/latch
copilot plugin install latch@latch-tools
```

### Pi

```sh
pi install git:github.com/BarakChamo/latch
```

Use Pi 1.5.2 or later.

### OpenCode

```sh
opencode plugin add 'github:BarakChamo/latch'
```

Use OpenCode 2.3.2 or later. [OpenCode v1 and project-local setup](https://barakchamo.github.io/latch/docs/plugins/#opencode)

### Grok Build

```sh
grok plugin install BarakChamo/latch --trust
```

Verified with Grok Build 1.0.92.

### Kimi Code

In Kimi Code, run:

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

Choose **Trust and install**, then run `/reload` or start a new session.

### OpenHands

In Agent Canvas, go to **Customize → Plugins → Add plugin**. Enter
`github:BarakChamo/latch` as the source, then click **Install**. The enabled
plugin loads in new local conversations.
[OpenHands SDK and Canvas setup](https://barakchamo.github.io/latch/docs/plugins/#openhands)

For OpenHands CLI, run this from the Git project you want to check.

```sh
curl -fsSLO https://github.com/BarakChamo/latch/releases/latest/download/install-latch-plugin.sh &&
  sh install-latch-plugin.sh install openhands
```

Already have `latch` on your path? Run `latch install openhands` in the project.
[Plugin settings, updates, and removal](https://barakchamo.github.io/latch/docs/plugins/)

## Run your first check

**Latch runs project commands in any language.** You can skip `latch.json`
if you define `hook:turnend` in `package.json` or `hook-turnend` as a
console script in a packaged uv project. For any other command, add it to
`latch.json`. [Package script names and discovery](docs/usage.md#package-script-discovery)

### Configure checks

For example, if `make lint` already works in your project, put this in
`latch.json` in the directory where you run it.

```json
{
  "hooks": {
    "turn_end": {
      "commands": [["make", "lint"]]
    }
  }
}
```

List each check as an array containing its executable and arguments. Add
more arrays to run checks in order. [Command syntax](docs/usage.md#command-syntax)

Create the file by hand or use `latch config init` with the
[standalone CLI](#install-the-standalone-cli). Explicit commands and discovered
scripts can be used together.

Start the agent in the project directory and ask it to change a file. Latch
runs the check when the turn finishes. A passing check is quiet. To check
immediately after supported file edits, use `after_edit`. [Quickstart](https://barakchamo.github.io/latch/docs/quickstart/#run-your-first-check) · [TypeScript and Python examples](examples/)

### Filters and failure output

This example runs your existing `npm test` script at turn end when a `.ts`
or `.tsx` file under `src/` changes. The same file must match both filters.

```json
{
  "hooks": {
    "turn_end": {
      "paths": ["src/**"],
      "extensions": ["ts", "tsx"],
      "commands": [["npm", "test"]],
      "failure_output": "compact"
    }
  }
}
```

Here’s an excerpt from the compact report for a failing Vitest run.

```text
Latch checks failed.
latch: command 1 ("npm") failed: exit status: 1
stdout:
Tests  1 failed | 36 passed (37)
[output compacted from 3175 bytes]
stderr:
FAIL  tests/message.test.ts > greets the caller by name
AssertionError: expected 'Hi, Latch!' to be 'Hello, Latch!'
Saved log: …/r-kgMl3D6EnjseUc7Nal1Wgf8Z.log
```

Open the saved log to read the full output without rerunning the tests. Use
`tail` to show the last lines of each stream, or leave `failure_output` unset
for the default excerpt.

[Filter examples](docs/usage.md#configuration) · [Failure output modes](docs/usage.md#commands-and-saved-logs)

### Give the agent a chance to act on a failure

With Latch v0.1.3 or later, set `"turn_end_feedback": "continue_once"` at the
top level of `latch.json`. On supported agents, this delivers the failed
turn-end report automatically and gives the agent one chance to respond
without another message from you or an automatic recheck. Grok Build
requires Latch v0.1.4 or later for this option.

[Supported agents and how they receive failures](docs/usage.md#act-on-a-turn-end-failure)

## Install the standalone CLI

For terminal or CI use, download and extract your platform’s archive from [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest) and follow the [CLI installation guide](docs/usage.md#install-the-cli).

From your project, run `latch run after_edit` or `latch run turn_end` to execute that event’s checks immediately. This works with either package scripts or `latch.json`. Use `latch explain after_edit` to see what Latch resolved.

## Compatibility

The local columns show when the agent gets failures by default, except where marked opt-in.

| Harness | After edit | Turn end | Managed cloud |
| --- | --- | --- | --- |
| Codex | Same turn | Next prompt | Latch CLI or CI |
| Claude Code | Same turn | Next prompt | Single-repo setup |
| GitHub Copilot CLI | Same turn | Next prompt | Actions runner setup |
| OpenCode v1 and v2 | Same turn | Next prompt | Unverified |
| Pi | Same turn | Next model request | Unverified |
| OpenHands CLI and SDK | Next prompt | Next prompt | Workspace setup |
| OpenHands Agent Canvas | — | Opt-in verified | Unverified |
| Kimi Code CLI | — | Next prompt | Unverified |
| Grok Build 1.0.92 | Same turn | No model feedback | Unverified |

“Next prompt” means your next message in the same session and project. Kimi 2.7.2 supports
`continue_once` in interactive and ACP sessions. Use `report` with `kimi -p`.
Hosted agent feedback is unverified.
[Hook and feedback details](docs/usage.md#hooks-and-feedback) · [Cloud setup](docs/usage.md#hosted-sessions)

## Docs

- [Hook lifecycle and where failures appear](docs/usage.md#hooks-and-feedback)
- [Package script discovery](docs/usage.md#package-script-discovery) and [TypeScript and Python examples](examples/)
- [Configuration, filters, and JSON schema](docs/usage.md#configuration)
- [CLI commands, plugin tools, and saved logs](docs/usage.md#commands-and-saved-logs)
- [Agent integration guides](plugins/), [cloud setup](docs/usage.md#hosted-sessions), and [troubleshooting](docs/usage.md#troubleshooting)
