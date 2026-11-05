//! Lightning bolts, against BDS's `LightningBolt` (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, RemoveEntity, SetEntityData};
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

fn kinds(h: &mut Harness) -> Vec<String> {
    ask(h, |ctx| ctx.entities().into_iter().map(|entity| entity.kind).collect())
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
fn a_bolt_takes_away_the_items_and_orbs_in_reach_netherite_too() {
    // research/captures/bds-m3struck.txt: sticks, netherite, a nether star, cobblestone and an orb all went.
    let mut h = ready(Difficulty::Easy);
    h.remote.run(|ctx| {
        for name in ["minecraft:stick", "minecraft:netherite_ingot", "minecraft:nether_star"] {
            assert!(ctx.drop_item(&sapling_api::Item::new(name, 1), [-4.5, 65.0, -4.5]));
        }
        assert!(ctx.drop_item(&sapling_api::Item::new("minecraft:stick", 1), [5.5, 65.0, 5.5]));
        assert!(ctx.spawn_entity("minecraft:xp_orb", [-4.5, 65.0, -4.5]).is_some());
    });
    watch(&mut h, 20);
    h.remote.run(|ctx| assert!(ctx.spawn_entity(BOLT, [-4.5, 65.0, -4.5]).is_some()));
    watch(&mut h, 2);
    let left = kinds(&mut h).into_iter().filter(|kind| kind == "minecraft:item" || kind == "minecraft:xp_orb").count();
    assert_eq!(left, 1, "only the stick out of reach is left");
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
    assert!(kinds(&mut h).contains(&"minecraft:pig".to_owned()), "a pig turns half a second after the strike (bds-n3struck.txt)");
    let turning = watch(&mut h, 11);
    let kinds = kinds(&mut h);
    assert!(kinds.contains(&"minecraft:zombie_pigman".to_owned()) && !kinds.contains(&"minecraft:pig".to_owned()), "{kinds:?}");
    assert!(all::<LevelSoundEvent>(&turning).iter().any(|sound| sound.sound_id == "mob.pig.death" && sound.entity_unique_id == -1), "heard turning");
    let powered = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == creeper).flat_map(|data| data.metadata).any(|item| matches!(item.value, Value::Flags(flags) if flags.contains(MetadataFlags1::POWERED)));
    assert!(powered, "the creeper is told it is charged");
}

/// A rod on `under` at 3 65 3 and a bolt summoned at `bolt`; the sparks at the rod's corner.
fn rod_struck(h: &mut Harness, under: &'static str, bolt: [f32; 3]) -> usize {
    h.remote.run(move |ctx| {
        ctx.set_block([3, 64, 3], &Block::new(under));
        ctx.set_block([3, 65, 3], &Block::new("minecraft:lightning_rod").with("facing_direction=1,powered_bit=0"));
        ctx.spawn_entity(BOLT, bolt);
    });
    all::<LevelEvent>(&watch(h, 1)).into_iter().filter(|event| event.event == LevelEventEvent::ParticleElectricSpark && [event.position.x, event.position.y, event.position.z] == [3.0, 65.0, 3.0]).count()
}

fn powered(h: &mut Harness) -> bool {
    let rod = ask(h, |ctx| ctx.block([3, 65, 3]).unwrap());
    rod.name == "minecraft:lightning_rod" && rod.properties.contains("powered_bit=1")
}

#[test]
fn a_bolt_in_a_rods_cell_sparks_once_powers_it_eight_ticks_and_scrapes_the_copper_it_stands_on() {
    // research/captures/bds-o3rod.txt: one spark as it appears, the rod powered as it lands a tick on.
    let mut h = ready(Difficulty::Easy);
    assert_eq!(rod_struck(&mut h, "minecraft:stone", [3.5, 65.0, 3.5]), 1);
    watch(&mut h, 1);
    assert!(powered(&mut h));
    watch(&mut h, 8);
    assert!(!powered(&mut h));
    assert!(rod_struck(&mut h, "minecraft:oxidized_copper", [3.5, 65.0, 3.5]) >= 1);
    watch(&mut h, 1);
    let copper = ask(&mut h, |ctx| ctx.block([3, 64, 3]).unwrap().name);
    assert_eq!(copper, "minecraft:copper_block", "3 to 5 walks from the rod's support, a stage each");
}

#[test]
fn a_bolt_over_a_rod_powers_it_with_a_spark_for_each_of_its_walks_calls() {
    // research/captures/bds-o3rod.txt, bds-rsdevices.txt: 7, 3, 3, 6 and 6 sparks; none from two over.
    let mut h = ready(Difficulty::Easy);
    let sparks = rod_struck(&mut h, "minecraft:stone", [3.5, 66.0, 3.5]);
    assert!(sparks >= 3, "{sparks}");
    watch(&mut h, 1);
    assert!(powered(&mut h));
    watch(&mut h, 10);
    assert_eq!(rod_struck(&mut h, "minecraft:stone", [3.5, 67.0, 3.5]), 0);
    watch(&mut h, 1);
    assert!(!powered(&mut h));
}
