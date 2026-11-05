//! Fire, magma, rain, dry land and a forced fuse against mobs, as research/captures/bds-burn2.txt,
//! bds-harm2.txt and bds-fights.txt saw them (docs/research/mob-damage.md, mob-attacks.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, UpdateAttributes};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, Weather};

/// A cow walled in at x `x` on `floor`, with `inside` in its cell; Alex and Steve far enough off.
fn penned(h: &mut Harness, x: i32, floor: &'static str, inside: Block, kind: &'static str) {
    h.remote.run(move |ctx| {
        for y in 64..=67 {
            for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
                assert!(ctx.set_block([x + dx, y, dz], &Block::new("stone")));
            }
        }
        assert!(ctx.set_block([x, 64, 0], &Block::new(floor)) && ctx.set_block([x, 65, 0], &inside));
        assert!(ctx.spawn_entity(kind, [x as f32 + 0.5, 65.0, 0.5]).is_some());
    });
}

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.receive(1, &common::input_at([0.5, 65.0, 12.5]));
    h
}

/// Ticks `n` times; what Alex was sent each tick.
fn ticks(h: &mut Harness, n: usize) -> Vec<Vec<RawPacket>> {
    (0..n).map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// The health last told of the mob shown first in `sent`, and the causes it was hurt by.
fn told(sent: &[Vec<RawPacket>]) -> (f32, Vec<i32>) {
    let sent = sent.concat();
    let id = all::<AddEntity>(&sent).first().expect("a mob").runtime_id;
    let health = all::<UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == id).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).last().unwrap_or(-1.0);
    let causes = all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == id && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
    (health, causes)
}

#[test]
fn soul_fire_costs_a_cow_2_a_hit_campfires_and_magma_1() {
    let cases = [
        ("soul_soil", Block::new("soul_fire"), (4.0, vec![6, 6, 6])),
        ("stone", Block::new("soul_campfire"), (4.0, vec![6, 6, 6])),
        ("stone", Block::new("campfire"), (7.0, vec![6, 6, 6])),
        ("magma", Block::air(), (7.0, vec![22, 22, 22])),
    ];
    for (floor, inside, want) in cases {
        let mut h = ready();
        penned(&mut h, 6, floor, inside, "minecraft:cow");
        let sent = ticks(&mut h, 25);
        assert_eq!(told(&sent), want, "{floor}");
    }
}

#[test]
fn rain_puts_out_a_burning_cow() {
    let mut h = ready();
    penned(&mut h, 6, "netherrack", Block::new("fire"), "minecraft:cow");
    let mut sent = ticks(&mut h, 4);
    h.remote.run(|ctx| {
        assert!(ctx.set_block([6, 65, 0], &Block::air()));
        ctx.set_weather(Weather::Rain, None);
    });
    let rain = ticks(&mut h, 60);
    // research/captures/bds-rainvoid.txt: steam as water's when the rain put it out, at a burn's turn
    // once the rain had set in (20 ticks on); a burn before that, none after.
    let out = rain.iter().position(|tick| all::<LevelEvent>(tick).iter().any(|event| event.event == LevelEventEvent::ParticleFizzEffect));
    assert!(out.is_some_and(|at| at >= 19), "steam {out:?} ticks into the rain");
    let burns = all::<EntityEvent>(&rain[out.unwrap() + 1..].concat()).iter().filter(|event| event.event_id == EntityEventEventId::HurtAnimation && event.data == 7).count();
    sent.extend(rain);
    let (_, causes) = told(&sent);
    assert!(causes.contains(&6) && burns == 0, "no burning on in the rain: {causes:?}, {burns} after the steam");
}

#[test]
fn rain_hurts_an_enderman_1_each_10_ticks_as_drowning() {
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_weather(Weather::Rain, None);
    });
    // Until the rain has set in (an_enderman_in_rain_teleports_on_and_on).
    ticks(&mut h, 20);
    penned(&mut h, 6, "stone", Block::air(), "minecraft:enderman");
    let (health, causes) = told(&ticks(&mut h, 25));
    assert_eq!((health, causes), (37.0, vec![9, 9, 9]));
}

#[test]
fn water_and_rain_hurt_a_blaze_and_a_snow_golem_1_each_10_ticks_as_drowning() {
    // research/captures/bds-o3wet.txt: in a pool and in the rain, hurt (data 9) every 10 ticks; a snow
    // golem's 4 health went in four.
    for (kind, health) in [("minecraft:blaze", 17.0), ("minecraft:snow_golem", 1.0)] {
        let mut h = ready();
        // Peaceful takes monsters away.
        h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
        penned(&mut h, 6, "stone", Block::new("water"), kind);
        // A lid, and water to it: BDS's were held in theirs by slowness.
        h.remote.run(|ctx| assert!(ctx.set_block([6, 66, 0], &Block::new("water")) && ctx.set_block([6, 67, 0], &Block::new("stone"))));
        assert_eq!(told(&ticks(&mut h, 25)), (health, vec![9, 9, 9]), "{kind} in water");
    }
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_weather(Weather::Rain, None);
    });
    ticks(&mut h, 20);
    penned(&mut h, 6, "stone", Block::air(), "minecraft:blaze");
    assert_eq!(told(&ticks(&mut h, 25)), (17.0, vec![9, 9, 9]), "a blaze in the rain");
}

#[test]
fn an_enderman_in_rain_teleports_on_and_on() {
    // research/captures/bds-enderrain.txt: the rain set in and first hurt it 20 ticks after
    // `/weather rain`, then seven teleports in 7 ticks.
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:enderman", [6.5, 65.0, 0.5]).is_some());
    });
    ticks(&mut h, 5);
    h.remote.run(|ctx| ctx.set_weather(Weather::Rain, None));
    let sent = ticks(&mut h, 30);
    let hurt = sent.iter().position(|tick| all::<EntityEvent>(tick).iter().any(|event| event.event_id == EntityEventEventId::HurtAnimation && event.data == 9));
    assert!(hurt.is_some_and(|at| (19..=21).contains(&at)), "first rain hurt {hurt:?} ticks in");
    let jumps = |ticks: &[Vec<RawPacket>]| all::<acacia_proto::packets::MoveEntityDelta>(&ticks.concat()).iter().filter(|moved| moved.force_move).count();
    assert!(jumps(&sent[..18]) <= 1, "{} teleports before the rain set in", jumps(&sent[..18]));
    assert!(jumps(&sent[19..]) >= 5, "{} teleports in 11 ticks of rain", jumps(&sent[19..]));
}

#[test]
fn a_cod_on_dry_stone_holds_its_breath_15_s_then_smothers() {
    let mut h = ready();
    penned(&mut h, 6, "stone", Block::air(), "minecraft:cod");
    let sent = ticks(&mut h, 320);
    let at = |id: EntityEventEventId| sent.iter().position(|tick| all::<EntityEvent>(tick).iter().any(|event| event.event_id == id));
    let (hurt, died) = (at(EntityEventEventId::HurtAnimation), at(EntityEventEventId::DeathAnimation));
    // research/captures/bds-harm2.txt: hurt for 2 of its 3 299 ticks after its summon, dead 10 after.
    assert!(hurt.is_some_and(|hurt| (298..=303).contains(&hurt) && died == Some(hurt + 10)), "{hurt:?} {died:?}");
}

#[test]
fn a_chicken_and_a_pig_over_lava_are_lit_and_scalded_where_their_move_left_them() {
    // research/captures/bds-bodies.txt: let go a block over a source, each was lit in the tick it
    // first dipped in; the chicken died of the lava 4 ticks on, the pig was scalded 2 on.
    for (kind, lava_after) in [("minecraft:chicken", 4), ("minecraft:pig", 2)] {
        let mut h = ready();
        penned(&mut h, 6, "lava", Block::air(), kind);
        let sent = ticks(&mut h, 12);
        let events: Vec<(usize, EntityEventEventId, i32)> = sent.iter().enumerate().flat_map(|(tick, sent)| all::<EntityEvent>(sent).into_iter().map(move |event| (tick, event.event_id, event.data))).collect();
        let lit = events.iter().find(|(_, id, data)| *id == EntityEventEventId::HurtAnimation && *data == 6).map(|(tick, ..)| *tick);
        let scalded = events.iter().find(|(_, id, data)| (*id == EntityEventEventId::HurtAnimation && *data == 8) || *id == EntityEventEventId::DeathAnimation).map(|(tick, ..)| *tick);
        assert!(lit.zip(scalded).is_some_and(|(lit, scalded)| scalded == lit + lava_after), "{kind}: {events:?}");
    }
}

#[test]
fn a_creeper_summoned_with_its_forced_fuse_bursts_30_ticks_on_with_no_prey() {
    let mut h = ready();
    // Peaceful takes monsters away.
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:creeper", [0.5, 65.0, 6.5], 0.0, 0.0, Some("minecraft:start_exploding_forced")).is_some()));
    let sent = ticks(&mut h, 40);
    let burst = sent.iter().position(|tick| all::<LevelEvent>(tick).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion));
    // research/captures/bds-fights.txt: 1.55 s after the summon, Alex in creative.
    assert!(burst.is_some_and(|at| (29..=33).contains(&at)), "{burst:?}");
}
