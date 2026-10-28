//! The wither, against BDS's `WitherBoss` (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, BossEvent, LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::{Harness, all, click};
use sapling_api::{Block, DamageCause, Difficulty, EntityId, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const WITHER: &str = "minecraft:wither";
const SKULL: &str = "minecraft:wither_skeleton_skull";
/// Ticks a new wither, built or summoned, charges for.
const CHARGE: usize = 201;

fn watch(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// The values of `key` told of the entity `id`, as ints.
fn told(sent: &[RawPacket], id: u64, key: Key) -> Vec<i64> {
    let value = |plain: &Plain| match plain {
        Plain::Byte(byte) => i64::from(*byte),
        Plain::Short(short) => i64::from(*short),
        Plain::Int(int) => i64::from(*int),
        Plain::Long(long) => *long,
        _ => -1,
    };
    all::<SetEntityData>(sent).iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| &data.metadata).filter(|item| item.key == key).filter_map(|item| if let Value::Default(plain) = &item.value { Some(value(plain)) } else { None }).collect()
}

/// Alex, in creative, puts the last skull on a T of soul sand two blocks off; what it was told.
fn build(difficulty: Difficulty) -> (Harness, Vec<RawPacket>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        ctx.set_game_mode(ALEX, GameMode::Creative);
        for pos in [[0, 65, 3], [-1, 66, 3], [0, 66, 3], [1, 66, 3]] {
            ctx.set_block(pos, &Block::new("minecraft:soul_sand"));
        }
        for pos in [[-1, 67, 3], [1, 67, 3]] {
            ctx.set_block(pos, &Block::new(SKULL));
        }
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(SKULL, 1))));
    });
    h.tick_all();
    h.receive(1, &click([0, 66, 3], 1, Some(SKULL)));
    let sent = watch(&mut h, 3);
    (h, sent)
}

#[test]
fn a_skull_on_a_t_of_soul_sand_calls_a_wither_that_charges_then_bursts() {
    let (mut h, sent) = build(Difficulty::Normal);
    let wither = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == WITHER).expect("the wither");
    assert!((wither.position.x, wither.position.z) == (0.5, 3.5) && (wither.position.y - 65.05).abs() < 0.1, "{:?}", wither.position);
    let cleared = ask(&mut h, |ctx| [[0, 65, 3], [0, 66, 3], [-1, 67, 3], [0, 67, 3]].iter().all(|pos| ctx.block(*pos).is_some_and(|block| block.name == "minecraft:air")));
    assert!(cleared, "the T is taken away");
    let id = EntityId(wither.runtime_id);
    assert!(!ask(&mut h, move |ctx| ctx.hurt_entity(id, 10.0, DamageCause::Attack)), "untouchable while it charges");
    let charge = watch(&mut h, 200);
    let left = told(&charge, wither.runtime_id, Key::WitherInvulnerableTicks);
    assert!(left.windows(2).all(|pair| pair[1] < pair[0]) && left.last() == Some(&0), "{left:?}");
    assert!(all::<LevelEvent>(&charge).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion && event.data == 7), "its blast at the end");
    let bars: Vec<BossEvent> = all::<BossEvent>(&charge).into_iter().filter(|bar| bar.target_entity_id == wither.unique_id || bar.target_entity_id == wither.runtime_id as i64).collect();
    assert!(bars.len() >= 9 && bars.iter().all(|bar| bar.title == "entity.wither.name"), "every second: {}", bars.len());
    assert!(bars.first().unwrap().progress < bars.last().unwrap().progress, "it heals as it charges");
    assert!(ask(&mut h, move |ctx| ctx.hurt_entity(id, 10.0, DamageCause::Attack)));
}

#[test]
fn on_peaceful_the_t_stays() {
    let (mut h, sent) = build(Difficulty::Peaceful);
    assert!(all::<AddEntity>(&sent).iter().all(|added| added.entity_type != WITHER));
    assert_eq!(ask(&mut h, |ctx| ctx.block([0, 65, 3]).map(|block| block.name)).as_deref(), Some("minecraft:soul_sand"));
}

/// Against research/captures/bds-witherdeath.txt.
#[test]
fn killed_it_counts_down_200_ticks_flashing_faster_then_bursts_and_leaves_a_nether_star() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    let id = ask(&mut h, |ctx| ctx.spawn_entity(WITHER, [6.5, 66.0, 6.5]).unwrap());
    watch(&mut h, CHARGE);
    assert!(ask(&mut h, move |ctx| ctx.kill_entity(id)));
    let sent = watch(&mut h, 199);
    let counted = told(&sent, id.0, Key::WitherInvulnerableTicks);
    assert_eq!((counted.first(), counted.last(), counted.len()), (Some(&199), Some(&1), 199));
    let left = |data: &SetEntityData| data.metadata.iter().find(|item| item.key == Key::WitherInvulnerableTicks).and_then(|item| if let Value::Default(Plain::Int(left)) = item.value { Some(i64::from(left)) } else { None });
    let flashes: Vec<i64> = all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == id.0 && data.metadata.iter().any(|item| item.key == Key::WitherAerialAttack)).filter_map(left).take(16).collect();
    assert_eq!(flashes, [185, 172, 161, 152, 145, 140, 137, 136, 130, 128, 125, 124, 122, 120, 119, 118]);
    let deaths = all::<acacia_proto::packets::EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == id.0).count();
    assert!(deaths == 1 && all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion && event.data == 7), "its second death and blast at the end");
    watch(&mut h, 2);
    let left = ask(&mut h, move |ctx| (ctx.entity(id).is_some(), ctx.entities().into_iter().filter_map(|entity| entity.item).map(|item| item.name).collect::<Vec<_>>()));
    assert!(!left.0 && left.1.contains(&"minecraft:nether_star".to_owned()), "{left:?}");
}

#[test]
fn at_half_health_it_is_armoured_and_calls_three_skeletons() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_game_mode(ALEX, GameMode::Creative);
    });
    let id = ask(&mut h, |ctx| ctx.spawn_entity(WITHER, [6.5, 70.0, 6.5]).unwrap());
    assert!(!ask(&mut h, move |ctx| ctx.hurt_entity(id, 350.0, DamageCause::Attack)), "a summoned one charges too");
    watch(&mut h, CHARGE);
    assert!(ask(&mut h, move |ctx| ctx.hurt_entity(id, 350.0, DamageCause::Attack)));
    let sent = watch(&mut h, 3);
    assert_eq!(told(&sent, id.0, Key::WitherAerialAttack), [0]);
    let skeletons = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:wither_skeleton").count());
    assert_eq!(skeletons, 3);
    assert!(!ask(&mut h, move |ctx| ctx.hurt_entity(id, 5.0, DamageCause::Projectile)), "arrows glance off its armour");
}

/// Against research/captures/bds-witherdash.txt: 197 ticks after its blast, 2 a tick for 9 ticks.
#[test]
fn armoured_it_dashes_straight_at_its_prey_and_through() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(ALEX, GameMode::Creative);
    });
    let id = ask(&mut h, |ctx| ctx.spawn_entity(WITHER, [6.5, 70.0, 6.5]).unwrap());
    watch(&mut h, CHARGE);
    h.remote.run(move |ctx| {
        assert!(ctx.hurt_entity(id, 200.0, DamageCause::Attack));
        ctx.set_game_mode(ALEX, GameMode::Survival);
        ctx.give_effect(ALEX, sapling_api::Effect::Resistance, 1000, 4, false);
    });
    let feet: Vec<[f32; 3]> = (0..215).map(|_| ask(&mut h, move |ctx| ctx.entity(id).map_or([0.0; 3], |wither| wither.feet))).collect();
    let steps: Vec<f32> = feet.windows(2).map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][2] - pair[0][2])).collect();
    let fast: Vec<usize> = steps.iter().enumerate().filter(|(_, step)| **step > 1.9).map(|(tick, _)| tick).collect();
    assert_eq!(fast.len(), 9, "{steps:?}");
    assert!(fast.windows(2).all(|pair| pair[1] == pair[0] + 1) && (195..=199).contains(&fast[0]), "{fast:?}");
}

#[test]
fn its_heads_shoot_skulls_at_a_player_in_sight() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity(WITHER, [0.5, 66.0, 8.5]).is_some());
    });
    let mut sent = watch(&mut h, CHARGE);
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Survival));
    sent.extend(watch(&mut h, 40));
    let skulls = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:wither_skull").count();
    assert!(skulls >= 2, "{skulls}");
    let wither = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == WITHER).unwrap().runtime_id;
    assert!(told(&sent, wither, Key::WitherTarget1).contains(&1), "the middle head looks at Alex");
}
