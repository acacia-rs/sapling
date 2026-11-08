//! Group Z of the fourteenth wave: the dolphin port (research/captures/bds-dolphin-findings.md,
//! bds-f14edolphin.txt; creatures/dolphins/{breath,swim}.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, MoveEntityDelta, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use common::{Harness, all};
use sapling_api::{Block, GameMode, PlayerId};

/// What one tick told of a dolphin: its `Air`, its flags, a hurt's data, its yaw byte.
#[derive(Default)]
struct Seen {
    air: Option<i16>,
    flags: Option<i64>,
    hurt: Option<i32>,
    yaw: Option<u8>,
}

fn seen(sent: &[RawPacket], id: u64) -> Seen {
    let mut seen = Seen::default();
    for item in all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata) {
        match (item.key, item.value) {
            (Key::Air, Value::Default(Plain::Short(air))) => seen.air = Some(air),
            (Key::Flags, Value::Flags(flags)) => seen.flags = Some(flags.0),
            _ => {}
        }
    }
    seen.hurt = all::<EntityEvent>(sent).iter().find(|event| event.runtime_entity_id == id && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data);
    seen.yaw = all::<MoveEntityDelta>(sent).iter().filter(|moved| moved.runtime_entity_id == id).find_map(|moved| moved.rot_y);
    seen
}

/// A glass-walled pool with water 65 to 71 inside x `x0..x0+4`, z -3..1, capped with glass or open.
fn pool(ctx: &mut sapling_api::ShardCtx<'_>,x0: i32, capped: bool) {
    for x in x0 - 1..=x0 + 5 {
        for z in -4_i32..=2 {
            for y in 65_i32..=72 {
                let rim = x == x0 - 1 || x == x0 + 5 || z == -4 || z == 2;
                let block = if y == 72 { if capped { "glass" } else { continue } } else if rim { "glass" } else { "water" };
                assert!(ctx.set_block([x, y, z], &Block::new(block)));
            }
        }
    }
}

#[test]
fn an_item_landing_is_heard_as_hit_ground() {
    // bds-w1pops.txt: an item dropped beside a sensor was heard at frequency 2 as it landed.
    let sensor = [4, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| assert!(ctx.update_block(sensor, &Block::new("minecraft:sculk_sensor"))));
    (0..4).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.drop_item(&sapling_api::Item::new("minecraft:cobblestone", 1), [3.5, 67.5, 3.5])));
    (0..30).for_each(|_| drop(h.tick_all()));
    let nbt = common::asking::ask(&mut h, move |ctx| ctx.block_entity(sensor).and_then(|data| data.get::<acacia_proto::nbt::Value>().cloned())).expect("a block actor");
    let event = nbt.get("VibrationListener").and_then(|listener| listener.get("event")).cloned();
    assert_eq!(event, Some(acacia_proto::nbt::Value::Int(31)), "hit_ground");
}

#[test]
fn a_dolphin_swims_up_for_breath_at_air_148_and_breaches_or_drowns_under_a_cap() {
    // bds-f14edolphin.txt: `Air` down a tick at a time from 4800; `SWIMMING` with `Air` 148, a
    // breach (`SPRINTING`) and its `Air` back to 4800; under a cap 2 every 10 ticks (data 9).
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        // 5 across each way: too narrow for a random breach's 6 open cells.
        pool(ctx, -7, false);
        pool(ctx, 2, true);
        assert!(ctx.spawn_entity("minecraft:dolphin", [-4.5, 66.0, -1.5]).is_some());
        assert!(ctx.spawn_entity("minecraft:dolphin", [4.5, 66.0, -1.5]).is_some());
    });
    let sent: Vec<Vec<RawPacket>> = (0..4880).map(|_| h.tick_all().to(1).to_vec()).collect();
    let ids: Vec<u64> = sent.iter().flat_map(|sent| all::<AddEntity>(sent)).filter(|added| added.entity_type == "minecraft:dolphin").map(|added| added.runtime_id).collect();
    let [open, capped] = ids[..] else { panic!("two dolphins: {ids:?}") };
    let swimming = MetadataFlags1::SWIMMING.0;
    for id in [open, capped] {
        let seen: Vec<Seen> = sent.iter().map(|sent| seen(sent, id)).collect();
        let airs: Vec<i16> = seen.iter().filter_map(|seen| seen.air).collect();
        assert!(airs.windows(2).take(200).all(|pair| pair[1] == pair[0] - 1), "{id}: a tick at a time: {:?}", &airs[..20]);
        let rose = seen.iter().position(|seen| seen.flags.is_some_and(|flags| flags & swimming != 0)).expect("swims up");
        assert_eq!(seen[rose].air, Some(148), "{id} swims up with `Air` 148");
        let yaws: Vec<(usize, u8)> = seen.iter().enumerate().filter_map(|(tick, seen)| seen.yaw.map(|yaw| (tick, yaw))).collect();
        // bds-dolphin.txt: 7 of 256 a tick while turning (260 of 423 steps).
        let steps: Vec<u8> = yaws.windows(2).filter(|pair| pair[1].0 == pair[0].0 + 1).map(|pair| pair[1].1.wrapping_sub(pair[0].1).min(pair[0].1.wrapping_sub(pair[1].1))).collect();
        let (sevens, over) = (steps.iter().filter(|step| **step == 7).count(), steps.iter().filter(|step| **step > 8).count());
        assert!(sevens * 3 > steps.len() && over * 100 < steps.len(), "{id} turns 10 degrees a tick at most: {sevens} of 7, {over} over 8 in {}", steps.len());
    }
    let opened: Vec<Seen> = sent.iter().map(|sent| seen(sent, open)).collect();
    let rose = opened.iter().position(|seen| seen.flags.is_some_and(|flags| flags & swimming != 0)).unwrap();
    let breached = opened.iter().skip(rose).position(|seen| seen.flags.is_some_and(|flags| flags & MetadataFlags1::SPRINTING.0 != 0)).expect("breaches");
    let breathed = opened.iter().skip(rose).position(|seen| seen.air == Some(4800)).expect("breathes again");
    // BDS's breathed 2 ticks into the breach; sapling's mobs breathe by where the tick began, so later.
    assert!(breached < 80 && (breached + 2..=breached + 4).contains(&breathed), "breached {breached}, breathed {breathed} ticks after it rose");
    let drowned: Vec<usize> = sent.iter().enumerate().filter(|(_, sent)| seen(sent, capped).hurt == Some(9)).map(|(tick, _)| tick).collect();
    assert!(drowned.len() >= 3 && drowned.windows(2).all(|pair| pair[1] - pair[0] == 10), "drowned every 10 ticks: {drowned:?}");
}
