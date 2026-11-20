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
    codex plugin add latch@latch-local
    ;;
  claude)
    claude plugin marketplace add "$root"
    claude plugin install latch@latch-marketplace
    ;;
  copilot)
    command -v copilot >/dev/null
    configure_mcp copilot '["mcp","--require-project-directory"]'
    copilot plugin marketplace add "$root"
    copilot plugin install latch@latch-marketplace
    ;;
  *)
    printf '%s\n' 'Usage: ./install codex|claude|copilot' >&2
    exit 2
    ;;
esac

printf '%s\n' 'Start a new agent session to use Latch.'
