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

Choose your agent. The marketplace plugins support macOS and glibc Linux on ARM64 and x64.

### Codex

```sh
codex plugin marketplace add BarakChamo/latch --ref codex/marketplace
codex plugin add latch@latch-tools
```

In a new Codex session in your project, review and approve the Latch hooks in `/hooks`.

### Claude Code

```sh
claude plugin marketplace add 'BarakChamo/latch#codex/marketplace'
claude plugin install latch@latch-tools
```

### GitHub Copilot CLI

```sh
copilot plugin marketplace add 'https://github.com/BarakChamo/latch.git#codex/marketplace'
copilot plugin install latch@latch-tools
```

Start a new session in your project. On first use, the plugin downloads the
matching Latch binary from GitHub and caches it for future sessions. To
download it before opening the agent, use the [terminal installer](docs/usage.md#install-the-plugin-from-a-terminal)
as the same user on that machine.

For OpenCode, Pi, and other agents, see the [integration guides](plugins/).

## Run your first check

**`latch.json` is optional.** Latch can discover checks from `package.json` or `pyproject.toml`. Start the agent in the project directory after installing its dependencies.

If your npm project already has a `lint` script, add this entry to the existing `scripts` object in `package.json`.

```json
{
  "scripts": {
    "hook:afteredit": "npm run lint"
  }
}
```

Ask the agent to make a real change to a source file using its edit tool. Latch runs lint after a supported edit. A passing check is quiet. To see a failure, ask for an edit that breaks a lint rule your project enforces, then restore the file. Use `hook:turnend` for a check at the end of the turn.

Try the runnable [TypeScript project](examples/typescript/) for formatting, type checking, and Vitest, or the [Python project](examples/python/) for Ruff and unit tests with uv. Each example includes an edit that produces a failure.

### Configure checks

To start with a `latch.json`, install the [standalone CLI](#install-the-standalone-cli)
(v0.1.2 or later), then run this from your project root.

```sh
latch config init
```

This optional command creates a starter config and keeps package script discovery enabled.
You can then add your own checks. Here’s an example for Ruff in a uv project.

```json
{
  "hooks": {
    "after_edit": {
      "commands": [["uv", "run", "ruff", "check", "."]]
    }
  }
}
```

Explicit commands and discovered scripts can be used together. See the [configuration reference](docs/usage.md#configuration) for ordering, defaults, and filters.

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
without another message from you or an automatic recheck.

[Supported agents and how they receive failures](docs/usage.md#act-on-a-turn-end-failure)

## Install the standalone CLI

For terminal or CI use, download and extract your platform’s archive from [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest) and follow the [CLI installation guide](docs/usage.md#install-the-cli).

From your project, run `latch run after_edit` or `latch run turn_end` to execute that event’s checks immediately. This works with either package scripts or `latch.json`. Use `latch explain after_edit` to see what Latch resolved.

## Compatibility

The local columns show when the agent gets failures by default.

| Harness | Local test | After edit | Turn end | Managed cloud |
| --- | --- | --- | --- | --- |
| Codex | Live | Same turn | Next prompt | Latch CLI or CI |
| Claude Code | Live | Same turn | Next prompt | Single-repo setup |
| GitHub Copilot CLI | Live | Same turn | Next prompt | Actions runner setup |
| OpenCode v1 and v2 | Scripted | Same turn | Next prompt | Unverified |
| Pi | Scripted | Same turn | Next model request | Unverified |
| OpenHands CLI | Scripted | Next prompt | Next prompt | Workspace setup |
| Kimi Code CLI | Scripted | — | Next prompt | Unverified |
| Grok Build | Protocol | Agent delivery unverified | Hook stderr | Unverified |

“Scripted” means a real CLI with a scripted model. “Next prompt” means your
next message in the same session and project. Hosted agent feedback is unverified.
[Hook and feedback details](docs/usage.md#hooks-and-feedback) · [Cloud setup](docs/usage.md#hosted-sessions)

## Docs

- [Hook lifecycle and where failures appear](docs/usage.md#hooks-and-feedback)
- [Package script discovery](docs/usage.md#package-script-discovery) and [TypeScript and Python examples](examples/)
- [Configuration, filters, and JSON schema](docs/usage.md#configuration)
- [CLI commands, plugin tools, and saved logs](docs/usage.md#commands-and-saved-logs)
- [Agent integration guides](plugins/), [cloud setup](docs/usage.md#hosted-sessions), and [troubleshooting](docs/usage.md#troubleshooting)
