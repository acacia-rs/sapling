//! A bot goes to the Nether and back: by `/execute in`, and through a portal it lights. It
//! answers each dimension change as a client does (bds_portal_probe's `full` answer).

mod common;

use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{ChangeDimension, CommandOutput, LevelChunk, MovePlayer, MovePlayerMode, PlayerAction, ServerboundLoadingScreen};
use acacia_bot::proto::types::{Action, BlockCoordinates};
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_world::BlockAccess;
use common::{GROUND, SPAWN};
use sapling::{Handler, Permission, Player, Server, ShardCtx, World};

const WAIT: Duration = Duration::from_secs(10);
/// BDS's eyes over the feet, as MovePlayer carries them.
const EYE_HEIGHT: f32 = 1.62001;
/// Where the Nether's floor is, around the fixture's spawn divided by 8.
const FLOOR_Y: i32 = 69;
const NETHER_SPAWN: (i32, i32) = (SPAWN.0 / 8, SPAWN.1 / 8);

struct Operators;

impl Handler for Operators {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.set_permission(player.id, Permission::Operator);
    }
}

/// A void Nether with a netherrack floor around the spawn's place in it.
fn nether() -> World {
    let mut nether = World::void_in(sapling::NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    let (x, z) = NETHER_SPAWN;
    nether.fill([x - 12, FLOOR_Y, z - 12], [x + 12, FLOOR_Y, z + 12], netherrack);
    nether
}

/// The next `T` the bot is sent; other packets are dropped.
async fn next<T: Packet>(bot: &mut Bot) -> T {
    let found = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(raw)) if raw.id == T::ID => return raw.decode::<T>().expect("decodes"),
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(_) => {}
                None => panic!("the connection is gone"),
            }
        }
    });
    found.await.unwrap_or_else(|_| panic!("no {} in time", T::NAME))
}

/// Follows a dimension change as a client does: the loading screen starts at once and ends,
/// with the client's DimensionChangeAck, when the server says the columns are there. Returns the
/// change and the cell the server put the feet in: by MovePlayer mode Reset through a portal,
/// else the change's place. (The bot's own physics falls through the loading screen a client
/// waits in, so where it stands after is not where the server put it.)
async fn arrive(bot: &mut Bot, dimension: i32) -> (ChangeDimension, [i32; 3]) {
    let change: ChangeDimension = next(bot).await;
    assert_eq!(change.dimension, dimension);
    bot.client().send(&ServerboundLoadingScreen { r#type: 1, loading_screen_id: change.loading_screen_id });
    let mut placed = [change.position.x, change.position.y, change.position.z];
    let found = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(raw)) if raw.id == MovePlayer::ID => {
                    let moved: MovePlayer = raw.decode().expect("decodes");
                    if moved.mode == MovePlayerMode::Reset {
                        placed = [moved.position.x, moved.position.y - EYE_HEIGHT, moved.position.z];
                    }
                }
                Some(BotEvent::Packet(raw)) if raw.id == PlayerAction::ID => return raw.decode::<PlayerAction>().expect("decodes"),
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(_) => {}
                None => panic!("the connection is gone"),
            }
        }
    });
    let ready = found.await.expect("ready in time");
    assert_eq!(ready.action, Action::DimensionChangeAck);
    bot.client().send(&ServerboundLoadingScreen { r#type: 2, loading_screen_id: change.loading_screen_id });
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    let runtime_entity_id = bot.client().runtime_entity_id();
    bot.client().send(&PlayerAction { runtime_entity_id, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 });
    bot.wait_ticks(30).await.expect("connected");
    (change, placed.map(|c| (c + 1e-3).floor() as i32))
}

/// The block a bot's own copy of its world has at `pos`, and its dimension.
fn block(bot: &Bot, [x, y, z]: [i32; 3]) -> (&'static str, i32) {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    (registry.get(view.block(x, y, z)).expect("a known block").name, bot.state().player.dimension)
}

async fn run(bot: &mut Bot, line: &str) -> (String, bool) {
    bot.client().command(line);
    let output: CommandOutput = next(bot).await;
    (output.output[0].message_id.clone(), output.output[0].success)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_lights_a_portal_goes_through_it_and_comes_back_through_the_one_made() {
    let server = common::start(Server::builder().nether(nether()).handler(Operators).vanilla_commands()).await;
    let subscribed = PacketFilter::none().with(MovePlayer::ID).with(ChangeDimension::ID).with(PlayerAction::ID).with(CommandOutput::ID);
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig { subscribe: subscribed, ..BotConfig::default() }).await;
    let (x, y, z) = (SPAWN.0 + 3, GROUND + 1, SPAWN.1);
    for line in [format!("fill {x} {y} {z} {} {} {z} obsidian", x + 3, y + 4), format!("fill {} {} {z} {} {} {z} air", x + 1, y + 1, x + 2, y + 3), "give @s flint_and_steel".into(), "gamemode creative".into(), format!("tp @s {}.5 {y} {}.5", x + 1, z - 2)] {
        assert!(run(&mut alice, &line).await.1, "{line}");
    }
    alice.wait_ticks(10).await.expect("connected");
    alice.use_item_on_block([x + 1, y, z], acacia_bot::interact::Face::Up).await.expect("the spark");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(block(&alice, [x + 1, y + 1, z]).0, "minecraft:portal");

    // In: creative goes at once, to a portal made on the Nether's floor.
    assert!(run(&mut alice, &format!("tp @s {}.5 {} {z}.5", x + 1, y + 1)).await.1);
    let (_, there) = arrive(&mut alice, 1).await;
    assert_eq!((block(&alice, there), block(&alice, [there[0], there[1] - 1, there[2]]).0), (("minecraft:portal", 1), "minecraft:obsidian"));

    // Out of it and back in: home through the portal it came by.
    assert!(run(&mut alice, &format!("tp @s {}.5 {} {}.5", there[0] + 3, there[1], there[2] + 3)).await.1);
    // Arrived in a portal, the trip's cooldown is held at 10 ticks: they run out outside it.
    alice.wait_ticks(30).await.expect("connected");
    assert!(run(&mut alice, &format!("tp @s {}.5 {} {}.5", there[0], there[1], there[2])).await.1);
    let (back, home) = arrive(&mut alice, 0).await;
    assert_eq!(back.loading_screen_id, Some(1));
    assert_eq!((home[1], block(&alice, home)), (y + 1, ("minecraft:portal", 0)));
    assert!((x + 1..=x + 2).contains(&home[0]) && home[2] == z, "in the portal it lit: {home:?}");
    alice.disconnect().await;
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_goes_to_the_nether_by_command_and_back() {
    let server = common::start(Server::builder().nether(nether()).handler(Operators).vanilla_commands()).await;
    let subscribed = PacketFilter::none().with(MovePlayer::ID).with(ChangeDimension::ID).with(PlayerAction::ID).with(CommandOutput::ID).with(LevelChunk::ID);
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig { subscribe: subscribed, ..BotConfig::default() }).await;
    alice.wait_ticks(20).await.expect("connected");

    let (x, z) = NETHER_SPAWN;
    alice.client().command(&format!("execute in nether run tp @s {x}.5 {} {z}.5", FLOOR_Y + 1));
    let (change, placed) = arrive(&mut alice, 1).await;
    assert_eq!(([change.position.x, change.position.y, change.position.z], change.loading_screen_id), ([x as f32 + 0.5, (FLOOR_Y + 1) as f32, z as f32 + 0.5], Some(0)));
    assert_eq!((placed, block(&alice, [x, FLOOR_Y, z])), ([x, FLOOR_Y + 1, z], ("minecraft:netherrack", 1)));
    assert_eq!(run(&mut alice, &format!("tp @s {}.5 {} {z}.5", x + 3, FLOOR_Y + 1)).await, ("commands.tp.success.coordinates".to_owned(), true), "a tp in the Nether stays there");

    alice.client().command(&format!("execute in overworld run tp @s {}.5 {} {}.5", SPAWN.0, GROUND + 1, SPAWN.1));
    let (back, placed) = arrive(&mut alice, 0).await;
    assert_eq!(back.loading_screen_id, Some(1));
    assert_eq!((placed, block(&alice, [SPAWN.0, GROUND, SPAWN.1])), ([SPAWN.0, GROUND + 1, SPAWN.1], ("minecraft:grass_block", 0)));
    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}
