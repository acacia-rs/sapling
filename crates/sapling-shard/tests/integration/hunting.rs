//! A zombie hunting a player, against research/captures/bds-bite.txt and bds-slain.txt.

use crate::common;

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
    // Off Alex's spot, so as not to push what comes to it twice over.
    h.receive(2, &common::input_at([0.5, 65.0, -6.5]));
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        // By night: the sun would burn most of them.
        ctx.set_time(18000);
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
    // bds-slain.txt: 30 degrees from a stand, then 60 while its legs come up to speed (235, 150, 127).
    assert_eq!(turns[..4], [235, 192, 150, 127], "round to the south");
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

/// bds-ah3stop2.txt: a husk walks at full pace (0.114 a tick) until a block off its still prey, then
/// eases in: each step the last one's drag-kept 0.546 plus the full push (0.0518) times its speed's
/// share squared, the share the squared distance off, at least half (BDS's 0.0967, 0.0746, 0.0556
/// from 0.90 off), and stands 0.56 off, unpushed by its prey.
#[test]
fn a_hunter_eases_in_over_its_last_block_to_its_prey() {
    let (mut h, husk) = hunted_by("minecraft:husk", Difficulty::Easy);
    let mut zs = Vec::new();
    for _ in 0..80 {
        let sent = h.tick_all().to(2).to_vec();
        zs.extend(all::<MoveEntityDelta>(&sent).iter().filter(|moved| moved.runtime_entity_id == husk).filter_map(|moved| moved.z));
    }
    let steps: Vec<f32> = zs.windows(2).map(|pair| pair[0] - pair[1]).collect();
    let full = steps.iter().position(|step| *step > 0.11).expect("full pace");
    let eased = full + steps[full..].iter().position(|step| *step < 0.11).expect("an ease");
    let model = |i: usize| steps[i - 1] * 0.546 + 0.0518 * ((zs[i] - 0.5).powi(2)).max(0.5).powi(2);
    assert!((eased..eased + 4).all(|i| (steps[i] - model(i)).abs() < 0.002), "{:?} from {:?}", &steps[eased..eased + 4], &zs[eased.saturating_sub(2)..]);
    let rest = zs.iter().copied().fold(f32::MAX, f32::min) - 0.5;
    assert!((0.5..0.58).contains(&rest), "stood {rest} off: {zs:?}");
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

    for wall in ["stone", "glass"] {
        let (mut h, _) = hunted(Difficulty::Easy);
        h.remote.run(move |ctx| (-8..8).for_each(|x| assert!(ctx.set_block([x, 65, 2], &Block::new(wall)) && ctx.set_block([x, 66, 2], &Block::new(wall)))));
        let sent: Vec<RawPacket> = (0..40).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
        assert!(targets(&sent).is_empty(), "{wall} hides a player");
    }
}

#[test]
fn each_kind_hunts_at_its_speed_with_its_hit_and_what_the_hit_gives() {
    use acacia_proto::packets::MobEffect;
    // research/captures/bds-kinds.txt, on easy: a husk's 3 and its hunger, a spider's 2 from
    // further off, a vindicator's 8 and its axe, a wither skeleton's 4, its sword and its wither, a silverfish's whole 1.
    let cases = [("minecraft:husk", 17.5, 0.114_189, vec![(17, 600)]), ("minecraft:spider", 18.0, 0.194_273, vec![]), ("minecraft:vindicator", 12.5, 0.264_427, vec![]), ("minecraft:wither_skeleton", 14.5, 0.210_799, vec![(20, 200), (20, 200)]), ("minecraft:silverfish", 19.0, 0.134_912, vec![])];
    for (kind, left, speed, given) in cases {
        let (mut h, mob) = hunted_by(kind, Difficulty::Easy);
        // A spider looks for prey one tick in 100 (research/captures/bds-targets2.txt).
        let (_, sent) = until_hit(&mut h, if kind == "minecraft:spider" { 600 } else { 60 });
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
fn a_cave_spider_s_bite_poisons_for_seven_seconds_none_on_easy_and_fifteen_on_hard() {
    use acacia_proto::packets::MobEffect;
    // research/captures/bds-e1cave.txt, on normal: poison 140 at each bite; bds-t12cave.txt and
    // bds-t12bites.txt: 0 on easy, 300 on hard.
    for (difficulty, ticks) in [(Difficulty::Normal, 140), (Difficulty::Easy, 0), (Difficulty::Hard, 300)] {
        let (mut h, _) = hunted_by("minecraft:cave_spider", difficulty);
        let (_, sent) = until_hit(&mut h, 600);
        let effects: Vec<(i32, i32)> = all::<MobEffect>(&sent).into_iter().map(|effect| (effect.effect_id, effect.duration)).take(1).collect();
        assert_eq!(effects, [(19, ticks)], "{difficulty:?}");
    }
}

#[test]
fn a_wither_skeleton_s_hit_gives_wither_twice_in_its_tick() {
    use acacia_proto::packets::MobEffect;
    // research/captures/bds-t12bites.txt and bds-kinds.txt: `Add` then `Update`, both 200.
    let (mut h, _) = hunted_by("minecraft:wither_skeleton", Difficulty::Normal);
    let (_, sent) = until_hit(&mut h, 600);
    let effects: Vec<String> = all::<MobEffect>(&sent).into_iter().filter(|effect| effect.effect_id == 20).map(|effect| format!("{:?} {}", effect.event_id, effect.duration)).collect();
    assert_eq!(effects, ["Add 200", "Update 200"]);
}

#[test]
fn a_cow_left_alone_strolls_at_its_pace_and_stays_on_the_floor() {
    // research/captures/bds-stroll.txt: a cow went 0.08634 a tick, up to ten blocks at a time.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    // A spectator, so as not to push it.
    h.remote.run(|ctx| {
        let _ = ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Spectator);
        assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5]).is_some());
    });
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

#[test]
fn cows_in_the_corners_of_a_small_floor_still_set_off_about_a_tick_in_120() {
    // research/captures/bds-strollstart.txt: on a walled floor 25 across BDS's cows set off once in
    // 114 ticks stood still; a single try at a place from a corner mostly lands off the floor.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        let _ = ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Spectator);
        for at in [[-7.0, 65.0, -7.0], [7.0, 65.0, -7.0], [-7.0, 65.0, 7.0], [7.0, 65.0, 7.0]] {
            assert!(ctx.spawn_entity("minecraft:cow", at).is_some());
        }
    });
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..2400 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().push(ctx.entities().iter().map(|cow| (cow.id, cow.feet)).collect::<Vec<_>>()));
        h.tick_all();
    }
    let seen = seen.lock().unwrap().clone();
    let walking = |tick: usize, cow: usize| seen[tick][cow].1[0] != seen[tick - 1][cow].1[0] || seen[tick][cow].1[2] != seen[tick - 1][cow].1[2];
    let (mut starts, mut idle) = (0, 0);
    for cow in 0..4 {
        for tick in 2..seen.len() {
            match (walking(tick - 1, cow), walking(tick, cow)) {
                (false, true) => starts += 1,
                (false, false) => idle += 1,
                _ => {}
            }
        }
    }
    let every = idle / starts.max(1);
    assert!((80..170).contains(&every), "{starts} starts in {idle} idle ticks");
}

#[test]
fn a_cow_a_player_hit_runs_off_faster_than_it_strolls() {
    use acacia_proto::packets::InventoryTransaction;
    use acacia_proto::types::{ItemV4, ItemV4Extra, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionDataItemUseOnEntity, TransactionTransactionDataItemUseOnEntityActionType as Action, TransactionTransactionType, Vec3f};
    // research/captures/bds-flee.txt: a cow went up to 0.2108 a tick for some three seconds.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 2.0]).is_some()));
    let cow = (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect::<Vec<_>>().remove(0).runtime_id;
    let held_item = ItemV4 { network_id: 0, count: 0, metadata: 0, has_stack_id: false, stack_id: None, block_runtime_id: 0, extra: ItemV4Extra::Default(None) };
    let on = TransactionTransactionDataItemUseOnEntity { entity_runtime_id: cow, action_type: Action::Attack, hotbar_slot: 0, held_item, player_pos: Vec3f { x: 0.5, y: 66.62, z: 0.5 }, click_pos: Vec3f { x: 0.0, y: 0.0, z: 0.0 } };
    let transaction = Transaction { legacy: TransactionLegacy { legacy_request_id: 0, legacy_transactions: None }, transaction_type: TransactionTransactionType::ItemUseOnEntity, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemUseOnEntity(on) };
    h.receive(1, &InventoryTransaction { transaction });
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..70 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entities().first().filter(|cow| cow.on_ground).map(|cow| cow.velocity[0].hypot(cow.velocity[2]))));
        h.tick_all();
    }
    let fastest = seen.lock().unwrap().iter().skip(15).copied().fold(0.0, f32::max);
    assert!((0.15..0.2109).contains(&fastest), "it ran: {fastest}");
}

#[test]
fn a_creeper_lights_its_fuse_three_blocks_off_and_bursts_as_on_bds() {
    use acacia_proto::packets::{LevelEvent, LevelEventEvent, RemoveEntity};
    // research/captures/bds-boom.txt: lit 3.08 off, it burst 2.92 off for 8.229 on easy and broke
    // the stone under it.
    let (mut h, creeper) = hunted_by("minecraft:creeper", Difficulty::Easy);
    let (ticks, sent) = until_hit(&mut h, 80);
    // The blast's loot comes with its whole metadata, FuseLength -1 among it.
    let fuses: Vec<i32> = all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == creeper).flat_map(|data| &data.metadata).filter(|item| item.key == Key::FuseLength).map(|item| if let Value::Default(Plain::Int(left)) = item.value { left } else { -1 }).collect();
    assert_eq!((fuses.len(), fuses.first(), fuses.last()), (30, Some(&29), Some(&0)), "lit at tick {}", ticks - 31);
    assert!(all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion && event.data == 3) && all::<RemoveEntity>(&sent).len() == 1);
    let hurt = 20.0 - health(&sent)[0];
    assert!((7.9..8.6).contains(&hurt), "{hurt}");
    let thrown: Vec<_> = all::<SetEntityMotion>(&sent).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| motion.velocity).collect();
    // bds-boom.txt: the knock alone (0.4 up), then the knock and the blast's push together.
    assert!(thrown.len() == 2 && thrown[0].y == 0.4 && thrown[1].y > 0.55 && thrown[1].z < -0.7, "left: the hit's throw, then with the blast's: {thrown:?}");
    h.remote.run(move |ctx| {
        assert!(ctx.entity(EntityId(creeper)).is_none());
        let broken: Vec<[i32; 3]> = (-3..=3).flat_map(|x| (-1..=6).map(move |z| [x, 64, z])).filter(|at| ctx.block(*at).is_some_and(|block| block.name == "minecraft:air")).collect();
        assert_eq!(broken.len(), 1, "the one stone under it: {broken:?}");
    });
    h.tick_all();
}

#[test]
fn a_skeleton_stands_off_and_looses_an_arrow_every_three_seconds() {
    // research/captures/bds-shoot.txt: an arrow a block before it and 1.61 up, at 1.39 to 1.79 a
    // tick (the spread is thrown on the aim made one long).
    let (mut h, _) = hunted_by("minecraft:skeleton", Difficulty::Easy);
    let sent: Vec<RawPacket> = (0..140).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let arrows: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:arrow").collect();
    assert_eq!(arrows.len(), 2, "one after 61 ticks, one 61 later");
    let (at, speed) = (&arrows[0].position, &arrows[0].velocity);
    let fast = (speed.x * speed.x + speed.y * speed.y + speed.z * speed.z).sqrt();
    // It walked some 1.7 blocks nearer first, as BDS's did.
    assert!((1.4..2.2).contains(&at.z) && (at.y - 66.61).abs() < 1e-3 && (1.3..1.9).contains(&fast) && speed.z < -1.2, "{at:?} {speed:?}");
    assert_eq!(all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "bow").count(), 2);
    let hurt: Vec<f32> = health(&sent).windows(2).map(|pair| pair[0] - pair[1]).collect();
    assert!(health(&sent).len() <= 2 && hurt.iter().all(|by| (3.0..=4.0).contains(by)), "an arrow costs its speed twice over, rounded up: {:?}", health(&sent));
}

#[test]
fn a_skeletons_arrow_is_announced_moves_in_its_loosing_tick_and_goes_the_tick_after_its_hit() {
    // research/captures/bds-shoot.txt: AddEntity, then the first move or the hit in the same tick;
    // the arrow that hit was removed the tick after.
    let (mut h, _) = hunted_by("minecraft:skeleton", Difficulty::Easy);
    let ticks: Vec<Vec<RawPacket>> = (0..140).map(|_| h.tick_all().to(2).to_vec()).collect();
    let found = ticks.iter().enumerate().find_map(|(at, sent)| all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == "minecraft:arrow").map(|added| (at, added.runtime_id)));
    let (born, arrow) = found.expect("an arrow");
    let hurt = |sent: &[RawPacket]| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation);
    let hurt_at = ticks.iter().position(|sent| hurt(sent)).expect("Alex hit");
    let moved = all::<MoveEntityDelta>(&ticks[born]).iter().any(|moved| moved.runtime_entity_id == arrow);
    assert!(moved || hurt_at == born, "born {born}, hit {hurt_at}");
    let gone_at = ticks.iter().position(|sent| all::<acacia_proto::packets::RemoveEntity>(sent).iter().any(|gone| gone.entity_id_self == arrow as i64));
    assert_eq!(gone_at, Some(hurt_at + 1));
}

#[test]
fn an_arrow_hurts_a_mob_that_stands_in_its_way() {
    // The skeleton shoots level at Alex's eyes through a llama a block before it, as BDS's hit a
    // villager in the way (bds-e1arrow.txt); a cow is too low to meet the arrow.
    for kind in ["minecraft:llama", "minecraft:villager_v2"] {
        let (mut h, _) = hunted_by("minecraft:skeleton", Difficulty::Easy);
        h.remote.run(move |ctx| assert!(ctx.spawn_entity(kind, [0.5, 65.0, 1.5]).is_some()));
        let sent: Vec<RawPacket> = (0..75).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
        let struck: Vec<f32> = all::<UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id > 3).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect();
        assert!(health(&sent).is_empty() && !struck.is_empty(), "the {kind} took it: {struck:?}");
    }
}

#[test]
fn a_zombie_goes_round_a_fence_it_cannot_jump() {
    // research/captures/bds-around.txt: BDS's went to the cell past the fence's end and on.
    let (mut h, zombie) = hunted(Difficulty::Easy);
    h.remote.run(|ctx| (-2..=2).for_each(|x| assert!(ctx.set_block([x, 65, 2], &Block::new("oak_fence")))));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut hit = None;
    for tick in 0..160 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entity(EntityId(zombie)).map(|found| found.feet)));
        if !health(h.tick_all().to(2)).is_empty() && hit.is_none() {
            hit = Some(tick);
        }
    }
    let widest = seen.lock().unwrap().iter().map(|feet| feet[0].abs()).fold(0.0, f32::max);
    assert!(hit.is_some() && (2.8..4.5).contains(&widest), "round the end, three cells out: {hit:?} {widest}");
}

#[test]
fn a_strays_arrow_slows_a_boggeds_poisons_and_a_pillager_shoots_on_the_walk() {
    // research/captures/bds-shoot2.txt: slowness for 600 ticks, poison for 112; a shot each 46 ticks.
    for (kind, effect) in [("minecraft:stray", (2, 600)), ("minecraft:bogged", (19, 112))] {
        let (mut h, _) = hunted_by(kind, Difficulty::Easy);
        let sent: Vec<RawPacket> = (0..110).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
        let given: Vec<(i32, i32)> = all::<acacia_proto::packets::MobEffect>(&sent).iter().map(|told| (told.effect_id, told.duration)).collect();
        assert!(given.contains(&effect), "{kind}: {given:?}");
    }
    let (mut h, pillager) = hunted_by("minecraft:pillager", Difficulty::Easy);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (mut arrows, mut heard) = (0, Vec::new());
    for _ in 0..100 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entity(EntityId(pillager)).map(|found| found.feet[2])));
        let sent = h.tick_all().to(2).to_vec();
        arrows += all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:arrow").count();
        heard.extend(all::<LevelSoundEvent>(&sent).into_iter().filter_map(|sound| sound.sound_id.strip_prefix("crossbow.loading.").map(str::to_owned)));
    }
    assert_eq!(heard[..3], ["start", "middle", "end"], "its crossbow is heard loading");
    let nearest = seen.lock().unwrap().iter().copied().fold(f32::MAX, f32::min);
    assert!(arrows == 2 && nearest < 2.0, "two shots, and it came near: {arrows} {nearest}");
}

#[test]
fn a_pillager_swaps_to_the_nearer_prey_the_tick_it_is_there() {
    // research/captures/bds-raidaim.txt, bds-pillswitch.txt: held 10 from a villager it took the
    // villager, moved 12 from the actor (15.5 from the villager) all four took the actor in the same tick.
    let (mut h, pillager) = hunted_by("minecraft:pillager", Difficulty::Easy);
    let villager = common::asking::ask(&mut h, move |ctx| {
        let villager = ctx.spawn_entity("minecraft:villager_v2", [0.5, 65.0, 7.5]).expect("spawned");
        for held in [villager, EntityId(pillager)] {
            assert!(ctx.give_entity_effect(held, sapling_api::Effect::Slowness, 1000, 255));
        }
        villager.0 as i64
    });
    let took = |h: &mut Harness, ticks| targets(&(0..ticks).flat_map(|_| h.tick_all().to(2).to_vec()).collect::<Vec<_>>()).last().copied();
    assert_eq!(took(&mut h, 25), Some(villager), "3 off, nearer than Alex's 4");
    h.remote.run(move |ctx| assert!(ctx.move_entity(EntityId(pillager), [0.5, 65.0, 2.5])));
    assert_eq!(took(&mut h, 2), Some(1), "Alex 2 off, the villager 5: not waiting for a look");
}

#[test]
fn a_pillagers_bolt_throws_its_prey_away_from_the_pillager() {
    // research/captures/bds-pillswitch2.txt: shot by pillagers 3 west and 1 north, the actor was
    // thrown 0.4 along (3, 1) and 0.4 up, whatever line the bolt flew.
    let (mut h, pillager) = hunted_by("minecraft:pillager", Difficulty::Easy);
    h.remote.run(move |ctx| {
        assert!(ctx.give_entity_effect(EntityId(pillager), sapling_api::Effect::Slowness, 1000, 255));
        assert!(ctx.move_entity(EntityId(pillager), [2.5, 65.0, 4.5]));
    });
    let sent: Vec<RawPacket> = (0..160)
        .flat_map(|_| {
            h.receive(1, &common::input_at([0.5, 65.0, 0.5]));
            h.tick_all().to(1).to_vec()
        })
        .collect();
    let thrown: Vec<_> = all::<SetEntityMotion>(&sent).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| motion.velocity).collect();
    let along = [-2.0 / 20f32.sqrt() * 0.4, -4.0 / 20f32.sqrt() * 0.4];
    assert!(!thrown.is_empty() && thrown.iter().all(|v| (v.x - along[0]).abs() < 1e-3 && (v.z - along[1]).abs() < 1e-3 && v.y == 0.4), "{thrown:?}");
}

#[test]
fn a_held_pillager_loads_from_afar_and_shoots_only_from_8_off() {
    // research/captures/bds-pillrange.txt: held 12 to 9 off it loaded and never shot; 8 and 7 off it
    // shot every 46 ticks.
    let (mut h, pillager) = hunted_by("minecraft:pillager", Difficulty::Easy);
    // The platform ends at z 7.
    h.remote.run(|ctx| (8..=10).for_each(|z| assert!(ctx.set_block([0, 64, z], &Block::new("stone")))));
    let shots = |h: &mut Harness, z: f32| {
        h.remote.run(move |ctx| {
            assert!(ctx.give_entity_effect(EntityId(pillager), sapling_api::Effect::Slowness, 1000, 255));
            assert!(ctx.move_entity(EntityId(pillager), [0.5, 65.0, z]));
        });
        // Alex is put back each tick against the arrows' knockback, as the drill's actor each second.
        let sent: Vec<RawPacket> = (0..120)
            .flat_map(|_| {
                h.receive(1, &common::input_at([0.5, 65.0, 0.5]));
                h.tick_all().to(2).to_vec()
            })
            .collect();
        let loads = all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "crossbow.loading.start").count();
        (all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:arrow").count(), loads)
    };
    let (far, loads) = shots(&mut h, 9.5);
    assert_eq!((far, loads.min(1)), (0, 1), "9 off: loaded, no shot");
    let near = shots(&mut h, 8.5);
    let at = common::asking::ask(&mut h, move |ctx| ctx.entity(EntityId(pillager)).map(|found| found.feet));
    assert!(near.0 >= 2, "8 off it shoots: {near:?} {at:?}");
}
