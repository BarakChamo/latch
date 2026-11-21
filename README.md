# Latch

Latch automatically runs your project’s linters, tests, and other checks while your coding agent works, so it can see failures without being asked to run the checks.

[How it works](#how-it-works) · [Compatibility](#compatibility) · [Install a plugin](#install-a-plugin) · [Run your first check](#run-your-first-check) · [Standalone CLI](#install-the-standalone-cli) · [Docs](#docs)

## How it works

Plugins map their agents’ native hooks to two shared Latch events. Available events vary by agent, but your check commands stay in the project and work with any integration.

| Event | When checks run |
| --- | --- |
| `after_edit` | After a supported file-edit tool changes the worktree. Use it for fast checks such as linting. |
| `turn_end` | When the agent finishes a turn that changed the worktree. This catches shell edits and works well for tests. |

- **Skip unchanged work.** Latch compares Git-visible files before and after the edit or turn. No-op edits and read-only turns skip checks by default, even in a dirty worktree.
- **Keep passing checks quiet.** Success is silent by default, so failures are easier to spot in the agent session.
- **Run checks that match the change.** Filter by path or file extension so a Markdown-only edit can run Markdown lint and skip TypeScript checks.
- **Keep failure reports short.** Compact output surfaces useful lines such as errors and assertions. The agent can retrieve more detail from a saved log without rerunning the check.

## Compatibility

These are the shipped integrations. The local columns describe the default
reporting behavior. Same-turn context reaches the model with the edit
result. A turn-end warning can be visible to you while the model is idle.

| Harness | Local verification | Local `after_edit` | Local `turn_end` | Managed cloud |
| --- | --- | --- | --- | --- |
| Codex | Live session | Model gets context after `apply_patch`. | UI or app-server warning. The model gets the failure on your next prompt. | Managed Codex Cloud has no Latch command hooks. Run Latch directly or in CI. |
| Claude Code | Live session | Model gets context after native edits. | Informational hook event. The model gets the failure on your next prompt. | Single-repo sessions outside Claude Projects can load committed hooks. Install the binary in setup. Hosted feedback unverified. |
| GitHub Copilot CLI | Live session | Model gets context after native edits. | Hook warning. The model gets the failure on your next prompt. | Copilot cloud agent requires repository hooks and runner setup. Same-turn context is documented by GitHub. Latch hosted feedback unverified. |
| OpenCode v1 | Real CLI, scripted model | Model gets context after native edits. | TUI toast or headless stderr. The model gets the failure on your next prompt. | Managed service unverified. |
| OpenCode v2 | Real CLI, scripted model | Model gets context after native edits. | Server stderr. The model gets the failure on your next prompt. | Managed service unverified. |
| Pi | Real CLI, scripted model | Model gets context after `edit` and `write`. | Report enters session history. The model reads it on its next request. | Managed service unverified. |
| OpenHands CLI | Real CLI, scripted model | Report reaches the next prompt after a native file edit. | Report reaches the next prompt. | Canvas and Cloud document repo hooks. Install Latch in the workspace. Hosted feedback unverified. |
| Kimi Code CLI | Real CLI, scripted model | Turn-end checks only. | Report reaches the next prompt. | With remote tools, hooks still run beside the CLI and see its local checkout. Managed cloud feedback unverified. |
| Grok Build | Hook protocol tested | Latch returns edit context. Model delivery unverified. | Hook stderr. | Grok Bot integration unverified. |

“Next prompt” means your next message in the same session and project.
See [hook and feedback details](docs/usage.md#hooks-and-feedback) and [hosted setup](docs/usage.md#hosted-sessions).
With MCP enabled, agents can [run checks on demand or search saved logs](docs/usage.md#commands-and-saved-logs).

With Latch v0.1.3 or later, set `"turn_end_feedback": "continue_once"` at the
top level of `latch.json`. A supported agent automatically receives the
failure and gets one chance to respond, without another message from you
or an automatic recheck. See [which agents can continue and how](docs/usage.md#act-on-a-turn-end-failure).

## Install a plugin

The marketplace plugins support macOS and glibc Linux on ARM64 and x64.

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

**`latch.json` is optional.** Latch can discover checks from `package.json` or `pyproject.toml`.

For example, if your npm project already has a `lint` script, add `hook:afteredit` to the existing `scripts` object in `package.json`:

```json
{
  "scripts": {
    "hook:afteredit": "npm run lint"
  }
}
```

Ask the agent to change a file with its edit tool, and Latch runs the script. Use `hook:turnend` for a check at the end of the turn. For Python projects using uv, see the [Python entry-point example](examples/python/).

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

Open the saved log for the full output. Use `tail` to show the last lines of
each stream, or leave `failure_output` unset for the default excerpt.

[Filter examples](docs/usage.md#configuration) · [Failure output modes](docs/usage.md#commands-and-saved-logs)

## Install the standalone CLI

For terminal or CI use, download and extract your platform’s archive from [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest) and follow the [CLI installation guide](docs/usage.md#install-the-cli).

From your project, run `latch run after_edit` or `latch run turn_end` to execute that event’s checks immediately. This works with either package scripts or `latch.json`. Use `latch explain after_edit` to see what Latch resolved.

## Docs

- [Hook lifecycle and where failures appear](docs/usage.md#hooks-and-feedback)
- [Package script discovery](docs/usage.md#package-script-discovery) and [TypeScript and Python examples](examples/)
- [Configuration, filters, and JSON schema](docs/usage.md#configuration)
- [CLI commands, plugin tools, and saved logs](docs/usage.md#commands-and-saved-logs)
- [Agent integration guides](plugins/) and [troubleshooting](docs/usage.md#troubleshooting)
