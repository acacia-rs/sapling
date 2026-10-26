//! A player ready to shoot or throw, and what it is told of it, for the projectile tests.

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, InventoryTransaction, PlayerAuthInput, UpdateAttributes};
use acacia_proto::types::{
    InputData, ItemV4, ItemV4Extra, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionDataItemRelease as Release,
    TransactionTransactionDataItemReleaseActionType as ReleaseAction, TransactionTransactionType, TransactionUseItemActionType, Vec3f,
};
use sapling_api::{GameMode, Item, PlayerId, Slot};

use super::{Harness, all, click, input_at};

pub const ALEX: PlayerId = PlayerId(1);
pub const STEVE: PlayerId = PlayerId(2);
pub const FEET: [f32; 3] = [0.5, 65.0, 0.5];
const AWAY: [f32; 3] = [-6.5, 65.0, -6.5];

/// A click in the air with what the hand holds.
pub fn click_air() -> InventoryTransaction {
    let mut click = click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut click.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    click
}

/// The item in use let go: a bow, a crossbow, a trident.
pub fn release() -> InventoryTransaction {
    let held_item = ItemV4 { network_id: 0, count: 0, metadata: 0, has_stack_id: false, stack_id: None, block_runtime_id: 0, extra: ItemV4Extra::Default(None) };
    let data = Release { action_type: ReleaseAction::Release, hotbar_slot: 0, held_item, head_pos: Vec3f { x: 0.5, y: 65.0 + sapling_data::EYE_HEIGHT, z: 0.5 } };
    let legacy = TransactionLegacy { legacy_request_id: 0, legacy_transactions: None };
    InventoryTransaction { transaction: Transaction { legacy, transaction_type: TransactionTransactionType::ItemRelease, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemRelease(data) } }
}

/// Alex at `FEET` in `mode`, holding `items` (name, count, data) from the first slot and looking
/// `pitch` down along +z; Steve out of the way.
pub fn ready(mode: GameMode, items: &[(&str, u8, i16)], pitch: f32) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let items: Vec<Item> = items.iter().map(|&(name, count, damage)| Item { damage, ..Item::new(name, count) }).collect();
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, mode);
        for (slot, item) in items.iter().enumerate() {
            assert!(ctx.set_item(ALEX, Slot::Main(slot as u8), Some(item)));
        }
        ctx.teleport(STEVE, AWAY);
    });
    h.tick_all();
    face(&mut h, 1, ALEX, FEET, pitch);
    h.receive(2, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..input_at(AWAY) });
    h.tick_all();
    h
}

/// The player `id` on `conn` stands at `feet` looking `pitch` down along +z; what `conn` was told meanwhile.
pub fn face(h: &mut Harness, conn: u64, id: PlayerId, feet: [f32; 3], pitch: f32) -> Vec<RawPacket> {
    h.remote.run(move |ctx| ctx.teleport_facing(id, feet, [0.0, pitch]));
    let mut sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(conn).to_vec()).collect();
    h.receive(conn, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], pitch, ..input_at(feet) });
    sent.extend(h.tick_all().to(conn).to_vec());
    sent
}

/// A mob of `kind` at `feet`, and its runtime id as Alex is told it.
pub fn spawn(h: &mut Harness, kind: &'static str, feet: [f32; 3]) -> u64 {
    h.remote.run(move |ctx| assert!(ctx.spawn_entity(kind, feet).is_some()));
    let added: Vec<AddEntity> = (0..5).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect();
    added.into_iter().find(|added| added.entity_type == kind).expect("the mob").runtime_id
}

/// Uses what Alex holds for `ticks` and lets go; what Alex was told over that and `after` more ticks.
pub fn draw(h: &mut Harness, ticks: usize, after: usize) -> Vec<RawPacket> {
    h.receive(1, &click_air());
    let mut sent = h.tick_all().to(1).to_vec();
    (1..ticks).for_each(|_| sent.extend(h.tick_all().to(1).to_vec()));
    h.receive(1, &release());
    sent.extend(watch(h, 1 + after));
    sent
}

/// Clicks in the air with what Alex holds; what Alex was told over that and `after` more ticks.
pub fn throw(h: &mut Harness, after: usize) -> Vec<RawPacket> {
    h.receive(1, &click_air());
    watch(h, 1 + after)
}

/// What Alex was told over `ticks` ticks.
pub fn watch(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// What Alex holds in `slot`.
pub fn held(h: &mut Harness, slot: Slot) -> Option<Item> {
    let seen = Arc::new(Mutex::new(None));
    let kept = seen.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.item(ALEX, slot));
    h.tick_all();
    seen.lock().unwrap().take()
}

/// The health values told in `sent`, of whoever.
pub fn healths(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

/// The entities of `kind` added in `sent`.
pub fn added(sent: &[RawPacket], kind: &str) -> Vec<AddEntity> {
    all::<AddEntity>(sent).into_iter().filter(|added| added.entity_type == kind).collect()
}
