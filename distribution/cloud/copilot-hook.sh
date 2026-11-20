#!/bin/sh
# Forward Copilot's JSON only when the Actions setup provisioned Latch.
set -eu

if [ "${GITHUB_ACTIONS:-}" != true ]; then
    exit 0
fi

latch="$HOME/.local/share/latch-cloud/bin/latch"
if [ ! -x "$latch" ]; then
    printf '%s\n' 'Latch cloud setup did not install the CLI. Check copilot-setup-steps.' >&2
    exit 1
fi
exec "$latch" hook copilot "$@"
