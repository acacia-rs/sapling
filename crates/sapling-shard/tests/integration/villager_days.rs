//! A villager's day (villagers/schedule.rs, beds.rs, fear.rs): it gathers at the bell, sleeps in its
//! bed at night, flees a zombie and hides in its bed when a bell rings; and a player's standing with
//! its village (villagers/standing.rs).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain};
use common::raising::feet_of;
use common::{Harness, all, attack, click, config};
use sapling_api::{Block, BlockEntityData, Difficulty, PlayerId};
use sapling_world::World;

const BELL: [i32; 3] = [12, 65, 12];
/// A bed whose head is south of its foot.
const FOOT: [i32; 3] = [-5, 65, 8];
const HEAD: [i32; 3] = [-5, 65, 9];

/// Stone, a bell, a bed and Alex; the time of day `time`, and a villager summoned at `feet` by
/// `event`: its id.
fn village(time: i64, feet: [f32; 3], event: &'static str) -> (Harness, u64) {
    let mut level = World::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-32, 64, -32], [31, 64, 31], stone);
    let mut h = Harness::with(config(), level);
    h.enter(1, "Alex");
    let [x, y, z] = BELL;
    let entity = Value::Compound(vec![("id".into(), Value::String("Bell".into())), ("x".into(), Value::Int(x)), ("y".into(), Value::Int(y)), ("z".into(), Value::Int(z))]);
    let id = common::asking::ask(&mut h, move |ctx| {
        let half = |head: u8| Block { name: "minecraft:bed".into(), properties: format!("direction=0,head_piece_bit={head},occupied_bit=0") };
        assert!(ctx.set_block(BELL, &Block::new("bell")) && ctx.set_block_entity(BELL, &BlockEntityData::new(entity)));
        assert!(ctx.set_block(FOOT, &half(0)) && ctx.set_block(HEAD, &half(1)));
        ctx.set_time(time);
        ctx.set_difficulty(Difficulty::Easy);
        ctx.summon_as(None, "minecraft:villager_v2", feet, 0.0, Some(event)).expect("summoned").0
    });
    (h, id)
}

fn off(feet: [f32; 3], to: [i32; 3]) -> f32 {
    (feet[0] - to[0] as f32 - 0.5).hypot(feet[2] - to[2] as f32 - 0.5)
}

/// Ticks until the villager `id` is within `near` of `to`, at most `most` ticks: how near it came.
fn walks_to(h: &mut Harness, id: u64, to: [i32; 3], near: f32, most: usize) -> f32 {
    for _ in 0..most / 10 {
        (0..10).for_each(|_| drop(h.tick_all()));
        if off(feet_of(h, id), to) <= near {
            break;
        }
    }
    off(feet_of(h, id), to)
}

#[test]
fn an_unskilled_villager_gathers_at_the_bell_in_the_afternoon() {
    let (mut h, id) = village(8_500, [0.5, 65.0, 0.5], "minecraft:become_unskilled");
    let came = walks_to(&mut h, id, BELL, 2.5, 1200);
    assert!(came <= 2.5, "at the bell: {came}");
}

/// The villager `id`'s box and `FlagsExtended` as each `SetEntityData` in `sent` changed them.
fn lying(sent: &[acacia_proto::RawPacket], id: u64) -> Vec<(f32, f32, i64)> {
    let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id);
    let float = |data: &SetEntityData, key| data.metadata.iter().find(|item| item.key == key).and_then(|item| if let Meta::Default(Plain::Float(v)) = item.value { Some(v) } else { None });
    told.filter_map(|data| {
        let extended = data.metadata.iter().find_map(|item| if let Meta::FlagsExtended(flags) = &item.value { Some(flags.0) } else { None })?;
        Some((float(&data, Key::BoundingboxWidth)?, float(&data, Key::BoundingboxHeight)?, extended))
    })
    .collect()
}

#[test]
fn a_villager_sleeps_in_its_bed_at_night_lying_down_as_bds_shows() {
    let (mut h, id) = village(18_000, [0.5, 65.0, 0.5], "minecraft:become_unskilled");
    let mut sent = Vec::new();
    for _ in 0..120 {
        (0..10).for_each(|_| sent.extend(h.tick_all().to(1).to_vec()));
        if off(feet_of(&mut h, id), HEAD) <= 0.6 {
            break;
        }
    }
    (0..5).for_each(|_| sent.extend(h.tick_all().to(1).to_vec()));
    assert_eq!(lying(&sent, id), [(1.0, 0.3, (1 << 25) | (1 << 17) | (1 << 12))], "research/captures/bds-village.txt");
    (0..100).for_each(|_| drop(h.tick_all()));
    assert!(off(feet_of(&mut h, id), HEAD) <= 0.8, "it stays asleep");
    h.remote.run(|ctx| ctx.set_time(24_000));
    let woke: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(lying(&woke, id), [(0.6, 1.9, (1 << 25) | (1 << 17))], "it rises the tick morning comes");
}

#[test]
fn a_baby_villager_takes_a_bed_too() {
    let (mut h, id) = village(18_000, [0.5, 65.0, 0.5], "minecraft:entity_born");
    let came = walks_to(&mut h, id, HEAD, 0.8, 1200);
    assert!(came <= 0.8, "on its bed: {came}");
}

#[test]
fn a_villager_flees_a_zombie() {
    let (mut h, id) = village(18_000, [20.5, 65.0, 0.5], "minecraft:become_unskilled");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:zombie", [16.5, 65.0, 0.5]).is_some()));
    (0..40).for_each(|_| drop(h.tick_all()));
    let feet = feet_of(&mut h, id);
    assert!(feet[0] > 21.5, "it ran from the zombie: {feet:?}");
}

#[test]
fn a_rung_bell_sends_villagers_to_their_beds() {
    let (mut h, id) = village(0, [4.5, 65.0, 4.5], "minecraft:become_unskilled");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &Block::new("bell").with("attachment=standing,direction=0,toggle_bit=0"))));
    (0..60).for_each(|_| drop(h.tick_all()));
    h.receive(1, &click([2, 65, 0], 4, None));
    let came = walks_to(&mut h, id, HEAD, 1.5, 600);
    assert!(came <= 1.5, "it hid in its bed: {came}");
}

/// The runtime ids an iron golem has taken for its prey in `sent`.
fn golem_targets(sent: &[acacia_proto::RawPacket], golem: u64) -> Vec<i64> {
    let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == golem).flat_map(|data| data.metadata);
    items.filter(|item| item.key == Key::TargetEid).filter_map(|item| if let Meta::Default(Plain::Long(id)) = item.value { Some(id) } else { None }).collect()
}

/// The golems' runtime ids, one summoned by each of `events` (`None` plainly).
fn golems(h: &mut Harness, events: [Option<&'static str>; 2]) -> Vec<u64> {
    for (n, event) in events.into_iter().enumerate() {
        h.remote.run(move |ctx| assert!(ctx.summon_as(None, "minecraft:iron_golem", [6.5 + 4.0 * n as f32, 65.0, 6.5], 0.0, event).is_some()));
    }
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:iron_golem").map(|added| added.runtime_id).collect()
}

#[test]
fn only_the_village_golem_takes_a_player_who_hit_a_grown_villager_as_bds_shows() {
    let (mut h, id) = village(6_000, [3.5, 65.0, 0.5], "minecraft:become_unskilled");
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [0.5, 65.0, 0.5]));
    let ids = golems(&mut h, [Some("minecraft:from_village"), None]);
    h.receive(1, &attack(id));
    let seen: Vec<_> = (0..10).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!((golem_targets(&seen, ids[0]), golem_targets(&seen, ids[1])), (vec![1], vec![]), "bds-golems.txt");
}
