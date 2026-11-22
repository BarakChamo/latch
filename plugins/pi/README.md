# Pi

Use Pi 1.5.2 or later.

```sh
pi install git:github.com/BarakChamo/latch
```

Start a new session. The plugin verifies and caches the Latch binary on first
use. Run `pi list` to see the package. Add `--local` to install it in a project
and share the resulting `.pi/settings.json`. Pi loads project packages after
workspace trust. [Pi package docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/packages.md)

For offline setup, extract the [standalone archive](https://github.com/BarakChamo/latch/releases/latest)
and pass its `plugins/pi` directory to `pi install`.

Latch adds diagnostics to results from Pi's `edit` and `write` tools.
Turn-end reports enter session history. With `continue_once` enabled, Pi can
respond once without another user message or an automatic recheck. The
plugin also registers `mcp__latch__run_turn_end` and
`mcp__latch__search_output`. [Quickstart](https://barakchamo.github.io/latch/docs/quickstart/)
