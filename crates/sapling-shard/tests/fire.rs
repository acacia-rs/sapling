//! Fire lit by hand, against research/captures/bds-fire.txt.

mod common;

use acacia_proto::packets::LevelSoundEvent;
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

#[test]
fn flint_and_steel_lights_fire_over_a_block_and_soul_fire_over_soul_sand() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:flint_and_steel", 1))));
        assert!(ctx.set_block([2, 64, 2], &Block::new("minecraft:netherrack")) && ctx.set_block([2, 64, -2], &Block::new("minecraft:soul_sand")));
    });
    h.tick_all();
    for z in [0, 2, -2] {
        h.receive(1, &click([2, 64, z], 1, None));
    }
    let heard: Vec<(f32, f32, f32)> = all::<LevelSoundEvent>(h.tick_all().to(1)).into_iter().filter(|sound| sound.sound_id == "ignite").map(|sound| (sound.position.x, sound.position.y, sound.position.z)).collect();
    assert_eq!(heard, [(2.5, 65.5, 0.5), (2.5, 65.5, 2.5), (2.5, 65.5, -1.5)]);
    let lit = |h: &Harness| {
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send([0, 2, -2].map(|z| ctx.block([2, 65, z]).map(|block| block.name).unwrap_or_default())).expect("the test waits"));
        got
    };
    let now = lit(&h);
    h.tick_all();
    assert_eq!(now.recv().expect("ran"), ["minecraft:fire", "minecraft:fire", "minecraft:soul_fire"]);
    // A second strike into the fire does nothing.
    h.receive(1, &click([2, 64, 0], 1, None));
    assert!(all::<LevelSoundEvent>(h.tick_all().to(1)).iter().all(|sound| sound.sound_id != "ignite"));
    for _ in 0..610 {
        h.tick_all();
    }
    let later = lit(&h);
    h.tick_all();
    assert_eq!(later.recv().expect("ran"), ["minecraft:air", "minecraft:fire", "minecraft:soul_fire"], "fire over stone goes out");
}
