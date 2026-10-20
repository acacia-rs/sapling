#!/usr/bin/env bash
# The End's regions against their one-worker, seed-0 canon on testbox (~/oracle-canon/end: tests/end_oracle.rs).
# usage: tools/end-replay.sh <name>   env: REGION, as tools/oracle.sh
here=$(cd "$(dirname "$0")" && pwd)
TEST=end_oracle exec bash "$here/nether-replay.sh" "$1" end
