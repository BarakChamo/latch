# Python with uv

Install uv and make it available on your `PATH`. Run these commands from
this directory.

```sh
uv sync --locked
latch explain after_edit
latch run after_edit
latch run turn_end
```

Latch discovers both entry points in `pyproject.toml` and runs them through
uv. The optional [`latch.json`](latch.json) applies shared output settings
to those discovered checks.

Add an unused `import os` to [`src/latch_python_example/__init__.py`](src/latch_python_example/__init__.py)
and run `latch run after_edit` to see Ruff's diagnostic. Remove the import,
then change `Hello` to `Hi` in that file and run `latch run turn_end` to see
the failed unit test. Both failures exit with status 1. Restore the greeting
to make the tests pass again.

To try automatic feedback, start an agent session with Latch enabled in
this directory and ask the agent to add the unused import with its edit tool.
Check [where your agent shows edit failures](../../docs/usage.md#hooks-and-feedback).
