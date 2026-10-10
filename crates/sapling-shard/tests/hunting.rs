//! A zombie hunting a player, against research/captures/bds-bite.txt and bds-slain.txt.

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, DeathInfo, EntityEvent, EntityEventEventId, LevelSoundEvent, MoveEntityDelta, SetEntityData, SetEntityMotion, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Block, DamageCause, Difficulty, EntityId, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);

/// Alex in survival at (0.5, 65, 0.5), Steve watching in creative, and a zombie 4 blocks north of
/// Alex on `difficulty`; the zombie's id.
fn hunted(difficulty: Difficulty) -> (Harness, u64) {
    hunted_by("minecraft:zombie", difficulty)
}

fn hunted_by(kind: &'static str, difficulty: Difficulty) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        assert!(ctx.spawn_entity(kind, [0.5, 65.0, 4.5]).is_some());
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

#[test]
fn a_zombie_jumps_a_block_in_its_way_and_does_not_see_through_a_wall() {
    let (mut h, zombie) = hunted(Difficulty::Easy);
    h.remote.run(|ctx| (-8..8).for_each(|x| assert!(ctx.set_block([x, 65, 2], &Block::new("stone")))));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..80 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entity(EntityId(zombie)).map(|found| found.feet[1])));
        h.tick_all();
    }
    let heights = seen.lock().unwrap().clone();
    let top = heights.iter().copied().fold(0.0, f32::max);
    // A jump of 0.42 a tick less gravity rises 1.25.
    assert!((top - 66.2522).abs() < 1e-3 && heights.contains(&66.0), "over the block: {top}");

    let (mut h, _) = hunted(Difficulty::Easy);
    h.remote.run(|ctx| (-8..8).for_each(|x| assert!(ctx.set_block([x, 65, 2], &Block::new("stone")) && ctx.set_block([x, 66, 2], &Block::new("stone")))));
    let sent: Vec<RawPacket> = (0..40).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert!(targets(&sent).is_empty());
}

#[test]
fn each_kind_hunts_at_its_speed_with_its_hit_and_what_the_hit_gives() {
    use acacia_proto::packets::MobEffect;
    // research/captures/bds-kinds.txt, on easy: a husk's 3 and its hunger, a spider's 2 from
    // further off, a vindicator's 8 and its axe, a wither skeleton's 4, its sword and its wither, a silverfish's whole 1.
    let cases = [("minecraft:husk", 17.5, 0.114_189, vec![(17, 600)]), ("minecraft:spider", 18.0, 0.194_273, vec![]), ("minecraft:vindicator", 12.5, 0.264_427, vec![]), ("minecraft:wither_skeleton", 14.5, 0.210_799, vec![(20, 200)]), ("minecraft:silverfish", 19.0, 0.134_912, vec![])];
    for (kind, left, speed, given) in cases {
        let (mut h, mob) = hunted_by(kind, Difficulty::Easy);
        let (_, sent) = until_hit(&mut h, 60);
        let effects: Vec<(i32, i32)> = all::<MobEffect>(&sent).into_iter().map(|effect| (effect.effect_id, effect.duration)).collect();
        assert_eq!((health(&sent), effects), (vec![left], given), "{kind}");
        h.remote.run(move |ctx| {
            let found = ctx.entity(EntityId(mob)).expect("still there");
            // The quick ones are on Alex before they are quite at speed.
            assert!((found.velocity[2] + speed).abs() < 1e-3, "{kind}: {found:?}");
        });
        h.tick_all();
    }
}

#[test]
fn a_cow_left_alone_strolls_at_its_pace_and_stays_on_the_floor() {
    // research/captures/bds-stroll.txt: a cow went 0.08634 a tick, up to ten blocks at a time.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5]).is_some()));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..1200 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entities().first().map(|cow| (cow.feet, cow.velocity))));
        h.tick_all();
    }
    let seen = seen.lock().unwrap().clone();
    let fastest = seen.iter().map(|(_, velocity)| velocity[0].hypot(velocity[2])).fold(0.0, f32::max);
    let furthest = seen.iter().map(|(feet, _)| (feet[0] - 0.5).hypot(feet[2] - 0.5)).fold(0.0, f32::max);
    assert!((fastest - 0.086_344).abs() < 1e-4 && furthest > 2.0, "{fastest} {furthest}");
    assert!(seen.iter().all(|(feet, _)| feet[1] == 65.0), "it never left the floor");
}
