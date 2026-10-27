//! Milestone 1 step 6: bots break and place blocks on a real connection. One builds in creative
//! mode while another watches its own copy of the world; a third, in adventure mode, is refused
//! and shown the blocks again.

use crate::common;

use std::collections::BTreeSet;
use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::interact::Face;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{InventoryTransaction, UpdateBlock};
use acacia_bot::proto::types::{
    BlockCoordinates, ItemV4, ItemV4Extra, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionType, TransactionUseItem,
    TransactionUseItemActionType, TransactionUseItemClientCooldownState, TransactionUseItemClientPrediction, TransactionUseItemHand, TransactionUseItemTriggerType, Vec3f,
};
use acacia_bot::{ActionError, Bot, BotConfig, BotEvent};
use acacia_world::BlockAccess;
use common::{GROUND, SPAWN};
use sapling::{Block, GameMode, Handler, Player, ShardCtx};

const STONE: &str = "minecraft:stone";
const DIRT: &str = "minecraft:dirt";
const AIR: &str = "minecraft:air";

/// A block of the row the server lays east of the spawn: `east` blocks away, `up` above the ground.
fn at(east: i32, up: i32) -> [i32; 3] {
    [SPAWN.0 + east, GROUND + up, SPAWN.1]
}

/// Lays a row of dirt with air above it when the first player joins; Alice builds, Carol only looks.
#[derive(Default)]
struct Plot {
    laid: bool,
}

impl Handler for Plot {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        if !std::mem::replace(&mut self.laid, true) {
            // Lit dirt beside grass turns to grass on a random tick.
            assert!(ctx.set_game_rule("randomTickSpeed", sapling::RuleValue::Int(0)));
            for east in 1..=4 {
                assert!(ctx.set_block(at(east, 0), &Block::new("dirt")));
                (1..=3).for_each(|up| assert!(ctx.set_block(at(east, up), &Block::air())));
            }
        }
        match player.name.as_str() {
            "Alice" => ctx.set_game_mode(player.id, GameMode::Creative),
            "Carol" => ctx.set_game_mode(player.id, GameMode::Adventure),
            _ => {}
        }
    }
}

/// The block a bot's own copy of the world has at `pos`.
fn seen(bot: &Bot, [x, y, z]: [i32; 3]) -> &'static str {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    registry.get(view.block(x, y, z)).expect("a known block").name
}

/// A click on the top of the block at `pos` by a client that says it holds stone.
fn click_with_stone(bot: &Bot, [x, y, z]: [i32; 3]) -> InventoryTransaction {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    let stone = view.world().wire_id(registry.states_of(STONE).next().expect("stone").0);
    let [ex, ey, ez] = bot.eye_position();
    let data = TransactionUseItem {
        action_type: TransactionUseItemActionType::ClickBlock,
        trigger_type: TransactionUseItemTriggerType::PlayerInput,
        block_position: BlockCoordinates { x, y, z },
        face: Face::Up as u8,
        hotbar_slot: 0,
        hand: TransactionUseItemHand::MainHand,
        held_item: ItemV4 { network_id: 1, count: 1, metadata: 0, has_stack_id: false, stack_id: None, block_runtime_id: stone, extra: ItemV4Extra::Default(None) },
        player_pos: Vec3f { x: ex, y: ey, z: ez },
        click_pos: Vec3f { x: 0.5, y: 1.0, z: 0.5 },
        block_runtime_id: view.world().wire_id(view.block(x, y, z)),
        client_prediction: TransactionUseItemClientPrediction::Success,
        client_cooldown_state: TransactionUseItemClientCooldownState::Off,
    };
    let legacy = TransactionLegacy { legacy_request_id: 0, legacy_transactions: None };
    InventoryTransaction { transaction: Transaction { legacy, transaction_type: TransactionTransactionType::ItemUse, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemUse(data) } }
}

async fn all(bots: &mut [&mut Bot], ticks: u32) {
    for bot in bots {
        bot.wait_ticks(ticks).await.expect("connected");
    }
}

/// The (position, layer) of every UpdateBlock in the plot's row that `bot` gets in the next half second;
/// the fixture's sheep graze elsewhere whatever randomTickSpeed is.
async fn updates(bot: &mut Bot) -> BTreeSet<([i32; 3], u32)> {
    let mut seen = BTreeSet::new();
    let end = tokio::time::Instant::now() + Duration::from_millis(500);
    while let Ok(event) = tokio::time::timeout_at(end, bot.next()).await {
        match event {
            Some(BotEvent::Packet(packet)) if packet.id == UpdateBlock::ID => {
                let update: UpdateBlock = packet.decode().expect("the bot decodes what sapling sent");
                let cell = [update.position.x, update.position.y, update.position.z];
                if cell[2] == SPAWN.1 && (SPAWN.0..=SPAWN.0 + 5).contains(&cell[0]) {
                    seen.insert((cell, update.layer));
                }
            }
            Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
            Some(_) => {}
            None => panic!("the connection is gone"),
        }
    }
    seen
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bots_break_and_place_blocks_and_adventure_is_refused() {
    let server = common::start(crate::common::builder().handler(Plot::default())).await;
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), BotConfig::default()).await;
    all(&mut [&mut alice, &mut bob], 20).await;
    assert_eq!((seen(&bob, at(1, 0)), seen(&bob, at(1, 1))), (DIRT, AIR), "a handler's blocks reach a player who joins later");

    // Alice breaks the first block of the row and builds on the second; Bob sees both.
    alice.break_block(at(1, 0)).await.expect("the server takes the break");
    alice.look_at([at(2, 1)[0] as f32 + 0.5, at(2, 1)[1] as f32, at(2, 1)[2] as f32 + 0.5]).await.expect("connected");
    alice.client().send(&click_with_stone(&alice, at(2, 0)));
    all(&mut [&mut alice, &mut bob], 20).await;
    for (who, bot) in [("Alice", &alice), ("Bob", &bob)] {
        assert_eq!((seen(bot, at(1, 0)), seen(bot, at(2, 1))), (AIR, STONE), "{who}");
    }

    // Carol, in adventure mode, is refused both and shown the blocks as they are, on both layers.
    let config = BotConfig { subscribe: PacketFilter::none().with(UpdateBlock::ID), ..BotConfig::default() };
    let mut carol = common::spawn(common::client(&server, "Carol"), config).await;
    carol.wait_ticks(20).await.expect("connected");
    assert_eq!((seen(&carol, at(1, 0)), seen(&carol, at(2, 1))), (AIR, STONE), "the column as it is now");
    let refused = carol.break_block(at(3, 0)).await;
    assert!(matches!(refused, Err(ActionError::Rejected(_))), "{refused:?}");
    updates(&mut carol).await;
    carol.client().send(&click_with_stone(&carol, at(4, 0)));
    let shown = updates(&mut carol).await;
    assert_eq!(shown, BTreeSet::from([(at(4, 0), 0), (at(4, 0), 1), (at(4, 1), 0), (at(4, 1), 1)]));
    all(&mut [&mut alice, &mut bob], 10).await;
    for (who, bot) in [("Alice", &alice), ("Bob", &bob), ("Carol", &carol)] {
        assert_eq!((seen(bot, at(3, 0)), seen(bot, at(4, 0)), seen(bot, at(4, 1))), (DIRT, DIRT, AIR), "{who}");
        assert_eq!(bot.disconnect_reason(), None, "{who}");
    }
    for bot in [&mut alice, &mut bob, &mut carol] {
        bot.disconnect().await;
    }
    server.shutdown().await;
}
