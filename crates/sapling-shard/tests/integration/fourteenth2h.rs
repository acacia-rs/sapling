//! Group H of the fourteenth wave: `/damage` with a source (research/captures/bds-h2dmgp.txt,
//! bds-h2dmgm.txt, bds-b1shield.txt; vitals/struck.rs `sourced`, creatures/sourced.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent, SetEntityData, SetEntityMotion};
use acacia_proto::types::{InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::shooting::{FEET, ready, spawn};
use common::{Harness, all, input_at};
use sapling_api::{DamageCause, Difficulty, EntityId, GameMode, Item, PlayerId, ShardCtx, Slot, Target};

const ALEX: PlayerId = PlayerId(1);

/// What `call` returned on the shard's next tick, and what that tick sent Alex.
fn told<T: Send + 'static>(h: &mut Harness, call: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Vec<RawPacket>) {
    let answer = std::sync::Arc::new(std::sync::Mutex::new(None));
    let put = answer.clone();
    h.remote.run(move |ctx| *put.lock().unwrap() = Some(call(ctx)));
    let sent = h.tick_all().to(1).to_vec();
    (answer.lock().unwrap().take().expect("the shard ran the call"), sent)
}

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// Alex's throws, each `[x, y, z]`, and whether the first came before his flash.
fn thrown(sent: &[RawPacket]) -> (Vec<[f32; 3]>, bool) {
    let throws: Vec<[f32; 3]> = all::<SetEntityMotion>(sent).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]).collect();
    let first = |wanted: fn(&RawPacket) -> bool| sent.iter().position(wanted);
    let motion = first(|packet| !all::<SetEntityMotion>(std::slice::from_ref(packet)).is_empty());
    let flash = first(|packet| all::<EntityEvent>(std::slice::from_ref(packet)).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation));
    (throws, motion.is_some() && motion < flash)
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-3)
}

#[test]
fn damage_from_a_source_throws_a_player_away_from_it_for_any_cause_before_the_hurt() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    let stand = spawn(&mut h, "minecraft:armor_stand", [FEET[0] + 2.0, FEET[1], FEET[2] + 1.0]);
    for cause in [DamageCause::Attack, DamageCause::Magic, DamageCause::Fall] {
        let (hurt, sent) = told(&mut h, move |ctx| ctx.hurt_player_from(ALEX, 2.0, cause, &Target::Entity(EntityId(stand))));
        let (throws, first) = thrown(&sent);
        assert!(hurt && first, "{cause:?} lands, thrown first: {throws:?}");
        assert!(near(throws[0], [-0.357_771, 0.4, -0.178_885]), "{cause:?}: {throws:?} (bds-h2dmgp.txt)");
        ticks(&mut h, 12);
        h.receive(1, &input_at(FEET));
        ticks(&mut h, 2);
    }
    let (_, sent) = told(&mut h, |ctx| {
        let alex = ctx.players().find(|player| player.id == ALEX).expect("Alex");
        ctx.hurt_player_from(ALEX, 2.0, DamageCause::Attack, &Target::Player(alex))
    });
    assert!(near(thrown(&sent).0[0], [0.0, 0.4, 0.0]), "from himself, straight up");
}

#[test]
fn damage_from_a_source_ahead_meets_a_raised_shield_and_one_behind_does_not() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Offhand, Some(&Item::new("minecraft:shield", 1)))));
    let mut input = input_at(FEET);
    input.input_data = vec![InputData::StartSneaking];
    h.receive(1, &input);
    ticks(&mut h, 8);
    let ahead = spawn(&mut h, "minecraft:armor_stand", [FEET[0], FEET[1], FEET[2] + 2.0]);
    let behind = spawn(&mut h, "minecraft:armor_stand", [FEET[0], FEET[1], FEET[2] - 2.0]);
    for cause in [DamageCause::Attack, DamageCause::Projectile, DamageCause::Thorns, DamageCause::Fireworks] {
        let (hurt, sent) = told(&mut h, move |ctx| ctx.hurt_player_from(ALEX, 3.0, cause, &Target::Entity(EntityId(ahead))));
        let blocked = all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "item.shield.block");
        assert!(!hurt && blocked && thrown(&sent).0.is_empty(), "{cause:?} turned away, nobody thrown (bds-b1shield.txt)");
        ticks(&mut h, 3);
    }
    let (hurt, sent) = told(&mut h, move |ctx| ctx.hurt_player_from(ALEX, 3.0, DamageCause::Attack, &Target::Entity(EntityId(behind))));
    assert!(hurt && near(thrown(&sent).0[0], [0.0, 0.4, 0.4]), "from behind it lands and throws ahead");
}

fn targets(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter(|item| item.key == Key::TargetEid).filter_map(|item| if let Value::Default(Plain::Long(target)) = item.value { Some(target) } else { None }).collect()
}

#[test]
fn damage_from_a_player_throws_a_mob_and_provokes_it_and_from_a_mob_sets_it_on_that_mob() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
    });
    let cow = spawn(&mut h, "minecraft:cow", [FEET[0] + 3.0, FEET[1], FEET[2]]);
    let (_, sent) = told(&mut h, move |ctx| {
        let alex = ctx.players().find(|player| player.id == ALEX).expect("Alex");
        assert!(ctx.hurt_entity_from(EntityId(cow), 1.0, DamageCause::Magic, &Target::Player(alex)));
    });
    drop(sent);
    let speed = common::asking::ask(&mut h, move |ctx| ctx.entity(EntityId(cow)).expect("a cow").velocity);
    assert!(speed[0] > 0.1 && speed[1] > 0.0, "thrown away and up: {speed:?}");

    let bystander = spawn(&mut h, "minecraft:zombie_pigman", [FEET[0] - 5.0, FEET[1], FEET[2] + 5.0]);
    let pigman = spawn(&mut h, "minecraft:zombie_pigman", [FEET[0] + 3.0, FEET[1], FEET[2] + 3.0]);
    let (_, mut sent) = told(&mut h, move |ctx| {
        let alex = ctx.players().find(|player| player.id == ALEX).expect("Alex");
        assert!(ctx.hurt_entity_from(EntityId(pigman), 1.0, DamageCause::Attack, &Target::Player(alex)));
    });
    sent.extend(ticks(&mut h, 3));
    assert_eq!(targets(&sent, pigman).first(), Some(&1), "the pigman takes Alex (bds-h2dmgm.txt)");
    assert_eq!(targets(&sent, bystander).first(), Some(&1), "and the pigman near it");

    let zombie = spawn(&mut h, "minecraft:zombie", [FEET[0] - 4.0, FEET[1], FEET[2]]);
    let skeleton = spawn(&mut h, "minecraft:skeleton", [FEET[0] + 3.0, FEET[1], FEET[2] - 3.0]);
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    ticks(&mut h, 2);
    let (_, mut sent) = told(&mut h, move |ctx| assert!(ctx.hurt_entity_from(EntityId(skeleton), 1.0, DamageCause::Attack, &Target::Entity(EntityId(zombie)))));
    sent.extend(ticks(&mut h, 3));
    assert_eq!(targets(&sent, skeleton).first(), Some(&(zombie as i64)), "the skeleton takes the zombie");
}
