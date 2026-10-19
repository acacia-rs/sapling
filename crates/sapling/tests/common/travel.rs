//! A bot that changes dimension as a client does, for the Nether's and the End's tests.
#![allow(dead_code)]

use std::time::Duration;

use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{ChangeDimension, CommandOutput, MovePlayer, MovePlayerMode, PlayerAction, ServerboundLoadingScreen};
use acacia_bot::proto::types::{Action, BlockCoordinates};
use acacia_bot::{Bot, BotEvent};
use acacia_world::BlockAccess;
use sapling::{Handler, Permission, Player, ShardCtx};

pub const WAIT: Duration = Duration::from_secs(10);
/// BDS's eyes over the feet, as MovePlayer carries them.
pub const EYE_HEIGHT: f32 = 1.62001;

pub struct Operators;

impl Handler for Operators {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.set_permission(player.id, Permission::Operator);
    }
}

/// The next `T` the bot is sent; other packets are dropped.
pub async fn next<T: Packet>(bot: &mut Bot) -> T {
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
pub async fn arrive(bot: &mut Bot, dimension: i32) -> (ChangeDimension, [i32; 3]) {
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
pub fn block(bot: &Bot, [x, y, z]: [i32; 3]) -> (&'static str, i32) {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    (registry.get(view.block(x, y, z)).expect("a known block").name, bot.state().player.dimension)
}

pub async fn run(bot: &mut Bot, line: &str) -> (String, bool) {
    bot.client().command(line);
    let output: CommandOutput = next(bot).await;
    (output.output[0].message_id.clone(), output.output[0].success)
}
