#!/usr/bin/env bash
# usage: PROBE_BDS=<private BDS copy> probe.sh <pack-dir> <log> [level-type]: fresh world with the pack, runs
# until the pack logs "PROBE done", then stops. Never point it at a shared BDS: probes swap its templates.
set -uo pipefail
D=$PROBE_BDS
pack=$(realpath "$1") log=$(realpath -m "$2") type=${3:-FLAT}
level="probe-$(date +%s)"
uuid=$(grep -m1 -oE '"uuid": "[^"]+"' "$pack/manifest.json" | cut -d'"' -f4)
rm -rf "$D/development_behavior_packs/probe"
cp -r "$pack" "$D/development_behavior_packs/probe"
mkdir -p "$D/worlds/$level"
echo "[{\"pack_id\": \"$uuid\", \"version\": [1, 0, 0]}]" > "$D/worlds/$level/world_behavior_packs.json"
grep -vE '^(level-name|level-seed|level-type|server-port|server-portv6|online-mode|content-log-console-output-enabled)=' $D/server.properties > $D/server.properties.new
printf "level-name=%s\nlevel-seed=${PROBE_SEED:-1}\nlevel-type=%s\nserver-port=19390\nserver-portv6=19391\nonline-mode=false\ncontent-log-console-output-enabled=true\n" "$level" "$type" >> $D/server.properties.new
rm -f $D/server.properties; mv $D/server.properties.new $D/server.properties
: > "$log"
{
  until grep -qE 'PROBE done|PROBE error|Quit correctly' "$log"; do sleep 1; done
  echo stop
  until grep -q 'Quit correctly' "$log"; do sleep 1; done
} | (cd $D && timeout ${PROBE_TIMEOUT:-1800} ./bedrock_server.exe > "$log" 2>&1)
rm -rf "$D/worlds/$level"
grep -c PROBE "$log"
