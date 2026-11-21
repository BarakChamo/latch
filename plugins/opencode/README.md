# OpenCode

Latch supports OpenCode v1 and v2 from one plugin entry point. It was tested
with 1.18.31, 1.19.6, and 2.3.2. The v2 integration uses the idle hook from
OpenCode 2.3.2.

Install Latch and find the plugin directory using the [harness guide](https://github.com/BarakChamo/latch/blob/main/apps/docs/content/docs/plugins.mdx).
For OpenCode v1, add the absolute path to `plugins/opencode/index.js` to
the `plugin` array in `opencode.json`. For v2, add the absolute path to
the `plugins/opencode` directory to the `plugins` array. Restart OpenCode
from a terminal where `latch --version` works.

Latch adds diagnostics to results from `edit`, `write`, and the native
patch tool. A failed turn-end check is included in the next prompt in the
same session. In v1, the report also appears as a TUI toast or on headless
stderr. In v2, the report goes to the server's stderr. To see it immediately
in a headless run, use `opencode --standalone --print-logs run "..."`.
Latch does not start another turn.
The plugin registers the local Latch MCP server with `run_turn_end` and
`search_output`.
