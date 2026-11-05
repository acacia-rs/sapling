//! The thirteenth wave's group C drills (research/parity/THIRTEENTH-BRIEF.md): hits whose packet
//! claims eyes 0 to 7 above the actor's (`c1reach`, `!fromhit <kind> <rise>`), and boats and
//! minecarts around a TNT at 6 to 7.8 (`c1blast`).

use acacia_bot::Bot;
use acacia_bot::interact::to_wire;
use acacia_bot::proto::packets::InventoryTransaction;
use acacia_bot::proto::types::{
    Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionDataItemUseOnEntity, TransactionTransactionDataItemUseOnEntityActionType, TransactionTransactionType,
    Vec3f,
};

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityData", "EntityEvent", "UpdateAttributes", "CommandOutput", "LevelSoundEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "c1reach" => (reach(), SHOWN),
        "c1blast" => (blasts(), SHOWN),
        _ => return None,
    })
}

/// `!fromhit <kind> <rise>`: an attack on the newest mob of `kind` the actor knows, the packet's
/// eyes `rise` above the actor's; false for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let Some(rest) = line.strip_prefix("!fromhit ") else { return false };
    let mut words = rest.split_whitespace();
    let kind = format!("minecraft:{}", words.next().unwrap_or("cow"));
    let rise: f32 = words.next().and_then(|word| word.parse().ok()).unwrap_or(0.0);
    let Some(id) = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() else {
        println!("## fromhit: no {kind}");
        return true;
    };
    let [x, y, z] = actor.eye_position();
    let inventory = &actor.state().inventory;
    let on = TransactionTransactionDataItemUseOnEntity {
        entity_runtime_id: id,
        action_type: TransactionTransactionDataItemUseOnEntityActionType::Attack,
        hotbar_slot: i32::from(inventory.selected_hotbar_slot),
        held_item: to_wire(inventory.held()),
        player_pos: Vec3f { x, y: y + rise, z },
        click_pos: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
    };
    let legacy = TransactionLegacy { legacy_request_id: 0, legacy_transactions: None };
    let packet = InventoryTransaction { transaction: Transaction { legacy, transaction_type: TransactionTransactionType::ItemUseOnEntity, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemUseOnEntity(on) } };
    actor.client().send(&packet);
    println!("## fromhit {id} rise {rise}");
    true
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// A held cow 2 east of the survival actor, put back before each hit, hit with the packet's eyes
/// 0, 5.8, 6.2, 7 and 5.95 above the actor's.
fn reach() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "summon cow 2.5 -26 0.5", "effect @e[type=cow] slowness 255 255 true", "effect @e[type=cow] resistance 255 3 true", ""]);
    for rise in ["0", "5.8", "6.2", "7", "5.95", "0"] {
        lines.extend(["tp @e[type=cow] 2.5 -26 0.5".to_owned(), String::new(), format!("!fromhit cow {rise}"), String::new()]);
    }
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", ""])
}

/// A minecart and a boat at each of `distances` from a TNT at (0.5, -26, 0.5), round it 60 degrees
/// apart, watched 6 s after the TNT is summoned.
fn round(lines: &mut Vec<String>, distances: [f32; 3]) {
    for (at, (kind, far)) in ["minecart", "boat"].iter().flat_map(|kind| distances.map(|far| (*kind, far))).enumerate() {
        let angle = (at as f32 * 60.0).to_radians();
        lines.push(format!("summon {kind} {:.3} -26 {:.3}", 0.5 + far * angle.cos(), 0.5 + far * angle.sin()));
    }
    lines.extend(owned(["", "summon tnt 0.5 -26 0.5"]));
    waits(lines, 6);
    lines.extend(owned(["testfor @e[type=minecart]", "testfor @e[type=boat]", "kill @e[type=!player]", "fill -12 -27 -12 12 -27 12 stone", ""]));
}

fn blasts() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "fill -12 -27 -12 12 -27 12 stone", "fill -12 -26 -12 12 -16 12 air", "tp @s -10.5 -26 -10.5", ""]);
    round(&mut lines, [6.0, 6.6, 7.0]);
    round(&mut lines, [7.2, 7.5, 7.8]);
    with(lines, &["tp @s 0.5 -26 0.5", ""])
}
