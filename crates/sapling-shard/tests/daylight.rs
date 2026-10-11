//! Daylight: what the sun burns and whom it keeps from hunting
//! (research/captures/bds-sun.txt, bds-spiders.txt).

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

#[test]
fn the_sun_burns_a_zombie_under_the_sky_and_not_a_creeper_or_one_under_a_roof() {
    // research/captures/bds-sun.txt: 1 a second from the tick it stands in the sun.
    let burnt = |kind: &'static str, time: i64, roofed: bool| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_game_mode(PlayerId(1), GameMode::Creative);
            ctx.set_time(time);
            if roofed {
                assert!(ctx.set_block([4, 68, 0], &Block::new("stone")));
            }
            assert!(ctx.spawn_entity(kind, [4.5, 65.0, 0.5]).is_some());
        });
        let sent: Vec<RawPacket> = (0..65).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        all::<EntityEvent>(&sent).iter().filter(|event| event.event_id == EntityEventEventId::HurtAnimation && event.data == 7).count()
    };
    assert_eq!([burnt("minecraft:zombie", 6000, false), burnt("minecraft:zombie", 6000, true), burnt("minecraft:zombie", 13000, false), burnt("minecraft:creeper", 6000, false)], [4, 0, 0, 0]);
}

#[test]
fn a_spider_in_the_sun_leaves_a_player_be_and_one_under_a_roof_does_not() {
    // research/captures/bds-spiders.txt.
    let bitten = |roofed: bool| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_difficulty(Difficulty::Easy);
            ctx.set_time(6000);
            assert!(ctx.spawn_entity("minecraft:spider", [0.5, 65.0, 4.5]).is_some());
            if roofed {
                (0..=5).for_each(|z| assert!(ctx.set_block([0, 69, z], &Block::new("stone"))));
            }
        });
        let sent: Vec<RawPacket> = (0..80).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation)
    };
    assert_eq!([bitten(false), bitten(true)], [false, true]);
}
