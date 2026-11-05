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

fn sparks_at(sent: &[RawPacket], corner: [f32; 3]) -> usize {
    all::<LevelEvent>(sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleElectricSpark && [event.position.x, event.position.y, event.position.z] == corner).count()
}

#[test]
fn a_struck_rod_is_powered_eight_ticks_and_sparks_each_time_a_walk_comes_by() {
    let mut h = ready(Difficulty::Normal);
    h.remote.run(|ctx| {
        ctx.set_block([3, 64, 3], &Block::new("minecraft:oxidized_copper"));
        ctx.set_block([3, 65, 3], &Block::new("minecraft:lightning_rod").with("facing_direction=1,powered_bit=0"));
        // On the rod's top: the walks start at the rod under the bolt's cell (research/captures/bds-rsdevices.txt).
        ctx.spawn_entity(BOLT, [3.5, 66.0, 3.5]);
    });
    // BDS's 6 and 3 were walks coming by at random (bds-rsdevices.txt, bds-p2copper.txt): at least one a walk.
    assert!(sparks_at(&watch(&mut h, 1), [3.0, 65.0, 3.0]) >= 3, "a spark at the rod's corner for each walk");
    // The rod powered a tick after the sparks, as the bolt landed (bds-rsdevices.txt 69133, 69180).
    watch(&mut h, 1);
    let rod = ask(&mut h, |ctx| ctx.block([3, 65, 3]).unwrap());
    assert!(rod.name == "minecraft:lightning_rod" && rod.properties.contains("powered_bit=1"), "{rod:?}");
    watch(&mut h, 8);
    assert!(ask(&mut h, |ctx| ctx.block([3, 65, 3]).unwrap()).properties.contains("powered_bit=0"));
}

#[test]
fn a_bolt_over_copper_walks_it_clean_a_stage_a_walk_sparking_as_it_appears() {
    // research/captures/bds-p2copper.txt: the bolt's cell is air, the copper under it took a stage a walk.
    let mut h = ready(Difficulty::Normal);
    h.remote.run(|ctx| {
        ctx.set_block([3, 64, 3], &Block::new("minecraft:oxidized_copper"));
        ctx.spawn_entity(BOLT, [3.5, 65.0, 3.5]);
    });
    assert_eq!(sparks_at(&watch(&mut h, 1), [3.0, 64.0, 3.0]), 3, "one a stage, on the tick the bolt shows");
    assert_eq!(ask(&mut h, |ctx| ctx.block([3, 64, 3]).unwrap().name), "minecraft:copper_block");
}
