# Releasing Latch

Keep the version in `crates/latch/Cargo.toml`, `distribution/npm/package.json`,
and the harness plugin manifests aligned. Update `Cargo.lock` when the Rust
version changes. The packager verifies the versions before creating artifacts.

## Build the artifacts

Run the **Package release** workflow or push a matching `v<version>` tag.
The workflow runs Cargo tests and prepares archives and npm tarballs for
macOS and Linux on ARM64 and x64, and Windows on x64. Linux builds use glibc.
Before upload, each platform job installs both npm tarballs into a temporary
prefix and checks version output, a passing check, and a failed check. It also
verifies the standalone archive checksum and runs the extracted binary.

To build artifacts locally, use your Rust host triple in place of
`aarch64-apple-darwin`.

```sh
cargo build --release --locked -p latch --target aarch64-apple-darwin
npm run package:release -- aarch64-apple-darwin
npm run package:release -- npm
npm run test:release -- aarch64-apple-darwin
```

Each native archive includes the CLI, marketplace catalogs, and harness
plugins. The archive and npm wrapper both include `latch.schema.json`. A
SHA-256 checksum accompanies the archive. On macOS and Linux, the archive also contains an installer and a copy of Latch inside
each plugin. The release smoke test verifies hooks and MCP startup with
the standalone CLI absent from `PATH`.

## Verify a real harness session

With the native archive built and signed-in harness CLIs on your `PATH`, run
the live test. Omit `--harness` to test Codex, Claude Code, and GitHub
Copilot CLI together, or repeat the option to select multiple harnesses.

```sh
npm run test:live -- dist/latch-0.1.0-aarch64-apple-darwin.tar.gz
npm run test:live -- dist/latch-0.1.0-aarch64-apple-darwin.tar.gz --harness copilot
```

The test installs the bundled plugin in temporary directories, sends a
synthetic Git fixture to the selected harness services, and checks that
one edit runs both checks once in order and surfaces both failure diagnostics.
The test also retrieves a line that the compact preview omitted.
It also checks that an explicit plugin tool call returns its output.
Codex uses a temporary copy of its file-based credentials
when available. The test removes its temporary files on exit.

### OpenCode and Pi

Install OpenCode 1.18.31 or 1.19.6 and Pi 1.5.2 on your `PATH`, then run:

```sh
npm run test:extensions -- dist/latch-0.1.0-aarch64-apple-darwin.tar.gz
```

The test runs both real CLIs against a scripted localhost model with
temporary profiles. It verifies that no-op edits skip checks, changed
edits report diagnostics in the same turn, and a turn-end failure is
available on the next prompt without an extra model call. It also invokes
the bundled MCP tool and verifies its failure reaches the model.

## Publish the CLI

Confirm the npm scope and project license for the first public release.
The package currently uses `@barakchamo/latch` and `UNLICENSED`. Publish
the platform tarballs before the wrapper after inspecting the artifacts.

```sh
set -e
for tarball in dist/barakchamo-latch-{darwin,linux,win32}-*.tgz; do
  npm publish "$tarball" --access public
done
npm publish dist/barakchamo-latch-0.1.0.tgz --access public
```

Attach the native archives and checksums to the matching GitHub release.
From source, users can also install with `cargo install --locked --path crates/latch`.

## Distribute the plugins

On macOS and Linux, extract the native archive and follow the
[bundle install instructions](../README.md#get-started). Keep the folder
in place.

The source, npm, and Windows options require the CLI on the harness's
`PATH`. See the [OpenCode](../plugins/opencode/README.md) and
[Pi](../plugins/pi/README.md) guides for their plugin setup.

| Harness | Marketplace file | Installation |
| --- | --- | --- |
| Codex | `.agents/plugins/marketplace.json` | Add the Git repository or extracted root with `codex plugin marketplace add`, run `codex plugin add latch@latch-local`, then review hooks in `/hooks`. |
| Claude Code | `.claude-plugin/marketplace.json` | Add the repository or extracted root with `claude plugin marketplace add`, then run `claude plugin install latch@latch-marketplace`. |
| GitHub Copilot CLI | `.github/plugin/marketplace.json` | Add the repository or extracted root with `copilot plugin marketplace add`, then run `copilot plugin install latch@latch-marketplace`. |

The hook-enabled Codex plugin uses repo or manual distribution. OpenAI's
public plugin directory requires lifecycle hooks to be removed before
submission. [OpenAI plugin submission requirements](https://developers.openai.com/plugins/deploy/submission).
