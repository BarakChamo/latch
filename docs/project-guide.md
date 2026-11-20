# Project guide

Template-Version: 1

## Stack

- Rust 2024 with a declared minimum of 1.85. Development uses rustup stable and Cargo.
- npm 11 installs Turborepo for development. The CLI uses Serde for JSON and the Rust MCP SDK for its local stdio tool.
- `tenets.json` selects the Rust profile for the repository. Deployment targets are pending.

## Commands

| Command | Use |
| --- | --- |
| `npm ci` | Install locked development tooling |
| `cargo run --locked -p latch -- --version` | Check the CLI version during iteration |
| `npm run build` | Build the Cargo workspace |
| `npm run check` | Full acceptance gate covering formatting, Clippy, and Cargo tests |
| `npm run fmt` | Apply Rust formatting |
| `npm run package:release -- <target-or-npm>` | Stage packages and archives as described in the release guide |
| `npm run test:release -- <target>` | Verify the native archive and install the npm tarballs for the current platform |

## Workspace map

| Path | Family | Responsibility |
| --- | --- | --- |
| `crates/latch` | App | Owns the `latch` binary and project command execution |
| `plugins/` | Integration | Harness plugin manifests and hook wiring |
| `examples/` | Examples | Runnable TypeScript and Python projects |
| `distribution/` | Distribution | npm launcher and native release packaging |
| `apps/docs/` | App | Static Fumadocs site with its own npm lockfile and build |

Cargo owns Rust crate membership and dependencies. The workspace has one private
crate, `latch`, with no inter-crate dependencies. The `@latch/cli` npm workspace
exposes its Cargo tasks to Turborepo.

## Shared primitives

- Rust's `std::result::Result` and `assert!` are available from the standard library.
- The configuration boundary uses `serde_json::from_slice` and validates command arguments before execution.

## Tests

- Use Cargo's test harness. Unit tests live with their source in `#[cfg(test)]` modules.
- Local integration tests live in `crates/latch/tests/*.rs`. Tests that launch `sh` run on Unix.

## Documentation map

- Rust file preambles use a brief module-level `//!` comment explaining responsibility and context.
- Tenets templates live in `.agents/skills/tenets/templates/` and `.agents/skills/tenets-plan/templates/`.
- Package contracts live in workspace READMEs. The plugin boundary is documented in `plugins/README.md`.
- Dedicated topic pages, ADRs, and the learnings inbox are pending.

## Vocabulary

- A harness is the host that runs a coding agent. A hook is its callback for a lifecycle event.
- A turn is one agent response cycle. A tool is an operation the agent can invoke directly.
- The dedicated glossary is pending.

## Data and runtime

- `latch.json` in the command working directory owns explicit command lists, hook defaults, and per-hook path filters. Local `package.json` and `pyproject.toml` own discovered script definitions.
- Git and per-file SHA-256 fingerprints supply changed paths for harness hooks. Baselines are disposable state in `latch/baselines-v1` under each harness's config directory. Codex keys by session and turn, and Claude Code and Copilot CLI key by session. Edit baselines use a sibling per-turn directory and are consumed after the edit or discarded at a turn boundary. Pending turn-end failure reports are stored in `latch/pending-v1` by session and working directory until the next prompt consumes them.

## Change delivery

- The repository starts on `main`. The [release guide](releasing.md) covers versioning and publication.

## Rule addenda and recorded deviations

None recorded.
