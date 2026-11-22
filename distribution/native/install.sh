#!/bin/sh
# Installs the archive's plugin using the selected harness's marketplace commands.

set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)

configure_mcp() {
    plugin="$root/plugins/$1"
    binary="$plugin/bin/latch"
    # An absolute executable path leaves the MCP process in the project directory.
    if [ "$binary" != "$(printf '%s' "$binary" | LC_ALL=C tr -d '[:cntrl:]')" ]; then
      printf '%s\n' 'Move the extracted Latch folder to a path without control characters.' >&2
      exit 1
    fi
    escaped=$(printf '%s' "$binary" | sed 's/\\/\\\\/g; s/"/\\"/g')
    config=$(mktemp "$plugin/.mcp.XXXXXX")
    trap 'rm -f -- "$config"' EXIT
    printf '{"mcpServers":{"latch":{"type":"stdio","command":"%s","args":%s}}}\n' "$escaped" "$2" > "$config"
    mv -- "$config" "$plugin/.mcp.json"
}

case "${1:-}" in
  codex)
    command -v codex >/dev/null
    configure_mcp codex '["mcp"]'
    codex plugin marketplace add "$root"
    codex plugin add latch@latch-tools
    ;;
  claude)
    claude plugin marketplace add "$root"
    claude plugin install latch@latch-tools
    ;;
  copilot)
    command -v copilot >/dev/null
    configure_mcp copilot '["mcp","--require-project-directory"]'
    copilot plugin marketplace add "$root"
    copilot plugin install latch@latch-tools
    ;;
  opencode)
    command -v opencode >/dev/null
    directory="${XDG_CONFIG_HOME:-"$HOME/.config"}/opencode/plugins"
    mkdir -p -- "$directory"
    target="$root/plugins/opencode/index.js"
    link="$directory/latch.js"
    if [ "$(readlink "$link" 2>/dev/null || true)" != "$target" ]; then
      ln -s -- "$target" "$link"
    fi
    ;;
  openhands|cursor|devin|grok|kiro)
    case "$1" in
      cursor) command -v cursor-agent >/dev/null || command -v agent >/dev/null ;;
      kiro) command -v kiro-cli >/dev/null ;;
      *) command -v "$1" >/dev/null ;;
    esac
    "$root/bin/latch" install "$@"
    ;;
  kimi)
    command -v kimi >/dev/null
    printf 'In Kimi Code, run: /plugins install %s\n' "$root/plugins/kimi"
    exit 0
    ;;
  pi)
    version=$(pi --version)
    if ! printf '%s\n' "$version" | awk -F '[.+-]' '
      $1 > 1 || ($1 == 1 && ($2 > 5 || ($2 == 5 && $3 >= 2))) { supported = 1 }
      END { exit !supported }
    '; then
      printf 'Latch requires Pi 1.5.2 or later. Found %s.\n' "$version" >&2
      exit 1
    fi
    pi install "$root/plugins/pi"
    ;;
  *)
    printf '%s\n' 'Usage: ./install codex|claude|copilot|opencode|pi|openhands|kimi|cursor|devin|grok|kiro' >&2
    exit 2
    ;;
esac

printf '%s\n' 'Start a new agent session to use Latch.'
