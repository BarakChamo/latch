# OpenCode

Latch supports OpenCode v1 and v2 from one plugin entry point. It was tested
with 1.18.31, 1.19.6, and 2.3.2. The v2 integration uses the idle hook from
OpenCode 2.3.2.

Download the matching macOS or Linux archive from [GitHub Releases](https://github.com/BarakChamo/latch/releases/latest).
Extract it and open a terminal in the extracted directory. Run:

```sh
./install opencode
```

Keep the extracted folder in place. Restart OpenCode from your project.
Connect a provider and pick a model if needed, then use the
[GitHub first-check demo](https://github.com/BarakChamo/latch#run-your-first-check).

For a project-specific install, set the absolute path in `opencode.json`:

- **v1:** add `/path/to/extracted-latch/plugins/opencode/index.js` to
  the `plugin` array.
- **v2:** add `/path/to/extracted-latch/plugins/opencode` to
  the `plugins` array.

Choose either the installer or the project-specific setup. To remove the
installer’s registration, delete the `latch.js` link under
`${XDG_CONFIG_HOME:-$HOME/.config}/opencode/plugins`.

Latch adds diagnostics to results from `edit`, `write`, and the native
patch tool. A failed turn-end check is included in the next prompt in the
same session. In v1, the report also appears as a TUI toast or on headless
stderr. In v2, the report goes to the server's stderr. To see it immediately
in a headless run, use `opencode --standalone --print-logs run "..."`.
Latch does not start another turn.
The plugin registers the local Latch MCP server with `run_turn_end` and
`search_output`.
