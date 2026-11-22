# OpenCode

Use OpenCode 2.3.2 or later for the global install.

```sh
opencode plugin add 'github:BarakChamo/latch'
```

Start a new session. The plugin verifies and caches the Latch binary on first
use. Run `opencode plugin list --standalone` to see the active plugin.

For a project, add the same Git package to `plugins` in `opencode.json`, or
put a checkout of the package in `.opencode/plugins/latch/` for automatic
discovery. Use one setup per project. [OpenCode plugin discovery](https://opencode.ai/docs/plugins/#discover)

For new installs, use OpenCode v2. If you stay on v1, the installer in
the Latch archive prepares the matching OpenCode dependencies before
registering the plugin. This first-time setup needs npm and network access.
[OpenCode setup details](https://barakchamo.github.io/latch/docs/plugins/#opencode)

Latch adds diagnostics to results from `edit`, `write`, and the native patch
tool. By default, the model gets failed turn-end reports on the next prompt.
With `continue_once`, OpenCode v2 gives the agent one chance to respond
without another user message or an automatic recheck. The plugin exposes
`run_turn_end` and `search_output` through MCP.
[Quickstart](https://barakchamo.github.io/latch/docs/quickstart/)
