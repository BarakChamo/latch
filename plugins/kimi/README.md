# Kimi Code

Run this inside Kimi Code:

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

Choose **Trust and install**, then run `/reload` or start a new session.
The plugin downloads and verifies the Latch binary on first use.

Latch runs checks after a changed turn. The model gets a failed report on
your next prompt by default. Use the default `report` mode for headless
Kimi Code 2.7.2, which completed immediately in our `continue_once` test.
The plugin exposes `run_turn_end` and `search_output` through MCP.
[Quickstart](https://barakchamo.github.io/latch/docs/quickstart/) · [Kimi Code plugin docs](https://github.com/MoonshotAI/kimi-code/blob/main/docs/en/customization/plugins.md)
