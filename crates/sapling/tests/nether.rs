//! A bot goes to the Nether and back by `/execute in`. It
//! answers each dimension change as a client does (bds_portal_probe's `full` answer).

mod common;

use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{ChangeDimension, CommandOutput, LevelChunk, PlayerAction, ServerboundLoadingScreen};
use acacia_bot::proto::types::{Action, BlockCoordinates};
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_world::BlockAccess;
use common::{GROUND, SPAWN};
use sapling::{Command, Handler, Output, Permission, Player, Sender, Server, ShardCtx, World};

const WAIT: Duration = Duration::from_secs(10);
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
/// with the client's DimensionChangeAck, when the server says the columns are there.
async fn arrive(bot: &mut Bot, dimension: i32) -> ChangeDimension {
    let change: ChangeDimension = next(bot).await;
    assert_eq!(change.dimension, dimension);
    bot.client().send(&ServerboundLoadingScreen { r#type: 1, loading_screen_id: change.loading_screen_id });
    let ready: PlayerAction = next(bot).await;
    assert_eq!(ready.action, Action::DimensionChangeAck);
    bot.client().send(&ServerboundLoadingScreen { r#type: 2, loading_screen_id: change.loading_screen_id });
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    let runtime_entity_id = bot.client().runtime_entity_id();
    bot.client().send(&PlayerAction { runtime_entity_id, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 });
    bot.wait_ticks(30).await.expect("connected");
    change
}

/// The block a bot's own copy of its world has at `pos`, and its dimension.
fn block(bot: &Bot, [x, y, z]: [i32; 3]) -> (&'static str, i32) {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    (registry.get(view.block(x, y, z)).expect("a known block").name, bot.state().player.dimension)
}

/// Where the server holds the player's feet: the bot's own physics falls through the loading
/// screen a client waits in, and is put back by corrections it does not follow.
#[derive(Command)]
#[command(name = "feet")]
struct Feet;

impl Command for Feet {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(feet) = sender.player().and_then(|player| ctx.position(player.id)) else { return out.error("nobody") };
        let [x, y, z] = feet.feet.map(|c| (c + 1e-3).floor() as i32);
        out.print(format!("{x} {y} {z}"));
    }
}

async fn feet(bot: &mut Bot) -> [i32; 3] {
    let (said, _) = run(bot, "feet").await;
    let parts: Vec<i32> = said.split(' ').filter_map(|part| part.parse().ok()).collect();
    [parts[0], parts[1], parts[2]]
}

async fn run(bot: &mut Bot, line: &str) -> (String, bool) {
    bot.client().command(line);
    let output: CommandOutput = next(bot).await;
    (output.output[0].message_id.clone(), output.output[0].success)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_goes_to_the_nether_by_command_and_back() {
    let server = common::start(Server::builder().nether(nether()).handler(Operators).vanilla_commands().command::<Feet>()).await;
    let subscribed = PacketFilter::none().with(ChangeDimension::ID).with(PlayerAction::ID).with(CommandOutput::ID).with(LevelChunk::ID);
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig { subscribe: subscribed, ..BotConfig::default() }).await;
    alice.wait_ticks(20).await.expect("connected");

    let (x, z) = NETHER_SPAWN;
    alice.client().command(&format!("execute in nether run tp @s {x}.5 {} {z}.5", FLOOR_Y + 1));
    let change = arrive(&mut alice, 1).await;
    assert_eq!(([change.position.x, change.position.y, change.position.z], change.loading_screen_id), ([x as f32 + 0.5, (FLOOR_Y + 1) as f32, z as f32 + 0.5], Some(0)));
    assert_eq!((feet(&mut alice).await, block(&alice, [x, FLOOR_Y, z])), ([x, FLOOR_Y + 1, z], ("minecraft:netherrack", 1)));
    assert_eq!(run(&mut alice, &format!("tp @s {}.5 {} {z}.5", x + 3, FLOOR_Y + 1)).await, ("commands.tp.success.coordinates".to_owned(), true), "a tp in the Nether stays there");

    alice.client().command(&format!("execute in overworld run tp @s {}.5 {} {}.5", SPAWN.0, GROUND + 1, SPAWN.1));
    assert_eq!(arrive(&mut alice, 0).await.loading_screen_id, Some(1));
    assert_eq!((feet(&mut alice).await, block(&alice, [SPAWN.0, GROUND, SPAWN.1])), ([SPAWN.0, GROUND + 1, SPAWN.1], ("minecraft:grass_block", 0)));
    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}
