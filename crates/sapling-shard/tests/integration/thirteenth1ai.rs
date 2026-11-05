//! Against the group AI drills: what a sculk sensor hears a tick after the request that caused it
//! (research/captures/bds-ai1vib.txt), and amethyst resonance (bds-uureson.txt).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::nbt::Value as NbtValue;
use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelEventGeneric, MovePlayer, UpdateBlock};
use acacia_proto::types::Vec3f;
use acacia_proto::RawPacket;
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const SENSOR: [i32; 3] = [6, 65, 4];

/// Alex in survival at the spawn holding `held`, a sculk sensor at [`SENSOR`].
fn by_a_sensor(held: Option<&'static str>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), held.map(|held| Item::new(held, 1)).as_ref()));
        assert!(ctx.set_block(SENSOR, &Block::new("minecraft:sculk_sensor")));
    });
    (0..3).for_each(|_| _ = h.tick_all());
    h
}

/// The vibration particles' origins in `sent`.
fn vibrations(sent: &[RawPacket]) -> Vec<[f32; 3]> {
    all::<LevelEventGeneric>(sent)
        .into_iter()
        .filter_map(|event| {
            let origin = &event.nbt.iter().find(|nbt| nbt.name == "origin")?.value;
            let axis = |key: &str| if let Some(NbtValue::Float(c)) = origin.get(key) { Some(*c) } else { None };
            Some([axis("x")?, axis("y")?, axis("z")?])
        })
        .collect()
}

/// The sensor's phase each tick for `ticks` ticks.
fn phases(h: &mut Harness, ticks: usize) -> Vec<String> {
    (0..ticks).map(|_| common::asking::ask(h, |ctx| ctx.block(SENSOR).expect("loaded").properties)).collect()
}

#[test]
fn a_tp_is_heard_a_tick_after_the_move_and_reaches_the_sensor_as_many_ticks_after_as_it_is_far() {
    // bds-ai1vib.txt, bds-v2tp.txt, bds-uusculk.txt: each /tp's particle came a tick after the answer, the
    // sensor 5 off went active 5 ticks after the particle.
    let mut h = by_a_sensor(None);
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [1.2, 65.0, 3.8]));
    let ticks: Vec<(bool, Vec<[f32; 3]>)> = (0..3).map(|_| h.tick_all().to(1).to_vec()).map(|sent| (!all::<MovePlayer>(&sent).is_empty(), vibrations(&sent))).collect();
    assert_eq!(ticks, [(true, vec![]), (false, vec![[1.5, 65.5, 3.5]]), (false, vec![])]);
    let active = phases(&mut h, 5).iter().position(|phase| phase == "sculk_sensor_phase=1");
    assert_eq!(active, Some(3), "5 ticks after the particle");
}

#[test]
fn a_poured_bucket_is_heard_from_the_poured_cell_a_tick_after_the_water() {
    // bds-ai1vib.txt: water poured at 2 -26 0, heard as fluid_place from that cell's middle a tick after its
    // UpdateBlock, the sensor 5 off active 5 ticks after the particle. Sapling heard none.
    let mut h = by_a_sensor(Some("minecraft:water_bucket"));
    h.receive(1, &click([2, 64, 0], 1, None));
    let ticks: Vec<(bool, Vec<[f32; 3]>)> = (0..3).map(|_| h.tick_all().to(1).to_vec()).map(|sent| (!all::<UpdateBlock>(&sent).is_empty(), vibrations(&sent))).collect();
    assert_eq!(ticks.iter().map(|(_, heard)| heard.clone()).collect::<Vec<_>>(), [vec![], vec![[2.5, 65.5, 0.5]], vec![]]);
    assert!(ticks[0].0, "the water shown first");
}

#[test]
fn a_camel_summoned_saddled_without_its_age_group_seats_no_one() {
    // bds-ai1ride.txt: the bot's click on a `camel_saddled` camel left it standing; sapling seated it.
    let mut h = by_a_sensor(None);
    let camel = common::asking::ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:camel", [1.5, 65.0, 2.5], 0.0, 0.0, Some("minecraft:camel_saddled"))).expect("summoned");
    h.receive(1, &crate::riding::interact(camel.0));
    h.tick_all();
    assert!(common::asking::ask(&mut h, move |ctx| ctx.riders(camel)).is_empty());
}

#[test]
fn an_amethyst_beside_a_sensor_going_active_passes_its_frequency_to_a_sensor_out_of_the_events_reach() {
    // bds-uureson.txt: LevelEvent 1067 at the amethyst's corner a tick before the sensor went active, then a
    // vibration from the amethyst set off a sensor 7 off it, 13 off the tp.
    const AMETHYST: [i32; 3] = [7, 65, 4];
    const FAR: [i32; 3] = [14, 65, 4];
    let mut h = by_a_sensor(None);
    h.remote.run(|ctx| {
        assert!(ctx.set_block(AMETHYST, &Block::new("minecraft:amethyst_block")));
        assert!(ctx.set_block(FAR, &Block::new("minecraft:sculk_sensor")));
    });
    h.tick_all();
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [1.2, 65.0, 3.8]));
    let mut ticks = Vec::new();
    for _ in 0..20 {
        let phases = Arc::new(Mutex::new(None));
        let out = phases.clone();
        h.remote.run(move |ctx| *out.lock().unwrap() = Some([SENSOR, FAR].map(|pos| ctx.block(pos).expect("loaded").properties == "sculk_sensor_phase=1")));
        let sent = h.tick_all().to(1).to_vec();
        let rang: Vec<Vec3f> = all::<LevelEvent>(&sent).into_iter().filter(|event| event.event == LevelEventEvent::Unknown(1067)).map(|event| event.position).collect();
        ticks.push((rang, phases.lock().unwrap().take().expect("the handler ran")));
    }
    let first = |which: usize| ticks.iter().position(|(_, active)| active[which]).expect("went active");
    let rang = ticks.iter().position(|(rang, _)| !rang.is_empty()).expect("the amethyst rang");
    assert_eq!(ticks[rang].0, [Vec3f { x: 7.0, y: 65.0, z: 4.0 }]);
    assert_eq!(rang + 1, first(0), "rang a tick before the sensor went active");
    assert_eq!(first(1), first(0) + 7, "7 ticks from the amethyst");
}
