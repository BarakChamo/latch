# Latch

Install with Node.js 18 or later.

```sh
npm install -g latch-hooks
latch --version
```

Latch runs project checks after coding-agent edits or turns. Use the CLI
directly from a terminal or CI. [Quickstart and plugin setup](https://barakchamo.github.io/latch/docs/quickstart/)

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
