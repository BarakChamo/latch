# Hosted checks for this repository

The repository uses its own `latch.json` to check Rust formatting after
edits and run formatting and Clippy at turn end. The pull-request check
also runs the Rust test suite.

## GitHub Copilot

Choose the **Actions** environment for a Copilot session on this
repository. The `copilot-setup-steps` job builds the branch's Latch
binary and installs it under the checkout's ignored `target/` directory.
This lets the setup process and hook runner use the same binary even
when their home directories differ. GitHub
reads `.github/hooks/latch.json` from the default branch. The hook
wrapper uses that binary once setup has installed it. For local
Copilot CLI sessions, use the Latch plugin and skip the hosted setup step.

To verify the setup, open a disposable Copilot session, make one
deliberately misformatted Rust edit using the native edit tool, and
look for the formatting diagnostic on the edit result. Undo the
test edit before handing off the session.

[GitHub hooks](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/use-hooks) · [GitHub Actions setup](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment)

## Codex Cloud

For an environment with a shell setup phase, add
`sh distribution/cloud/setup.sh` after checkout. Then run the checks
from the repository root with
`target/latch-cloud/bin/latch run turn_end`.
`npm ci && npm run check` runs the full repository gate.

The hosted task must support command hooks to call Latch automatically.
OpenAI documents plugin installation and hook execution as separate
capabilities. In managed cloud orchestration, local command hooks are
unavailable. For automatic Latch feedback in a VM or container you
control, run Codex CLI inside it and load and trust the Latch plugin
there.

[Codex cloud hooks](https://learn.chatgpt.com/docs/hooks#hooks-with-cloud-orchestration) · [Bundled plugin hooks](https://developers.openai.com/plugins/build/plugins)
