//! A creaking that loses its prey, against research/captures/bds-gxcreak.txt: `deactivate` heard,
//! its state back to 0 and its ambient sound `ambient`, still where it was.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, SetEntityData};
use common::{Harness, all};
use sapling_api::{Difficulty, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn states(sent: &[RawPacket], id: u64) -> Vec<i32> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.properties.ints).filter(|int| int.index == 0).map(|int| int.value).collect()
}

#[test]
fn a_creaking_whose_prey_turns_creative_deactivates() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.teleport_facing(ALEX, [0.5, 65.0, 0.5], [180.0, 0.0]);
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.spawn_entity("minecraft:creaking", [0.5, 65.0, -6.5]).is_some());
    });
    let hunted: Vec<RawPacket> = (0..40).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let creaking = all::<AddEntity>(&hunted).into_iter().find(|added| added.entity_type == "minecraft:creaking").expect("shown").runtime_id;
    assert!(states(&hunted, creaking).contains(&1), "active on its prey: {:?}", states(&hunted, creaking));
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let lost: Vec<RawPacket> = (0..5).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<LevelSoundEvent>(&lost).iter().any(|sound| sound.sound_id == "deactivate"), "deactivate heard");
    assert_eq!(states(&lost, creaking), [0]);
}
