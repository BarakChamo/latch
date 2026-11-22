# Python with uv

Latch discovers the entry points in [`pyproject.toml`](pyproject.toml). Ruff runs after edits and unit tests run at turn end. The optional [`latch.json`](latch.json) applies shared output settings to both.

## Run the checks

Install uv and [the Latch CLI](https://barakchamo.github.io/latch/docs/cli/#install-the-cli). Run these from this directory.

```sh
uv sync --locked
latch run after_edit
latch run turn_end
```

## See a failing check

1. Add an unused `import os` to [`src/latch_python_example/__init__.py`](src/latch_python_example/__init__.py), then run `latch run after_edit`. Ruff reports the unused import.
2. Remove the import, change `Hello` to `Hi` in the same file, then run `latch run turn_end`. The unit test reports the mismatch.

Both failures exit with status 1. Restore the greeting to pass again.

## Try automatic feedback

Start a new session in this directory with the [Latch plugin enabled](https://barakchamo.github.io/latch/docs/quickstart/#install-a-plugin)
and ask the agent to make the same edit. [When your agent sees failures](https://barakchamo.github.io/latch/docs/hooks/#where-reports-appear)
