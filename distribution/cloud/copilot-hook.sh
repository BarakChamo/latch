#!/bin/sh
# Forward Copilot's JSON when the hosted setup provisioned Latch.
set -eu

latch="$(dirname "$0")/../../target/latch-cloud/bin/latch"
if [ -x "$latch" ]; then
    exec "$latch" hook copilot "$@"
fi
if [ "${GITHUB_ACTIONS:-}" = true ]; then
    printf '%s\n' 'Latch cloud setup did not install the CLI. Check copilot-setup-steps.' >&2
    exit 1
fi
