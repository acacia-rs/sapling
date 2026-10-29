//! Primed TNT, against research/captures/bds-tnt.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, CommandRequest, LevelEvent, LevelEventEvent, RemoveEntity, SetEntityData, SetEntityMotion};
use acacia_proto::manual::Uuid;
use acacia_proto::types::CommandOrigin;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, click};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

/// The fuse lengths told of the entity `id` (an item's whole metadata has one too).
fn fuses(sent: &[RawPacket], id: u64) -> Vec<i32> {
    all::<SetEntityData>(sent).iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| &data.metadata).filter(|item| item.key == Key::FuseLength).map(|item| if let Value::Default(Plain::Int(left)) = item.value { left } else { -1 }).collect()
}

#[test]
fn tnt_by_a_block_of_redstone_hops_burns_eighty_ticks_and_bursts() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.set_block([3, 65, 0], &Block::new("minecraft:tnt"));
        ctx.set_block([3, 65, 1], &Block::new("minecraft:redstone_block"));
    });
    let mut sent = Vec::new();
    let lit = (1..=10).find(|_| {
        sent.extend(h.tick_all().to(1).to_vec());
        !all::<AddEntity>(&sent).is_empty()
    });
    assert!(lit.is_some(), "not lit");
    let tnt = all::<AddEntity>(&sent).remove(0);
    let speed = (tnt.velocity.x.powi(2) + tnt.velocity.z.powi(2)).sqrt();
    assert!(tnt.entity_type == "minecraft:tnt" && (tnt.position.x, tnt.position.y, tnt.position.z) == (3.5, 65.49, 0.5) && tnt.velocity.y == 0.2 && (speed - 0.02).abs() < 1e-4, "{:?} {:?}", tnt.position, tnt.velocity);
    assert!(all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::SoundIgnite && (event.position.x, event.position.y, event.position.z) == (3.5, 65.0, 0.5)));

    let burst = |sent: &[RawPacket]| all::<LevelEvent>(sent).into_iter().find(|event| event.event == LevelEventEvent::ParticleExplosion);
    let ticks = (1..=90).find(|_| {
        sent.extend(h.tick_all().to(1).to_vec());
        burst(&sent).is_some()
    });
    assert_eq!(ticks, Some(80));
    let fuses = fuses(&sent, tnt.runtime_id);
    assert_eq!((fuses.len(), fuses.first(), fuses.last()), (80, Some(&79), Some(&0)));
    let flash = burst(&sent).expect("found above");
    assert!(flash.data == 4 && (flash.position.y - 65.06125).abs() < 1e-3, "{flash:?}");
    sent.extend(h.tick_all().to(1).to_vec());
    assert!(all::<RemoveEntity>(&sent).iter().any(|gone| gone.entity_id_self == tnt.unique_id));
    // A creative player is thrown away from it: BDS's, 2.9 off, at 0.56 across and 0.22 up.
    let thrown: Vec<_> = all::<SetEntityMotion>(&sent).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| motion.velocity).collect();
    assert!(thrown.len() == 1 && thrown[0].x < -0.4 && thrown[0].y > 0.1, "{thrown:?}");
    let left: Vec<u16> = all::<AddItemEntity>(&sent).iter().map(|item| item.item.count as u16).collect();
    assert!(left.iter().any(|count| *count >= 6) && left.len() <= 2, "the floor's stone in one stack, and the block of redstone: {left:?}");
    h.remote.run(|ctx| {
        let broken = (0..=6).flat_map(|x| (-3..=3).map(move |z| [x, 64, z])).filter(|at| ctx.block(*at).is_some_and(|block| block.name == "minecraft:air")).count();
        assert!((6..=14).contains(&broken), "BDS's broke ten of the floor: {broken}");
        assert!(ctx.block([3, 65, 1]).is_some_and(|block| block.name == "minecraft:air"));
    });
    h.tick_all();
}

#[test]
fn a_blast_lights_the_tnt_it_reaches_and_a_summoned_one_is_primed() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_block([6, 65, 0], &Block::new("minecraft:tnt"));
        assert!(ctx.spawn_entity("minecraft:tnt", [4.5, 65.0, 0.5]).is_some());
    });
    let sent: Vec<RawPacket> = (0..125).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let primed = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:tnt").count();
    let bursts = all::<LevelEvent>(&sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleExplosion).count();
    assert_eq!((primed, bursts), (2, 2));
}

#[test]
fn tnt_summoned_under_water_makes_no_splash() {
    // research/captures/bds-bkblocks.txt: none at its first tick in the water, nor after.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        (64..=67).for_each(|y| assert!(ctx.set_block([4, y, 0], &Block::new("minecraft:water"))));
        assert!(ctx.spawn_entity("minecraft:tnt", [4.5, 65.0, 0.5]).is_some());
    });
    let sent: Vec<RawPacket> = (0..20).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<acacia_proto::packets::LevelSoundEvent>(&sent).iter().all(|sound| sound.sound_id != "splash"));
}

#[test]
fn a_blast_throws_a_pig_in_the_open_off_and_up() {
    // research/captures/bds-blasts.txt: TNT 3 west of a pig threw it at (0.88, 0.48, -0.14); the
    // primed TNT hops, so how far it is here is not pinned.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let pig = common::asking::ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.spawn_entity("minecraft:pig", [5.5, 65.0, 0.5])
    })
    .expect("a pig");
    // As the drill's pigs: resistance V takes the whole hit, which still throws.
    let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid::default(), request_id: String::new(), player_entity_id: 0 };
    h.receive(1, &CommandRequest { command: "/effect @e[type=pig] resistance 100 4 true".into(), origin, internal: false, version: "latest".into() });
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:tnt", [2.5, 65.0, 0.5]).is_some()));
    let sent: Vec<RawPacket> = (0..85).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let thrown: Vec<_> = all::<SetEntityMotion>(&sent).into_iter().filter(|motion| motion.runtime_entity_id == pig.0).map(|motion| motion.velocity).collect();
    let first = thrown.first().expect("thrown");
    // The hit's own 0.4 away and up, and the blast's push on top.
    assert!(first.x > 0.4 && first.y > 0.4 && first.z.abs() < 0.1, "{thrown:?}");
}

#[test]
fn a_charged_respawn_anchor_bursts_when_clicked_in_the_overworld() {
    // research/captures/bds-clicks2.txt: a blast of power 5 at its middle.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_block([3, 65, 0], &Block { name: "minecraft:respawn_anchor".into(), properties: "respawn_anchor_charge=1".into() }));
    });
    h.tick_all();
    h.receive(1, &click([3, 65, 0], 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let flash = all::<LevelEvent>(&sent).into_iter().find(|event| event.event == LevelEventEvent::ParticleExplosion).expect("a blast");
    assert_eq!((flash.data, flash.position.x, flash.position.y, flash.position.z), (5, 3.5, 65.5, 0.5));
    h.remote.run(|ctx| assert!(ctx.block([3, 65, 0]).is_some_and(|block| block.name == "minecraft:air") && ctx.block([3, 64, 0]).is_some_and(|block| block.name == "minecraft:air")));
    h.tick_all();
}

#[test]
fn a_fire_charge_from_a_dispenser_sets_fire_before_the_tnt_it_hits() {
    // research/captures/bds-bkticks.txt: fire in the cell west of the TNT, and no TNT primed.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("doFireTick", sapling_api::RuleValue::Bool(false)));
        assert!(ctx.update_block([3, 65, 3], &Block::new("dispenser").with("facing_direction=5")));
        assert!(ctx.set_container_item([3, 65, 3], 0, Some(&sapling_api::Item::new("minecraft:fire_charge", 1))));
        assert!(ctx.update_block([6, 65, 3], &Block::new("minecraft:tnt")));
    });
    (0..4).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| assert!(ctx.update_block([3, 65, 4], &Block::new("redstone_block"))));
    let sent: Vec<RawPacket> = (0..30).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<AddEntity>(&sent).iter().all(|added| added.entity_type != "minecraft:tnt"), "primed");
    assert_eq!(common::asking::ask(&mut h, |ctx| ctx.block([5, 65, 3]).map(|block| block.name)).as_deref(), Some("minecraft:fire"));
}
