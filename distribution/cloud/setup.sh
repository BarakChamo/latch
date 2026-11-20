#!/bin/sh
# Build the repository's Latch binary for an ephemeral hosted runner.
set -eu

cd "$(dirname "$0")/../.."
cargo build --locked -p latch --target-dir target
install -Dm755 target/debug/latch "$HOME/.local/share/latch-cloud/bin/latch"
"$HOME/.local/share/latch-cloud/bin/latch" --version
