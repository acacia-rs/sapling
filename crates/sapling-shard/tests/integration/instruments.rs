//! Goat horns and their cooldown (instruments.rs; research/captures/bds-small.txt).

use crate::common;

use acacia_proto::packets::{ClientStartItemCooldown, CompletedUsingItem, LevelSoundEvent};
use common::all;
use common::shooting::{click_air, ready};
use sapling_api::GameMode;

#[test]
fn a_horn_sounds_its_variant_and_cools_down_until_used_again() {
    let mut h = ready(GameMode::Survival, &[("minecraft:goat_horn", 1, 3)], 0.0);
    h.receive(1, &click_air());
    let blown = h.tick_all().to(1).to_vec();
    assert_eq!(all::<LevelSoundEvent>(&blown).iter().map(|sound| sound.sound_id.as_str()).collect::<Vec<_>>(), ["horn_call3"]);
    assert_eq!(all::<ClientStartItemCooldown>(&blown).iter().map(|cool| cool.duration).collect::<Vec<_>>(), [140]);
    let mut later = Vec::new();
    for _ in 0..20 {
        later.extend(h.tick_all().to(1).to_vec());
    }
    assert_eq!(all::<CompletedUsingItem>(&later).len(), 1, "the use completes 20 ticks on");
    assert_eq!(all::<ClientStartItemCooldown>(&later).len(), 1, "told again as it completes");
    h.receive(1, &click_air());
    assert!(all::<LevelSoundEvent>(h.tick_all().to(1)).is_empty(), "still cooling");
}
