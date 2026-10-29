//! A right click held on a block, as a vanilla client sends it: `StartItemUseOn`, a swing and a
//! `PlayerInput` ClickBlock, then a ClickBlock with the simulation tick as its trigger every tick
//! while held, then `StopItemUseOn` (acacia docs/research/vanilla-actions-2026-10-02.md "Held
//! right-click"). acacia-bot's own clicks end the use after a moment, which stops a brush.

use acacia_bot::Bot;
use acacia_bot::interact::{Face, to_wire};
use acacia_bot::proto::packets::{Animate, AnimateActionId, InventoryTransaction, PlayerAction};
use acacia_bot::proto::types::{
    Action, BlockCoordinates, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionType, TransactionUseItem,
    TransactionUseItemActionType, TransactionUseItemClientCooldownState, TransactionUseItemClientPrediction, TransactionUseItemHand,
    TransactionUseItemTriggerType, Vec3f,
};
use acacia_world::BlockAccess;

/// The face's middle, relative to the block's corner.
fn click_of(face: Face) -> [f32; 3] {
    match face {
        Face::Down => [0.5, 0.0, 0.5],
        Face::Up => [0.5, 1.0, 0.5],
        Face::North => [0.5, 0.5, 0.0],
        Face::South => [0.5, 0.5, 1.0],
        Face::West => [0.0, 0.5, 0.5],
        Face::East => [1.0, 0.5, 0.5],
    }
}

fn cell([x, y, z]: [i32; 3]) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

/// The block's id on the wire as the bot knows it; 0 when it does not.
fn wire_id(bot: &Bot, [x, y, z]: [i32; 3]) -> u32 {
    bot.world().and_then(|world| world.view()).map_or(0, |view| view.world().wire_id(view.block(x, y, z)))
}

fn click(bot: &Bot, pos: [i32; 3], face: Face, trigger: TransactionUseItemTriggerType, prediction: TransactionUseItemClientPrediction) -> InventoryTransaction {
    let inventory = &bot.state().inventory;
    let [ex, ey, ez] = bot.eye_position();
    let [cx, cy, cz] = click_of(face);
    InventoryTransaction {
        transaction: Transaction {
            legacy: TransactionLegacy { legacy_request_id: 0, legacy_transactions: None },
            transaction_type: TransactionTransactionType::ItemUse,
            actions: Vec::new(),
            transaction_data: TransactionTransactionData::ItemUse(TransactionUseItem {
                action_type: TransactionUseItemActionType::ClickBlock,
                trigger_type: trigger,
                block_position: cell(pos),
                face: face as u8,
                hotbar_slot: i32::from(inventory.selected_hotbar_slot),
                hand: TransactionUseItemHand::MainHand,
                held_item: to_wire(inventory.held()),
                player_pos: Vec3f { x: ex, y: ey, z: ez },
                click_pos: Vec3f { x: cx, y: cy, z: cz },
                block_runtime_id: wire_id(bot, pos),
                client_prediction: prediction,
                client_cooldown_state: TransactionUseItemClientCooldownState::Off,
            }),
        },
    }
}

fn use_on(bot: &Bot, action: Action, pos: [i32; 3], face: Face) -> PlayerAction {
    let (result, face) = match action {
        Action::StopItemUseOn => ([0, 0, 0], 0),
        _ => (pos, face as i32),
    };
    PlayerAction { runtime_entity_id: bot.client().runtime_entity_id(), action, position: cell(pos), result_position: cell(result), face }
}

/// Presses use on `face` of the block at `pos` (the bot already looks at it).
pub fn press(bot: &Bot, pos: [i32; 3], face: Face) {
    let me = bot.client().runtime_entity_id();
    bot.client().send(&use_on(bot, Action::StartItemUseOn, pos, face));
    bot.client().send(&Animate { action_id: AnimateActionId::SwingArm, runtime_entity_id: me, data: 0.0, has_swing_source: true, swing_source: Some("interact".into()) });
    let packet = click(bot, pos, face, TransactionUseItemTriggerType::PlayerInput, TransactionUseItemClientPrediction::Success);
    bot.client().send(&packet);
}

/// One tick of the use held after [`press`].
pub fn held(bot: &Bot, pos: [i32; 3], face: Face) {
    let packet = click(bot, pos, face, TransactionUseItemTriggerType::SimulationTick, TransactionUseItemClientPrediction::Failure);
    bot.client().send(&packet);
}

/// Lets go of the use started by [`press`].
pub fn release(bot: &Bot, pos: [i32; 3]) {
    let packet = use_on(bot, Action::StopItemUseOn, pos, Face::Up);
    bot.client().send(&packet);
}
