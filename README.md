# sapling

A Rust framework for building Minecraft: Bedrock Edition servers, from hubs and minigames to survival.

[![Discord](https://img.shields.io/badge/Discord-Join%20us-5865F2?logo=discord&logoColor=white)](https://discord.gg/DNFJBJBVPr)

_Currently supported Minecraft version: `1.26.52` (protocol `2193`)._

> [!WARNING]
> sapling is pre-release. APIs change without notice, and the game data it needs must be generated
> locally before it builds (see [Getting started](#getting-started)).

## Why sapling

- **Built for many cores.** Each [shard](crates/sapling-shard) is an OS thread with its own `bevy_ecs`
  world, ticking at 20 Hz. Worlds are created and removed at runtime, and players move between shards
  with a two-phase handoff. Game code takes no locks, and outbound packets never block a shard.
- **Vanilla where it counts.** Mechanics are checked against the vanilla dedicated server (BDS): the
  same drill runs on both and the packets are compared. Overworld generation matches BDS block for
  block.
- **As much vanilla as you want.** A plugin can run a lobby with nothing but a platform, or turn on
  survival with mobs, redstone and the Nether.

## Features

### For plugin authors

- A [plugin API](crates/sapling-api) of handler traits (`on_join`, `on_chat`, `on_break`, …) with
  priorities, cancellable events and a `ShardCtx` facade that never exposes bevy types
- `#[derive(Command)]` and `#[derive(Form)]` for typed commands and forms, checked at compile time
- Scoreboards, boss bars, titles, sidebars, cameras, sounds and effects from plugin code
- Copy-on-write arenas for minigames, reset in one call

### Running a server

- BDS's `server.properties`, allow and ban lists, operators
- Online (Xbox Live) and offline login over RakNet (IPv4 and IPv6) or NetherNet, LAN discovery and the
  GameSpy4 server-list query
- A console that runs commands typed on stdin, with tab completion
- Per-address and per-session packet limits, idle kicks, a tick watchdog, tick metrics, and crash
  reports from a shard that saves before it stops
- Resource packs sent to joining clients, and behavior pack functions (`/function`, `/schedule`,
  `tick.json`)

### Worlds

- Vanilla [LevelDB world folders](crates/sapling-leveldb), read and saved, or in-memory worlds
- [Overworld generation](crates/sapling-worldgen) that matches BDS block for block: terrain, caves,
  features and structures. The Nether and the End are ported from BDS too.
- Nether and End portals, the ender dragon fight, strongholds found by eyes of ender

### Survival gameplay

- Blocks: doors, beds, rails, signs, liquids, falling blocks, redstone, pistons, hoppers, dispensers,
  command blocks and structure blocks
- Inventories, containers, crafting, furnaces, brewing, enchanting and the other workstations
- Health, hunger, effects, combat (shields, critical hits, knockback) and experience
- Mobs with vanilla spawning, attacks and loot; breeding, taming and riding
- Fishing, maps, bundles, projectiles, fireworks, lightning and weather
- Vanilla's commands, including `/execute`, `/loot`, `/structure` and `/camera`

## Example

```rust
use sapling::{BreakEvent, Handler, Player, Server, ShardCtx, World};

struct Lobby;

impl Handler for Lobby {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.broadcast(&format!("{} joined the lobby", player.name));
    }

    fn on_break(&mut self, _ctx: &mut ShardCtx<'_>, _player: &Player, event: &mut BreakEvent) {
        event.cancel();
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::void();
    let stone = world.block_id("minecraft:stone").ok_or("no stone")?;
    world.fill([-16, 63, -16], [16, 63, 16], stone);

    let server = Server::builder()
        .bind("0.0.0.0:19132".parse()?)
        .world(world, [0.5, 64.0, 0.5])
        .handler(Lobby)
        .start()
        .await?;
    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}
```

## Getting started

sapling isn't on crates.io. Clone it, then generate the game data it embeds. Mojang's data isn't
redistributed here, so you build it from your own copy of the game; the commands and what each needs
are in [crates/sapling-data/README.md](crates/sapling-data/README.md#provenance). Requires Rust 1.98
or newer.

Then run one of the examples and connect a Bedrock client to `127.0.0.1:19132`:

```sh
# A hub: forms, commands, a sidebar, a boss bar, protection and a second world on its own shard
cargo run -p sapling --example hub -- 19132 --offline

# A creative world generated from a seed, as BDS would generate it
cargo run --release -p sapling --example generated -- 19132 --seed=12345 --offline
```

Each example's header lists its options. [`examples/hub.rs`](crates/sapling/examples/hub.rs) is the
best place to start reading.

## Crates

| Crate | Role |
|---|---|
| [`sapling`](crates/sapling) | The `Server` builder and vanilla's commands; re-exports the plugin API |
| [`sapling-api`](crates/sapling-api) | What plugins see: handler traits, events, `ShardCtx`, commands and forms |
| [`sapling-macros`](crates/sapling-macros) | `#[derive(Command)]`, `#[derive(CommandEnum)]`, `#[derive(Form)]` |
| [`sapling-shard`](crates/sapling-shard) | The shard thread: its ECS world, schedule and game rules |
| [`sapling-net`](crates/sapling-net) | tokio networking: RakNet, NetherNet, login, per-player inbox and outbox |
| [`sapling-world`](crates/sapling-world) | Chunk columns, storage, the chunk encode cache, templates and arenas |
| [`sapling-leveldb`](crates/sapling-leveldb) | Vanilla world folders: `level.dat` and the chunk format |
| [`sapling-data`](crates/sapling-data) | Packet templates and registries recorded from the vanilla server |
| [`sapling-worldgen`](crates/sapling-worldgen) | World generation for all three dimensions, ported from BDS |

Protocol, transports, sessions and sign-in come from [acacia](https://github.com/acacia-rs/acacia).

## Goals

- Host any kind of server from one framework: plugins choose how much of vanilla to turn on.
- Behave as the vanilla dedicated server does wherever sapling implements a mechanic.
- Scale across cores: no locks in game code, no fixed number of worlds.
- Track the latest Minecraft version.

## Non-goals

- Supporting several protocol versions at once.
- Java Edition clients.
- Loading plugins at runtime. Plugins are crates compiled into the server.

## Not yet

- Custom items, blocks and entities, and the Bedrock Script API
- Raids and some late-game mobs; [docs/NEXT.md](docs/NEXT.md) has the full list
- Releases: there are no published binaries or Docker images yet

## Documentation

- [docs/DESIGN.md](docs/DESIGN.md): architecture, the decisions behind it and the open items
- [docs/NEXT.md](docs/NEXT.md): the work list, in order
- [docs/research](docs/research): how the vanilla server behaves, mechanic by mechanic, with the
  evidence
- `cargo doc --open` for the API reference

## Contributing

**[Join us on Discord](https://discord.gg/DNFJBJBVPr)** for questions, bug reports and to share servers
you've built. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to work on sapling, and report security
issues privately, as [SECURITY.md](SECURITY.md) describes.

## License

[MIT](LICENSE). Includes MIT-licensed code from cubiomes, featurelab and dragonfly; see
[THIRD-PARTY.md](THIRD-PARTY.md).

sapling is not an official Minecraft product and is not approved by or associated with Mojang or
Microsoft.
