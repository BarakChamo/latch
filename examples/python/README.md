# Python with uv

Run these commands from this directory.

```sh
uv sync --locked
latch explain after_edit
latch run after_edit
latch run turn_end
```

Latch discovers the entry points in `pyproject.toml` and executes them through
uv. The optional `latch.json` shows how defaults apply to those entry points.
Add an unused import to see Ruff's edit diagnostic, or change the greeting to
see the turn-end test fail.
