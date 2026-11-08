//! Group X of the fourteenth wave: sculk hearing a summoned TNT's fuse and the items a blast hurts, the
//! nearest of a tick's vibrations winning (research/captures/bds-b1sculk.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value as Nbt;
use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags2};
use common::{Harness, all};
use common::asking::ask;
use common::shooting::{ALEX, ready};
use sapling_api::{Block, GameMode, Item};

fn ticks(h: &mut Harness, ticks: usize) {
    (0..ticks).for_each(|_| drop(h.tick_all()));
}

fn told(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn took_off(sent: &[RawPacket]) -> bool {
    all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "takeoff" && sound.entity_type == "minecraft:bat")
}

/// bds-x2bat.txt: a hanging bat kept still with a player 4.1 off and let go, heard `takeoff`, with one 3.2 off.
#[test]
fn a_hanging_bat_lets_go_when_a_player_comes_within_4() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    ask(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block([4, 67, 0], &Block::new("minecraft:stone")));
        ctx.teleport(ALEX, [-0.5, 65.0, 0.5]);
        ctx.summon_as(None, "minecraft:bat", [4.5, 66.0, 0.5], 0.0, 0.0, None).expect("a bat");
    });
    assert!(!took_off(&told(&mut h, 20)), "4.1 off it hangs on");
    ask(&mut h, |ctx| ctx.teleport(ALEX, [1.5, 65.0, 0.5]));
    assert!(took_off(&told(&mut h, 5)), "3.2 off it lets go");
}

fn heard(h: &mut Harness, sensor: [i32; 3]) -> Option<Nbt> {
    let data = ask(h, move |ctx| ctx.block_entity(sensor).and_then(|data| data.get::<Nbt>().cloned()))?;
    data.get("VibrationListener").and_then(|listener| listener.get("event")).cloned()
}

/// bds-b1sculk.txt: a TNT summoned 7 off was heard (`prime_fuse`), then its blast behind an obsidian wall
/// hurt stacks lying 3 off the sensor, which lay on, and those were what the sensor heard, not the blast.
#[test]
fn a_summoned_tnt_is_heard_and_then_the_items_its_blast_hurt_over_the_blast() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let sensor = [0, 65, 0];
    ask(&mut h, move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block(sensor, &Block::new("minecraft:sculk_sensor")));
        for y in 65..=67 {
            for z in -2..=2 {
                assert!(ctx.set_block([4, y, z], &Block::new("minecraft:obsidian")));
            }
        }
        assert!(ctx.drop_item(&Item::new("minecraft:feather", 1), [3.5, 65.0, 0.5]));
    });
    ticks(&mut h, 20);
    assert!(ask(&mut h, |ctx| ctx.spawn_entity("minecraft:tnt", [7.5, 65.0, 0.5]).is_some()));
    ticks(&mut h, 12);
    assert_eq!(heard(&mut h, sensor), Some(Nbt::Int(39)), "prime_fuse");
    ticks(&mut h, 80);
    assert_eq!(heard(&mut h, sensor), Some(Nbt::Int(17)), "entity_damage, the nearer");
    let feathers = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.as_ref().is_some_and(|item| item.name == "minecraft:feather")).count());
    assert_eq!(feathers, 1, "the hidden stack took 1 of its 5");
}

fn laying(sent: &[RawPacket], id: u64) -> Vec<(Key, f32)> {
    let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata);
    items
        .filter_map(|item| match (item.key, item.value) {
            (key @ (Key::LayingAmount | Key::LayingAmountPrevious), Value::Default(Plain::Float(amount))) => Some((key, amount)),
            _ => None,
        })
        .collect()
}

/// bds-x2panda.txt: a lazy panda lay down (`FlagsExtended` bit 0) at about 1 in 400 a tick, its
/// `LayingAmount` going up 0.15 a tick from the tick after, `LayingAmountPrevious` a tick behind.
#[test]
fn a_lazy_panda_lies_down_its_laying_amount_rising_a_tick_on() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let id = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:panda", [0.5, 65.0, 6.5], 0.0, 0.0, Some("minecraft:panda_lazy")).expect("a panda").0);
    let mut sent = Vec::new();
    let mut ticked = 0;
    while !all::<SetEntityData>(&sent).iter().any(|data| data.runtime_entity_id == id && data.metadata.iter().any(|item| matches!(item.value, Value::FlagsExtended(MetadataFlags2(bits)) if bits == (1 << 25) | 1))) {
        assert!(ticked < 4000, "it lay down");
        sent = told(&mut h, 1);
        ticked += 1;
    }
    assert!(laying(&sent, id).is_empty(), "the amount moves from the tick after");
    let ramp: Vec<_> = (0..8).flat_map(|_| laying(&told(&mut h, 1), id)).collect();
    let rises = [0.15, 0.3, 0.45000002, 0.6, 0.75, 0.9, 1.0];
    let mut want = vec![(Key::LayingAmount, 0.15)];
    for pair in rises.windows(2) {
        want.extend([(Key::LayingAmount, pair[1]), (Key::LayingAmountPrevious, pair[0])]);
    }
    want.push((Key::LayingAmountPrevious, 1.0));
    assert_eq!(ramp, want);
}

fn bees_in(h: &mut Harness, hive: [i32; 3]) -> usize {
    let inside = ask(h, move |ctx| ctx.block_entity(hive).and_then(|data| data.get::<Nbt>().cloned()));
    match inside.as_ref().and_then(|data| data.get("Occupants")) {
        Some(Nbt::List(bees)) => bees.items.len(),
        _ => 0,
    }
}

/// By night, two hives facing north and bees summoned before the first; the hives.
fn hives_by_night(second: Option<[i32; 3]>, bees: &[f32]) -> (Harness, [i32; 3]) {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let first = [0, 65, 6];
    let bees = bees.to_vec();
    ask(&mut h, move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_time(18_000);
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(false)));
        for hive in std::iter::once(first).chain(second) {
            assert!(ctx.set_block(hive, &Block { name: "minecraft:beehive".into(), properties: "direction=2,honey_level=0".into() }));
        }
        for x in bees {
            ctx.spawn_entity("minecraft:bee", [x, 66.0, 3.5]).expect("a bee");
        }
    });
    (h, first)
}

/// bds-x2bee.txt: three of four bees went into a hive by night; the fourth left it full at once and
/// went into a second hive 10 off 13 s later.
#[test]
fn a_bee_at_a_full_hive_wanders_off_and_goes_into_another() {
    let second = [10, 65, 6];
    let (mut h, first) = hives_by_night(Some(second), &[-1.5, 0.5, 1.5, 2.5]);
    let mut ticked = 0;
    while bees_in(&mut h, second) == 0 {
        assert!(ticked < 3000, "the fourth found no other hive");
        ticks(&mut h, 20);
        ticked += 20;
    }
    assert_eq!(bees_in(&mut h, first), 3);
}

/// bds-x2bee.txt: fire set beside a hive by night let its two bees out a tick later, unheard, over and
/// behind it.
#[test]
fn fire_beside_a_hive_lets_its_bees_out_by_night() {
    let (mut h, hive) = hives_by_night(None, &[-0.5, 1.5]);
    let mut ticked = 0;
    while bees_in(&mut h, hive) < 2 {
        assert!(ticked < 600, "the bees went in");
        ticks(&mut h, 10);
        ticked += 10;
    }
    ask(&mut h, |ctx| assert!(ctx.set_block([1, 65, 6], &Block::new("minecraft:fire"))));
    let sent = told(&mut h, 3);
    let out: Vec<_> = all::<acacia_proto::packets::AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:bee").collect();
    assert_eq!(out.len(), 2, "both out");
    assert!(out.iter().all(|bee| [bee.position.x, bee.position.z] != [0.5, 5.5]), "beside it, at random");
    assert!(!all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "block.beehive.exit"));
    assert_eq!(bees_in(&mut h, hive), 0);
}
