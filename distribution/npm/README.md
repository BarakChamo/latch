# Latch

Download this wrapper and your platform’s binary tarball from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/latest).
Install both with npm from the downloads directory. For Apple Silicon:

```sh
npm install --global --offline ./barakchamo-latch-0.1.0.tgz ./barakchamo-latch-darwin-arm64-0.1.0.tgz
latch --version
```

Register the installed package as a local plugin source for Codex:

```sh
codex plugin marketplace add "$(npm root -g)/@barakchamo/latch"
codex plugin add latch@latch-local
```

Start Codex from a shell that can run `latch --version` and approve
the hooks in `/hooks`. See [CLI installation](https://github.com/BarakChamo/latch/blob/main/apps/docs/content/docs/cli.mdx#install-the-cli)
for the platform tarball names and the [harness guide](https://github.com/BarakChamo/latch/blob/main/apps/docs/content/docs/plugins.mdx)
for other agents.

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
