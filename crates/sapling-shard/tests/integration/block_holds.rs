//! What rising rails, hanging signs and small dripleaves need to stay, and coral's `place` sounds
//! (research/captures/bds-t5holds.txt, bds-e3coral.txt).

use crate::common;

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::Block;

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name)).unwrap_or_default()
}

fn put(h: &mut Harness, cells: Vec<([i32; 3], Block)>) {
    h.remote.run(move |ctx| cells.iter().for_each(|(pos, block)| assert!(ctx.update_block(*pos, block), "{block:?} at {pos:?}")));
}

fn ticks(h: &mut Harness, n: usize) {
    (0..n).for_each(|_| drop(h.tick_all()));
}

fn items(h: &mut Harness) -> usize {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.is_some()).count())
}

#[test]
fn a_rising_rail_needs_the_cell_it_rises_to_at_its_own_height() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let stone = || Block::new("minecraft:stone");
    put(&mut h, vec![([2, 70, 0], stone()), ([3, 70, 0], stone()), ([6, 70, 0], stone()), ([3, 71, 0], stone())]);
    put(&mut h, vec![([2, 71, 0], Block::new("minecraft:activator_rail").with("rail_direction=2")), ([6, 71, 0], Block::new("minecraft:activator_rail").with("rail_direction=4"))]);
    ticks(&mut h, 3);
    assert_eq!([name_at(&mut h, [2, 71, 0]), name_at(&mut h, [6, 71, 0])], ["minecraft:activator_rail", "minecraft:air"], "the second rises to air");
    put(&mut h, vec![([3, 71, 0], Block::air())]);
    ticks(&mut h, 3);
    assert_eq!(name_at(&mut h, [2, 71, 0]), "minecraft:air");
}

#[test]
fn a_hanging_sign_goes_at_a_change_beside_it_without_a_bottom_above_to_hang_from() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sign = || Block::new("minecraft:oak_hanging_sign").with("hanging=1,attached_bit=1");
    let slab = Block::new("minecraft:smooth_stone_slab").with("minecraft:vertical_half=top");
    put(&mut h, vec![([2, 72, 0], Block::new("minecraft:iron_chain")), ([4, 72, 0], slab), ([6, 72, 0], Block::new("minecraft:stone"))]);
    put(&mut h, vec![([2, 71, 0], sign()), ([4, 71, 0], sign()), ([6, 71, 0], sign()), ([8, 71, 0], sign())]);
    ticks(&mut h, 3);
    let signs = [2, 4, 6, 8].map(|x| name_at(&mut h, [x, 71, 0]));
    assert!(signs.iter().all(|name| name == "minecraft:oak_hanging_sign"), "none goes as it is set: {signs:?}");
    put(&mut h, vec![([2, 71, 1], Block::new("minecraft:stone")), ([4, 71, 1], Block::new("minecraft:stone")), ([8, 71, 1], Block::new("minecraft:stone")), ([6, 72, 0], Block::air())]);
    ticks(&mut h, 3);
    let signs = [2, 4, 6, 8].map(|x| name_at(&mut h, [x, 71, 0]));
    assert_eq!(signs, ["minecraft:oak_hanging_sign", "minecraft:air", "minecraft:air", "minecraft:air"], "only the chain holds one");
}

#[test]
fn a_small_dripleaf_set_on_stone_comes_whole_and_goes_whole_and_its_lower_half_goes_with_the_upper() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let leaf = || Block::new("minecraft:small_dripleaf_block").with("minecraft:cardinal_direction=north,upper_block_bit=0");
    put(&mut h, vec![([2, 70, 0], Block::new("minecraft:stone")), ([6, 70, 0], Block::new("minecraft:clay"))]);
    ticks(&mut h, 1);
    put(&mut h, vec![([2, 71, 0], leaf())]);
    let sent = h.tick_all().to(1).to_vec();
    let broke: Vec<f32> = all::<LevelEvent>(&sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleDestroy).map(|event| event.position.y).collect();
    assert_eq!(broke, [71.5, 72.5], "both halves shown breaking");
    ticks(&mut h, 2);
    assert_eq!(name_at(&mut h, [2, 71, 0]), "minecraft:air");
    assert_eq!(name_at(&mut h, [2, 72, 0]), "minecraft:air");
    put(&mut h, vec![([6, 71, 0], leaf())]);
    ticks(&mut h, 3);
    assert_eq!(name_at(&mut h, [6, 72, 0]), "minecraft:small_dripleaf_block", "on clay it stays, its upper half with it");
    put(&mut h, vec![([6, 72, 0], Block::air())]);
    ticks(&mut h, 3);
    assert_eq!(name_at(&mut h, [6, 71, 0]), "minecraft:air");
    assert_eq!(items(&mut h), 0, "it leaves nothing but to shears");
}

fn placed_at(sent: &[acacia_proto::RawPacket]) -> Vec<f32> {
    all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.sound_id == "place").map(|sound| sound.position.x).collect()
}

#[test]
fn coral_plants_and_fans_set_or_dying_play_place_and_coral_blocks_do_not() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, vec![([2, 70, 0], Block::new("minecraft:stone")), ([4, 70, 0], Block::new("minecraft:stone"))]);
    ticks(&mut h, 1);
    put(&mut h, vec![([2, 71, 0], Block::new("minecraft:tube_coral")), ([4, 71, 0], Block::new("minecraft:tube_coral_block"))]);
    let sent = h.tick_all().to(1).to_vec();
    assert_eq!(placed_at(&sent), [2.5], "the plant alone");
    let mut later = Vec::new();
    for _ in 0..220 {
        later.extend(placed_at(&h.tick_all().to(1).to_vec()));
    }
    assert_eq!(name_at(&mut h, [2, 71, 0]), "minecraft:dead_tube_coral");
    assert_eq!(name_at(&mut h, [4, 71, 0]), "minecraft:dead_tube_coral_block");
    assert_eq!(later, [2.5], "the plant's death alone");
}
