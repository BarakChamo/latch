# Latch

**Give your coding agent feedback from the checks you already trust.**

An agent edits a file. Your type checker catches a regression. Latch gets
that error back into the agent’s workflow while the change is still fresh.
Passing checks stay quiet. If the output is long, the report points to the
full log.

[Run your first check](apps/docs/content/docs/quickstart.mdx) · [Download Latch](https://github.com/BarakChamo/latch/releases/latest) · [Read the docs](apps/docs/content/docs/index.mdx)

## Get started

Download the npm wrapper and your platform’s binary package from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/latest).
Install both tarballs together with Node.js 18 or newer. For Apple Silicon:

```sh
npm install --global --offline ./barakchamo-latch-0.1.0.tgz ./barakchamo-latch-darwin-arm64-0.1.0.tgz
latch --version
```

Run the command from the directory containing both files. For Linux or
an Intel Mac, use the [matching platform package](apps/docs/content/docs/cli.mdx#install-the-cli).

If you use Codex, register the installed npm package as a local plugin source:

```sh
codex plugin marketplace add "$(npm root -g)/@barakchamo/latch"
codex plugin add latch@latch-local
```

Start Codex in your project and review the hooks in `/hooks`. For another
agent, see the [harness guide](apps/docs/content/docs/plugins.mdx).
The release also provides a [standalone CLI](apps/docs/content/docs/cli.mdx#install-the-cli).

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
