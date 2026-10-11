#!/usr/bin/env bash
# Runs an example on testbox under the replay of a run sent by tools/replay-world.sh.
# usage: tools/replay-ex.sh <log name> <replay world> <example> [args...]
#   e.g. tools/replay-ex.sh s1 rp structure_slice 20261003 20261003-structures 11 18 11 18 65 67
# env: ENVS adds to the replay's; the rest as tools/remote-example.sh.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
log=$1 name=$2
shift 2
ENVS="$(bash "$here/replay-env.sh" "$name") ${ENVS:-}" bash "$here/remote-example.sh" "$log" "$@"
