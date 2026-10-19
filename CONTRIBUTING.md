# Contributing to sapling

Thanks for helping. Bug reports, research into how the vanilla server behaves and pull requests are all
welcome. For anything bigger than a fix, open an issue or ask on [Discord](https://discord.gg/DNFJBJBVPr)
first, so two people don't build the same thing and the design fits [docs/DESIGN.md](docs/DESIGN.md).
[docs/NEXT.md](docs/NEXT.md) is the work list.

## Setup

1. Rust 1.98 or newer.
2. Generate the game data sapling embeds; it isn't in the repository
   ([crates/sapling-data/README.md](crates/sapling-data/README.md#provenance)).
3. `cargo run -p sapling --example hub -- 19132 --offline` and connect a Bedrock client to it.

## Tests

- Test the crates you touched: `cargo test -p sapling-shard`. A whole-workspace build links many large
  test binaries and needs a lot of memory.
- Worldgen tests that compare against worlds the vanilla server generated skip themselves unless
  `SAPLING_ORACLE_WORLDS` points at a folder of them ([tools/worldgen-oracle](tools/worldgen-oracle)).
- `examples/bds_*` drive a running vanilla dedicated server (BDS) and record what it does.

## Matching vanilla

sapling copies the vanilla dedicated server, not Java Edition and not other servers. A change to how a
mechanic behaves needs evidence of what BDS does: a capture from one of the `bds_*` probes, an oracle
diff for worldgen, or a note in [docs/research](docs/research) that says how it was found. Put the
evidence in the pull request, and keep worldgen changes block-exact against the oracle worlds.

## Style

- There is no rustfmt configuration and the code is wider than rustfmt's defaults. Don't run `cargo fmt`;
  match the code around your change.
- No new clippy warnings in what you touch.
- Comment only what the code can't say: a non-obvious rule, an edge case, a vanilla quirk being copied.
- Update the docs your change makes wrong (DESIGN.md, a crate README, NEXT.md) in the same pull request.
- One topic per pull request. Say how you tested it.

## Licensing

- Contributions are under the [MIT License](LICENSE).
- Don't copy or port code from GPL or LGPL projects (PocketMine-MP, PowerNukkitX, Pumpkin and others).
  Citing how they behave is fine.
- Code ported from an MIT-compatible project keeps its notice: add it to [THIRD-PARTY.md](THIRD-PARTY.md).
- Never commit Mojang's data (pack files, recorded registries, structure templates). Commit the script
  that generates it instead.

## AI tools

You may use them. You are responsible for every line you submit: read it, run it and be able to explain
it. Say in the pull request if most of a change was generated.
