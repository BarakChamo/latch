# Kimi Code CLI

Download and extract the standalone archive from
[GitHub Releases](https://github.com/BarakChamo/latch/releases/latest).
In Kimi Code, enter `/plugins install /absolute/path/to/extracted-latch/plugins/kimi`.
Approve the installation and start a new session. Kimi copies the
bundled Latch executable to its managed plugin directory.

The plugin runs `turn_end` on changed turns and sends output to the next
user prompt in the same conversation. Its MCP tools take an absolute
project directory. Use `/plugins remove latch` to remove it.

Kimi starts post-tool hooks in the background and the CLI can exit while
they are still running. Use `turn_end` for automatic checks. Run Kimi in
the checkout you want to validate. Kimi’s remote-tool mode runs hooks
beside the local CLI.

[Kimi plugin management](https://moonshotai.github.io/kimi-code/en/customization/plugins)
