#!/bin/sh
# Installs the release binary for this platform and launches it for plugin hooks and tools.
set -eu

version='@LATCH_VERSION@'

fail() {
  printf 'latch: %s\n' "$*" >&2
  exit 1
}

# The source package uses the release version from its checked-in manifest.
case "$version" in
  @*)
    manifest="$(dirname -- "$0")/opencode/package.json"
    version=$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$manifest")
    ;;
esac
case "$version" in
  ''|*[!0-9A-Za-z.-]*) fail 'invalid Latch release version.' ;;
esac
release="https://github.com/BarakChamo/latch/releases/download/v$version"

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=darwin-arm64 ;;
  Darwin:x86_64) platform=darwin-x64 ;;
  Linux:aarch64|Linux:arm64) platform=linux-arm64 ;;
  Linux:x86_64) platform=linux-x64 ;;
  *) fail 'plugins support macOS and glibc Linux on ARM64 and x64.' ;;
esac

# Some harnesses pass HOME but strip XDG_CACHE_HOME when starting MCP.
directory="${HOME:?HOME is required}/.cache/latch/bin/$version/$platform"
binary="$directory/latch"

ensure_binary() (
  if [ -f "$binary" ] && [ -x "$binary" ]; then
    exit 0
  fi
  umask 077
  mkdir -p -- "$directory"
  staging=$(mktemp -d "$directory/.download.XXXXXX")
  trap 'rm -rf -- "$staging"' EXIT
  trap 'exit 1' HUP INT TERM
  asset="barakchamo-latch-$platform-$version.tgz"
  for name in "$asset.sha256" "$asset"; do
    curl -q -fsSL --proto '=https' --proto-redir '=https' --connect-timeout 10 --max-time 30 \
      --retry 2 --retry-max-time 24 \
      -o "$staging/$name" "$release/$name" ||
      fail "could not download $name from release v$version. Run the plugin installer in a terminal with GitHub access."
  done
  read -r expected _ < "$staging/$asset.sha256" || fail "could not read the checksum for $asset."
  if command -v sha256sum >/dev/null 2>&1; then
    checksum=$(sha256sum "$staging/$asset")
  else
    checksum=$(shasum -a 256 "$staging/$asset")
  fi
  [ "${checksum%% *}" = "$expected" ] || fail "checksum mismatch for $asset."
  tar -xOzf "$staging/$asset" package/latch > "$staging/latch" || fail "could not extract Latch from $asset."
  chmod 755 "$staging/latch"
  [ ! -d "$binary" ] || fail "expected a file at $binary."
  mv -f -- "$staging/latch" "$binary"
)

install_plugin() {
  case "$1" in
    codex|claude|copilot) ;;
    *) printf '%s\n' 'Usage: sh install-latch-plugin.sh --install-plugin codex|claude|copilot' >&2; exit 2 ;;
  esac
  command -v "$1" >/dev/null 2>&1 || fail "install the $1 CLI first."
  ensure_binary
  case "$1" in
    codex)
      codex plugin marketplace add BarakChamo/latch
      codex plugin add latch@latch-tools
      ;;
    claude)
      claude plugin marketplace add https://github.com/BarakChamo/latch.git
      claude plugin install latch@latch-tools
      ;;
    copilot)
      copilot plugin marketplace add BarakChamo/latch
      copilot plugin install latch@latch-tools
      ;;
  esac
  printf '%s\n' 'Start a new agent session to use Latch.'
}

if [ "${1:-}" = --install-plugin ]; then
  [ "$#" -eq 2 ] || { printf '%s\n' 'Usage: sh install-latch-plugin.sh --install-plugin codex|claude|copilot' >&2; exit 2; }
  install_plugin "$2"
  exit 0
fi

ensure_binary
exec "$binary" "$@"
