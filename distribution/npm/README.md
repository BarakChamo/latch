# Latch

Download this wrapper and your platform’s binary tarball from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/latest).
Install both with npm from the downloads directory. For Apple Silicon:

```sh
npm install --global --offline ./barakchamo-latch-0.1.7.tgz ./barakchamo-latch-darwin-arm64-0.1.7.tgz
latch --version
```

For the standalone CLI commands and plugin setup, see the
[Latch guide on GitHub](https://github.com/BarakChamo/latch#readme).

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
