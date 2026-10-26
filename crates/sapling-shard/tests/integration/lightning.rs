//! Lightning bolts, against BDS's `LightningBolt` (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, RemoveEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemValue as Value, MetadataFlags1};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Block, Difficulty, PlayerId};

const ALEX: PlayerId = PlayerId(1);
const BOLT: &str = "minecraft:lightning_bolt";

fn ready(difficulty: Difficulty) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| ctx.set_difficulty(difficulty));
    h.tick_all();
    h
}

fn watch(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

#[test]
fn a_summoned_bolt_thunders_burns_hurts_once_and_goes() {
    let mut h = ready(Difficulty::Normal);
    let before = ask(&mut h, |ctx| ctx.health(ALEX)).unwrap();
    h.remote.run(|ctx| assert!(ctx.spawn_entity(BOLT, [2.5, 65.0, 0.5]).is_some()));
    let sent = watch(&mut h, 2);
    let bolt = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == BOLT).expect("the bolt");
    let sounds: Vec<String> = all::<LevelSoundEvent>(&sent).into_iter().map(|sound| sound.sound_id).collect();
    assert!(sounds.contains(&"thunder".to_owned()) && sounds.contains(&"explode".to_owned()), "{sounds:?}");
    let after = ask(&mut h, |ctx| ctx.health(ALEX)).unwrap();
    assert!((before - after - 5.0).abs() <= 1.0, "5, and maybe a tick of burning: {before} -> {after}");
    assert_eq!(ask(&mut h, |ctx| ctx.block([2, 65, 0]).map(|block| block.name)).as_deref(), Some("minecraft:fire"));
    let later = watch(&mut h, 40);
    assert!(all::<RemoveEntity>(&later).iter().any(|gone| gone.entity_id_self == bolt.unique_id), "gone after its flashes");
    assert!(ask(&mut h, |ctx| ctx.health(ALEX)).unwrap() > after - 5.0, "struck once");
}

#[test]
fn on_easy_a_bolt_lights_no_fire() {
    let mut h = ready(Difficulty::Easy);
    h.remote.run(|ctx| assert!(ctx.spawn_entity(BOLT, [2.5, 65.0, 0.5]).is_some()));
    watch(&mut h, 3);
    assert_eq!(ask(&mut h, |ctx| ctx.block([2, 65, 0]).map(|block| block.name)).as_deref(), Some("minecraft:air"));
}

#[test]
fn a_pig_becomes_a_zombified_piglin_and_a_creeper_is_charged_unhurt() {
    let mut h = ready(Difficulty::Normal);
    h.remote.run(|ctx| {
        assert!(ctx.spawn_entity("minecraft:pig", [6.5, 65.0, 6.5]).is_some());
        assert!(ctx.spawn_entity("minecraft:creeper", [-5.5, 65.0, -5.5]).is_some());
    });
    let added = watch(&mut h, 2);
    let creeper = all::<AddEntity>(&added).into_iter().find(|added| added.entity_type == "minecraft:creeper").expect("the creeper").runtime_id;
    h.remote.run(|ctx| {
        ctx.spawn_entity(BOLT, [6.5, 65.0, 6.5]);
        ctx.spawn_entity(BOLT, [-5.5, 65.0, -5.5]);
    });
    let sent = watch(&mut h, 2);
    let kinds = ask(&mut h, |ctx| ctx.entities().into_iter().map(|entity| entity.kind).collect::<Vec<_>>());
    assert!(kinds.contains(&"minecraft:zombie_pigman".to_owned()) && !kinds.contains(&"minecraft:pig".to_owned()), "{kinds:?}");
    let powered = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == creeper).flat_map(|data| data.metadata).any(|item| matches!(item.value, Value::Flags(flags) if flags.contains(MetadataFlags1::POWERED)));
    assert!(powered, "the creeper is told it is charged");
}

#[test]
fn a_struck_rod_is_powered_eight_ticks_and_scrapes_the_copper_it_stands_on() {
    let mut h = ready(Difficulty::Normal);
    h.remote.run(|ctx| {
        ctx.set_block([3, 64, 3], &Block::new("minecraft:oxidized_copper"));
        ctx.set_block([3, 65, 3], &Block::new("minecraft:lightning_rod").with("facing_direction=1,powered_bit=0"));
        ctx.spawn_entity(BOLT, [3.0, 65.0, 3.0]);
    });
    watch(&mut h, 1);
    let rod = ask(&mut h, |ctx| ctx.block([3, 65, 3]).unwrap());
    assert!(rod.name == "minecraft:lightning_rod" && rod.properties.contains("powered_bit=1"), "{rod:?}");
    let copper = ask(&mut h, |ctx| ctx.block([3, 64, 3]).unwrap().name);
    assert_ne!(copper, "minecraft:oxidized_copper", "scraped at least a stage");
    watch(&mut h, 8);
    assert!(ask(&mut h, |ctx| ctx.block([3, 65, 3]).unwrap()).properties.contains("powered_bit=0"));
}
