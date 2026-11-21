# Latch

**Give your coding agent feedback from the checks you already trust.**

An agent edits a file. Your type checker catches a regression. Latch gets
that error back into the agent’s workflow while the change is still fresh.
Passing checks stay quiet. If the output is long, the report points to the
full log.

[Run your first check](apps/docs/content/docs/quickstart.mdx) · [Download Latch](https://github.com/BarakChamo/latch/releases/latest) · [Read the docs](apps/docs/content/docs/index.mdx)

## Get started

Download the bundle for your machine and extract it. The bundle includes
the CLI and harness integrations. For example, to set up Codex:

```sh
./install codex
```

Start a new session in your project and approve the hooks when prompted.
Choose your setup in the [harness guide](apps/docs/content/docs/plugins.mdx)
for Claude Code, Copilot CLI, OpenCode, Pi, OpenHands, Kimi Code, or Grok Build.
The [compatibility guide](apps/docs/content/docs/hooks.mdx) shows when
each harness delivers feedback.

## Run the checks your project already has

If your project defines a `typecheck` script, add this `latch.json` in the
directory where you normally run it:

```json
{
  "hooks": {
    "turn_end": {
      "commands": [["npm", "run", "typecheck"]]
    }
  }
}
```

Latch takes a Git snapshot at the start of a turn. It skips unchanged
work and picks up fresh edits to files that were already dirty. Use
`after_edit` for fast checks and `turn_end` for a longer test suite. Keep
both in one file when teammates use different agents.

For JavaScript, TypeScript, and packaged Python projects, [package
scripts](apps/docs/content/docs/project-scripts.mdx) can configure checks
without a `latch.json`. See the [TypeScript and Python examples](examples/).

## Find your next step

| I want to… | Go to |
| --- | --- |
| See one automatic failure end to end | [Quickstart](apps/docs/content/docs/quickstart.mdx) |
| Install the CLI for a terminal or CI | [Standalone, npm tarballs, and Cargo](apps/docs/content/docs/cli.mdx#install-the-cli) |
| Control which files trigger checks or shorten failure output | [Configuration](apps/docs/content/docs/configuration.mdx) |
| Run a check from chat or recover a saved log | [Harness tools](apps/docs/content/docs/plugins.mdx#run-checks-from-chat) |
| Set up a hosted session | [Cloud guide](apps/docs/content/docs/cloud.mdx) |

## Develop Latch

With Rust installed through rustup and Node.js with npm 11, run `npm ci`
and `npm run check`. Use `npm run dev --prefix apps/docs` after installing
the docs dependencies with `npm ci --prefix apps/docs`.

[Project guide](docs/project-guide.md) · [Release guide](docs/releasing.md)
