#!/bin/sh
# Normalize the numeric turn ID in Kimi Code's current Stop event for Latch 0.1.3.
set -eu

directory=$(dirname -- "$0")
LC_ALL=C sed -E 's/("turn_id"[[:space:]]*:[[:space:]]*)([0-9]+)([[:space:]]*[,}])/\1"\2"\3/g' |
  "$directory/../extension-launch.sh" hook kimi
