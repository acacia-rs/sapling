# Packs, functions, /schedule, /scriptevent

What BDS 1.26 does, as recorded on a testbox BDS (`tools/bds-packs-setup.sh` builds it: a 3 MB
resource pack `probe_rp` with `texturepack-required=true`, a behavior pack `probe_bp` of probe
functions and a `tick.json`) and read in bdsre. Built: DESIGN.md "Packs (as built)".

Captures: `research/captures/bds-packs.txt` (`cargo run -p sapling --example bds_pack_probe <addr>`:
the pack negotiation and the commands' syntax), `research/captures/bds-functions.txt`
(`bds_command_probe functions`, lines in `examples/shared/functions_drill.rs`).

## Resource packs

- ResourcePacksInfo: `must_accept` = `texturepack-required`; per pack size = the unpacked files'
  total (3000271, not the zip's 3349), `content_key` empty, `content_identity` the uuid even
  unencrypted, `cdn_url` empty; world template zero uuid, version `0.0.0`.
- DataInfo: `pack_id` `<uuid>_<version>`, `max_chunk_size` 102400, `size` and the 32-byte SHA-256
  are the zip's (BDS deflates the folder), type Resources. ChunkData `progress` = byte offset.
- Stack: `must_accept` as info, `name` empty, `game_version` `*`.

## Functions (bdsre: FunctionCommand::execute 0x143a7c100, FunctionManager::execute 0x140fc4150)

- Name: the path under `functions/` without `.mcfunction`, case-sensitive; `deep/a/b` works,
  `deep/a/b.mcfunction` does not. Comments and blank lines are not lines; leading spaces trimmed.
- Not loaded (so `commands.function.functionNameNotFound [name]`): an empty file, a file with a line
  that does not parse, a line starting with `/`.
- Count: the stack model; every popped line counts, a `function` line counts 1 and pushes its lines
  (nested of 2 lines + a 2-line function: 4); a failing line counts. The run stops when the count
  reaches `functionCommandLimit` (3 → hello 2, nested 3, self-recursive 3; 10000 when unset).
- Answer `commands.function.success [count]`; inner output goes nowhere (`say` still broadcasts),
  operators get the usual `[name: ...]` echo. Extra words: syntax error.
- `tick.json` `values` run every tick (20 a second) as the server.

## /schedule (bdsre 0x143b10560; scheduled run 0x1409ebcf0)

- `delay add f 40` → `commands.schedule.delay.functionQueued [f, 40, due tick]`; `2s` 40, `1d`
  24000, `1t` 1; `0` runs at once with no answer; `-1` → `commands.schedule.delay.negativeTime []`;
  unknown function (add or clear) → functionNameNotFound.
- Due: runs as the server (`[Server] later`), operators get a Json Text
  `§7§o[` + `commands.origin.server` + `: ` + `commands.schedule.functionRan [count, f]` + `]`.
- `delay clear f` → `commands.schedule.clear.success [cleared, f]`, none → `.clear.failure [f]`.
- `on_area_loaded add` ran at once as the player (area loaded); not built (needs ticking areas).

## /scriptevent (bdsre 0x1403ea1a0)

- Answer `Script event %%s has been sent [id]` (an untranslated literal), success. Message optional.
- `hi`, `minecraft:hi`, `ns:` → `commands.scriptevent.invalidNamespace []`.

## Unverified

- `messageSizeExceed`'s limit: sapling assumes 2048 characters.
- `/schedule` default mode when neither `replace` nor `append` is given: sapling replaces; the
  capture's replace/append runs did not show two queued copies running.
- The due tick parameter: BDS's level tick; sapling gives the shard's tick.
- Which pack wins when two behavior packs have a function of one name: sapling, the first listed in
  `world_behavior_packs.json`.
- The scheduled-run echo is sent as JsonWhisper (`tell_raw`), BDS sends Json; same text.
- Encrypted packs (content keys, `contents.json`) were not recorded; the key is passed through.
- CDN urls: not recorded; sapling passes the configured url and still serves chunks if asked.
