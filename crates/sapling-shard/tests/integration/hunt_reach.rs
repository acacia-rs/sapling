//! How far a hunter keeps its prey, against research/captures/bds-y3hunt.txt.

use crate::common;

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, PlayerId};

const ALEX: PlayerId = PlayerId(1);

#[test]
fn a_zombie_28_off_takes_its_prey_at_each_look_and_lets_it_go_a_tick_later() {
    // BDS: TargetEid set every 10 ticks and 0 a tick after each, past `within_radius` 25.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        for x in 26..=31 {
            for z in -2..=2 {
                assert!(ctx.set_block([x, 64, z], &Block::new("stone")));
            }
        }
        assert!(ctx.spawn_entity("minecraft:zombie", [28.5, 65.0, 0.5]).is_some());
    });
    let told: Vec<(usize, i64)> = (0..60)
        .flat_map(|tick| {
            let sent = h.tick_all().to(ALEX.0).to_vec();
            let target = |data: SetEntityData| data.metadata.iter().find(|item| item.key == Key::TargetEid).map(|item| item.value.clone());
            all::<SetEntityData>(&sent).into_iter().filter_map(target).map(move |value| if let Value::Default(Plain::Long(id)) = value { (tick, id) } else { panic!("{value:?}") }).collect::<Vec<_>>()
        })
        .collect();
    let takes: Vec<usize> = told.iter().filter(|(_, id)| *id != 0).map(|(tick, _)| *tick).collect();
    assert!(takes.len() >= 3, "{told:?}");
    assert!(takes.iter().filter(|tick| **tick < 59).all(|tick| told.contains(&(tick + 1, 0))), "{told:?}");
    assert!(takes.windows(2).all(|pair| pair[1] - pair[0] == 10), "{told:?}");
}
