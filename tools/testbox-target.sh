#!/usr/bin/env bash
# Runs on testbox (piped through ssh by the sync scripts): links ~/<remote dir>/target to a dir under
# ~/targets and drops the least recently used targets no job holds while they total over 100 GB or the
# disk has under 40 GB free.
# Targets were on tmpfs while testbox was an HDD box (2026-10-24: a rebuild after one edit took 1210 s on
# disk, 47 s on tmpfs). Since testbox is this NVMe devbox (2026-10-24), tmpfs only cost: evicted whenever
# the shared box dipped under 8 GB available, every drill then rebuilt cold (aws-lc-sys and all, 30+ min
# under load on 2026-11-03). Disk targets survive between waves.
# usage: ssh testbox bash -s -- <remote dir> < tools/testbox-target.sh
set -u
remote=$HOME/$1 base=$HOME/targets
mine=$base/$(echo "$1" | tr / _)
mkdir -p "$remote" "$mine/testbox"
# A job synced but still queued for its lock holds nothing; the 3 h grace on .used keeps its target (and job
# script) from going under it, which left pollers waiting for a .done forever (2026-10-30).
held() { [ -e "$1" ] && ! flock -n "$1" true; }
busy() { held "$1/.busy" || held "$1/testbox/lock" || [ -n "$(find "$1/.used" -mmin -180 2>/dev/null)" ]; }
linked=$(readlink "$remote/target" 2>/dev/null)
if [ "$linked" != "$mine" ]; then
  if [ -L "$remote/target" ]; then
    # An old tmpfs target moves to disk once idle, its build kept; a job running in it keeps it until then.
    case $linked in /dev/shm/*) busy "$linked" && { echo "testbox-target: $linked busy, kept for now" >&2; exit 0; }
      [ -d "$mine/debug" ] || { rm -rf "$mine" && mv "$linked" "$mine" && echo "testbox-target: moved $linked to $mine"; }
      rm -rf "$linked" ;; esac
    rm "$remote/target"
  elif [ -e "$remote/target" ]; then
    # An old disk target is moved aside, not deleted: rm of ~100 GB would load the disk for everyone.
    stale=$HOME/stale-targets/$(basename "$mine")
    mkdir -p "$HOME/stale-targets" && mv "$remote/target" "$stale" && echo "testbox-target: old target moved to $stale"
  fi
  ln -s "$mine" "$remote/target"
fi
touch "$mine/.used"
# Cargo's incremental dir only grows; past 8 GB it goes, costing one slower rebuild.
incremental=$mine/debug/incremental
[ -d "$incremental" ] && [ "$(du -sm "$incremental" | cut -f1)" -gt 8192 ] && ! busy "$mine" && rm -rf "$incremental" && echo "testbox-target: dropped $(basename "$mine")'s incremental cache"
roomy() { [ "$(du -sm "$base" | cut -f1)" -le 102400 ] && [ "$(df -m --output=avail "$base" | tail -1)" -ge 40960 ]; }
for used in $(ls -tr "$base"/*/.used 2>/dev/null); do
  victim=$(dirname "$used")
  roomy && break
  [ "$victim" = "$mine" ] || busy "$victim" || { rm -rf "$victim"; echo "testbox-target: evicted $(basename "$victim")"; }
done
roomy || echo "testbox-target: targets over 100 GB or disk under 40 GB free with the rest busy" >&2
