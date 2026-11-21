#!/bin/sh
# Selects the native executable shipped with the Codex plugin.
set -eu
case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=aarch64-apple-darwin ;;
  Darwin:x86_64) platform=x86_64-apple-darwin ;;
  Linux:aarch64|Linux:arm64) platform=aarch64-unknown-linux-gnu ;;
  Linux:x86_64) platform=x86_64-unknown-linux-gnu ;;
  *) printf '%s\n' 'Latch for Codex supports macOS and glibc Linux on ARM64 and x64.' >&2; exit 1 ;;
esac
exec "$(dirname -- "$0")/$platform" "$@"
