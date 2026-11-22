# Try Latch in a project

The projects below demonstrate package script discovery and language-specific checks.
To add a check to your own project, follow the
[quickstart](https://github.com/BarakChamo/latch#run-your-first-check).

Install Latch using the [GitHub setup guide](https://github.com/BarakChamo/latch#install-the-standalone-cli).
Run the commands in the example's own directory with `latch` on your
`PATH`. To try automatic checks, start a new session in that directory
with the plugin enabled and ask your agent to make one of the edits below
using its native edit tool.

| Example | After a native edit | At turn end | What it shows |
| --- | --- | --- | --- |
| [TypeScript](typescript/) | Prettier | Type checking and Vitest | Script discovery with npm and no `latch.json` |
| [Python with uv](python/) | Ruff | Unit tests | Entry-point discovery with shared defaults in `latch.json` |
