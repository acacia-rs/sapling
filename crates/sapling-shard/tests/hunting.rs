//! A zombie hunting a player, against research/captures/bds-bite.txt and bds-slain.txt.

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, DeathInfo, EntityEvent, EntityEventEventId, LevelSoundEvent, MoveEntityDelta, SetEntityData, SetEntityMotion, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{DamageCause, Difficulty, EntityId, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);

/// Alex in survival at (0.5, 65, 0.5), Steve watching in creative, and a zombie 4 blocks north of
/// Alex on `difficulty`; the zombie's id.
fn hunted(difficulty: Difficulty) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:zombie", [0.5, 65.0, 4.5]).is_some());
    });
    let zombie = (0..2).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id;
    (h, zombie)
}

fn health(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn targets(sent: &[RawPacket]) -> Vec<i64> {
    let target = |data: SetEntityData| data.metadata.iter().find(|item| item.key == Key::TargetEid).map(|item| item.value.clone());
    all::<SetEntityData>(sent).into_iter().filter_map(target).map(|value| if let Value::Default(Plain::Long(id)) = value { id } else { panic!("{value:?}") }).collect()
}

fn sounds(sent: &[RawPacket]) -> Vec<(String, f32)> {
    all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.entity_type == "minecraft:zombie").map(|sound| (sound.sound_id, sound.position.y)).collect()
}

/// Ticks until Alex's health is told, at most `most`; what Steve was sent on the way.
fn until_hit(h: &mut Harness, most: usize) -> (usize, Vec<RawPacket>) {
    let mut sent = Vec::new();
    for tick in 1..=most {
        sent.extend(h.tick_all().to(2).to_vec());
        if !health(&sent).is_empty() {
            return (tick, sent);
        }
    }
    panic!("not hit in {most} ticks");
}

#[test]
fn a_zombie_turns_to_a_survival_player_walks_up_and_hits_it_once_a_second() {
    let (mut h, zombie) = hunted(Difficulty::Easy);
    let (_, sent) = until_hit(&mut h, 60);
    assert_eq!((targets(&sent), health(&sent)), (vec![1], vec![17.5]), "3 is 2.5 on easy");
    assert_eq!(sounds(&sent), [("mob.warning".to_owned(), 65.0 + 0.85 * 1.9)]);
    let turns: Vec<u8> = all::<MoveEntityDelta>(&sent).into_iter().filter_map(|moved| moved.rot_y).collect();
    assert_eq!(turns[..3], [213, 170, 128], "round to the south");
    let events: Vec<_> = all::<EntityEvent>(&sent).into_iter().map(|event| (event.runtime_entity_id, event.event_id, event.data)).collect();
    assert_eq!(events, [(zombie, EntityEventEventId::ArmSwing, 0), (1, EntityEventEventId::HurtAnimation, 2)]);
    let thrown: Vec<_> = all::<SetEntityMotion>(&sent).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| (motion.velocity.x, motion.velocity.y, motion.velocity.z)).collect();
    // Its turn left it a little off the line.
    assert!(thrown.len() == 1 && thrown[0].0.abs() < 1e-2 && thrown[0].1 == 0.4 && (thrown[0].2 + 0.4).abs() < 1e-4, "away from it and up: {thrown:?}");
    h.remote.run(move |ctx| {
        let found = ctx.entity(EntityId(zombie)).expect("still there");
        // Its box, 0.8 further across, first reached Alex's a tick ago (1.4 between them), at full speed.
        assert!((1.9 - 0.229..1.9 - 0.114).contains(&found.feet[2]) && (found.velocity[2] + 0.114_189).abs() < 1e-4, "{found:?}");
    });
    let (ticks, sent) = until_hit(&mut h, 25);
    assert_eq!((ticks, health(&sent), sounds(&sent)[0].clone()), (20, vec![15.0], ("attack".to_owned(), 65.0 + 0.5 * 1.9)));
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let sent: Vec<RawPacket> = (0..25).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!((targets(&sent), health(&sent)), (vec![0], vec![]), "nobody hunts a player in creative");
}

#[test]
fn a_zombies_last_hit_names_it_and_a_peaceful_world_is_not_hunted() {
    let (mut h, _) = hunted(Difficulty::Normal);
    h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 18.0, DamageCause::Generic)));
    let sent: Vec<RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(all::<DeathInfo>(&sent).into_iter().map(|info| (info.cause, info.messages)).collect::<Vec<_>>(), [("death.attack.mob".to_owned(), vec!["Alex".to_owned(), "%entity.zombie.name".to_owned()])]);
    assert!(all::<SetEntityMotion>(&sent).iter().all(|motion| motion.runtime_entity_id != 1), "the dead are not thrown");
    assert_eq!(targets(&sent), [1, 0]);

    let (mut h, _) = hunted(Difficulty::Peaceful);
    let sent: Vec<RawPacket> = (0..60).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert!(targets(&sent).is_empty() && all::<MoveEntityDelta>(&sent).is_empty());
}
