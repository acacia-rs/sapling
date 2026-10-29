//! Brushing suspicious sand and gravel as BDS did (blocks/brushing.rs; research/captures/bds-brush.txt).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddItemEntity, BlockEntityData, InventoryContent, InventoryTransaction, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock};
use acacia_proto::RawPacket;
use acacia_proto::types::{ItemV4Extra, TransactionTransactionData, TransactionUseItemClientPrediction, TransactionUseItemTriggerType, WindowIDVarint};
use common::shooting::ready;
use common::{all, click};
use sapling_api::{Block, BlockEntityData as Kept, GameMode};

use common::asking::ask;

const CELL: [i32; 3] = [0, 64, 1];
/// The capture's desert pyramid sand: its seed rolled an archer pottery sherd on BDS.
const SAND_TABLE: &str = "loot_tables/entities/desert_pyramid_brushable_block.json";
const SAND_SEED: i32 = 1461741536;

/// A brush held on the top of `CELL`, as a client repeats it each tick.
fn held_click() -> InventoryTransaction {
    let mut click = click(CELL, 1, None);
    if let TransactionTransactionData::ItemUse(used) = &mut click.transaction.transaction_data {
        (used.trigger_type, used.client_prediction) = (TransactionUseItemTriggerType::SimulationTick, TransactionUseItemClientPrediction::Failure);
    }
    click
}

/// Alex in survival with a brush, suspicious sand of the capture's loot at `CELL`.
fn sand() -> common::Harness {
    let mut h = ready(GameMode::Survival, &[("minecraft:brush", 1, 0)], 60.0);
    h.remote.run(|ctx| assert!(ctx.set_block(CELL, &Block::new("minecraft:suspicious_sand"))));
    h.tick_all();
    let kept = Value::Compound(vec![
        ("id".into(), Value::String("BrushableBlock".into())),
        ("LootTable".into(), Value::String(SAND_TABLE.into())),
        ("LootTableSeed".into(), Value::Int(SAND_SEED)),
        ("brush_count".into(), Value::Int(0)),
        ("brush_direction".into(), Value::Byte(6)),
        ("type".into(), Value::String("minecraft:suspicious_sand".into())),
        ("x".into(), Value::Int(CELL[0])),
        ("y".into(), Value::Int(CELL[1])),
        ("z".into(), Value::Int(CELL[2])),
    ]);
    h.remote.run(move |ctx| assert!(ctx.set_block_entity(CELL, &Kept::new(kept))));
    h.tick_all();
    h
}

/// Holds the brush for `ticks` ticks (or none: just waits); what Alex was told each tick.
fn hold(h: &mut common::Harness, ticks: usize, brushing: bool) -> Vec<Vec<RawPacket>> {
    (0..ticks)
        .map(|_| {
            if brushing {
                h.receive(1, &held_click());
            }
            h.tick_all().to(1).to_vec()
        })
        .collect()
}

fn progress(packets: &[RawPacket]) -> Option<i32> {
    let kept = all::<BlockEntityData>(packets).into_iter().find(|data| [data.position.x, data.position.y, data.position.z] == CELL)?;
    match kept.nbt.decode().ok()?.value.get("brush_count") {
        Some(Value::Int(count)) => Some(*count),
        _ => None,
    }
}

/// The ticks at which a count was shown, with the count.
fn shown(ticks: &[Vec<RawPacket>]) -> Vec<(usize, i32)> {
    ticks.iter().enumerate().filter_map(|(tick, packets)| Some((tick, progress(packets)?))).collect()
}

/// The brush's wear as the last inventory sent shows it.
fn wear(packets: &[RawPacket]) -> Option<i32> {
    let windows = all::<InventoryContent>(packets);
    let inventory = windows.iter().rev().find(|window| window.window_id == WindowIDVarint::Inventory)?;
    let ItemV4Extra::Default(Some(brush)) = &inventory.input[0].extra else { return None };
    match brush.nbt.as_ref().and_then(|tag| tag.nbt.value.get("Damage")) {
        Some(Value::Int(worn)) => Some(*worn),
        _ => Some(0),
    }
}

fn sounds(packets: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(packets).into_iter().map(|sound| sound.sound_id).collect()
}

#[test]
fn a_held_brush_uncovers_suspicious_sand_in_ten_strokes_ten_ticks_apart() {
    let mut h = sand();
    let ticks = hold(&mut h, 100, true);
    // BDS: count 1 at the first click, then a stroke every 10 ticks; shown only as the progress moves.
    assert_eq!(shown(&ticks), [(0, 1), (21, 3), (51, 6)]);
    let done = ticks.iter().position(|packets| sounds(packets).contains(&"brush_completed".to_owned())).expect("uncovered");
    assert_eq!(done, 91);
    let last = &ticks[done];
    let dust = all::<LevelEvent>(last).into_iter().find(|event| event.event == LevelEventEvent::ParticleDestroyBlockNoSound).expect("the dust");
    assert_eq!([dust.position.x, dust.position.y, dust.position.z], [0.0, 64.0, 1.0], "at the corner");
    assert_eq!(all::<UpdateBlock>(last).last().map(|update| update.block_runtime_id), Some(common::hash("minecraft:sand")));
    let item = all::<AddItemEntity>(&ticks[done..].concat()).pop().expect("the find");
    assert_eq!(sapling_data::item_name(item.item.network_id.into()), Some("minecraft:archer_pottery_sherd"));
    assert_eq!([item.position.x, item.position.y, item.position.z], [0.5, 65.625, 1.5], "over the face brushed");
    assert_eq!(wear(&ticks.concat()), Some(1), "one use of the brush");
    // The brush sound every 9 ticks names the block's first state, then plain sand.
    let heard: Vec<usize> = ticks.iter().enumerate().filter(|(_, packets)| sounds(packets).contains(&"brush".to_owned())).map(|(tick, _)| tick).collect();
    assert_eq!(heard[..3], [0, 9, 18]);
    let first = all::<LevelSoundEvent>(&ticks[0]).pop().unwrap();
    assert_eq!(first.extra_data as u32, common::hash("minecraft:suspicious_sand"));
}

#[test]
fn a_brushed_block_let_go_falls_back_two_strokes_a_time_then_catches_up() {
    let mut h = sand();
    let ticks = hold(&mut h, 22, true);
    assert_eq!(shown(&ticks), [(0, 1), (21, 3)]);
    // Checks queued at each stroke (+40), rechecked every 20 while brushed within 40: the first fall
    // is the second stroke's check (tick 51 + 20), then one every 10.
    let ticks = hold(&mut h, 60, false);
    assert_eq!(shown(&ticks), [(49, 1), (58, 0)]);
    // Brushed again, the strokes owed since the last come one a tick.
    let ticks = hold(&mut h, 6, true);
    assert_eq!(shown(&ticks), [(0, 1), (2, 3), (5, 6)]);
    let entity = ask(&mut h, |ctx| ctx.block_entity(CELL).and_then(|data| data.get::<Value>().cloned())).expect("the entity");
    assert!(matches!(entity.get("item"), Some(Value::Compound(_))) && entity.get("LootTable").is_none(), "rolled at the first stroke");
}
