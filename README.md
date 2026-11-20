# Latch

Give your coding agent feedback from the checks you already use. Latch runs
lint, formatting, and tests after edits or at the end of a turn. Passing
checks stay quiet by default. When a check fails, Latch sends the diagnostics
to the agent so you spend less time rerunning commands and pasting errors.

## Get started

For the **private preview**, get the macOS or Linux bundle from the
[private preview release](https://github.com/BarakChamo/latch/releases/tag/preview-0.1.0)
with your invited GitHub account, or [build it locally](docs/releasing.md#build-the-artifacts).
Extract it and run the command for your agent.

| Agent | Install from the extracted folder |
| --- | --- |
| Codex | `./install codex` |
| Claude Code | `./install claude` |
| GitHub Copilot CLI | `./install copilot` |
| OpenCode | `./install opencode` |
| Pi | `./install pi` |

The plugin includes Latch. Keep the extracted folder and start a new
session in your project. In Codex, review project trust first, then approve
the hooks at startup or in `/hooks`.
The [quickstart](apps/docs/content/docs/quickstart.mdx) walks through a
first automatic check.

For terminal use, the archive has a standalone binary at `bin/latch`.
You can also install from this checkout with
`cargo install --locked --path crates/latch` or
[install the preview npm tarballs](apps/docs/content/docs/cli.mdx#install-the-cli).
The npm registry package is pending publication.

## Choose when to run checks

In JavaScript and TypeScript projects, define `hook:afteredit` or
`hook:turnend` in `package.json` to use your project's package manager.
For packaged Python projects, define `hook-afteredit` or `hook-turnend`
under `[project.scripts]` in `pyproject.toml` to use uv. Both approaches
are shown in the [runnable examples](examples/).

For any project, put a `latch.json` file where the checks should run.
For example, a Rust project can check formatting and run tests at turn end.

```json
{
  "hooks": {
    "turn_end": {
      "commands": [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "test", "--workspace"]
      ]
    }
  }
}
```

Automatic checks use Git to skip unchanged work by default. Each hook
can also filter by path or extension. Latch runs commands in order and
reports all failures by default. Set `on_failure` to `stop` to stop the
current command list at its first failure. Run `latch explain turn_end`
to see what will run or `latch run turn_end` to run it immediately.

Copy [`latch.schema.json`](latch.schema.json) from this repo or either
distribution next to your config and add
`"$schema": "./latch.schema.json"` for editor completion and validation.

## Learn more

- [Configuration and output modes](apps/docs/content/docs/configuration.mdx)
- [Hooks, commands, and tools in your agent](apps/docs/content/docs/plugins.mdx)
- [Hosted session compatibility](apps/docs/content/docs/cloud.mdx)

Run `npm ci --prefix apps/docs` and `npm run dev --prefix apps/docs` to
browse the docs locally.

## Develop Latch

Use Rust via rustup and Node.js with npm 11. Run `npm ci`, then
`npm run check` for formatting, lint, Rust tests, and schema validation.
`npm run build` builds the CLI at `target/debug/latch`. See the
[project guide](docs/project-guide.md) and the [release guide](docs/releasing.md).
