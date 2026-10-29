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
    sent.extend(ticks(&mut h, 60));
    let (_, causes) = told(&sent);
    assert!(causes.contains(&6) && !causes.contains(&7), "no burning on in the rain: {causes:?}");
}

#[test]
fn rain_hurts_an_enderman_1_each_10_ticks_as_drowning() {
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_weather(Weather::Rain, None);
    });
    penned(&mut h, 6, "stone", Block::air(), "minecraft:enderman");
    let (health, causes) = told(&ticks(&mut h, 25));
    assert_eq!((health, causes), (37.0, vec![9, 9, 9]));
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
fn a_creeper_summoned_with_its_forced_fuse_bursts_30_ticks_on_with_no_prey() {
    let mut h = ready();
    // Peaceful takes monsters away.
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:creeper", [0.5, 65.0, 6.5], 0.0, Some("minecraft:start_exploding_forced")).is_some()));
    let sent = ticks(&mut h, 40);
    let burst = sent.iter().position(|tick| all::<LevelEvent>(tick).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion));
    // research/captures/bds-fights.txt: 1.55 s after the summon, Alex in creative.
    assert!(burst.is_some_and(|at| (29..=33).contains(&at)), "{burst:?}");
}
