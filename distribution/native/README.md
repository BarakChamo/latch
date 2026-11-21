# Latch

## Install Latch

Keep this extracted directory in a stable location. From that directory,
add the executable to your shell’s path:

```sh
export LATCH_PLUGIN_DIR="$(pwd -P)"
export PATH="$LATCH_PLUGIN_DIR/bin:$PATH"
latch --version
```

For the marketplace commands and project setup, see the
[Latch guide on GitHub](https://github.com/BarakChamo/latch#install-a-plugin).

## Configuration

Define `hook:afteredit` or `hook:turnend` in `package.json`, or
`hook-afteredit` or `hook-turnend` under `[project.scripts]` in
`pyproject.toml`. You can also define commands and defaults in
`latch.json`.

Use `latch explain after_edit` or `latch explain turn_end` in your project
to inspect the resolved checks.

Copy the included `latch.schema.json` next to your project config and set
`"$schema": "./latch.schema.json"` for editor completion and validation.
