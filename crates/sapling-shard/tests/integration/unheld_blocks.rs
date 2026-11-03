//! Blocks set where nothing holds them (research/captures/bds-unheld.txt): a torch, button or rail
//! is shown breaking at once and goes with its item a tick later; a lever or flower stays.

use crate::common;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::Block;

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name)).unwrap_or_default()
}

fn items(h: &mut Harness) -> usize {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.is_some()).count())
}

#[test]
fn a_torch_set_on_air_breaks_a_tick_later_and_a_lever_stays() {
    const TORCH: [i32; 3] = [2, 70, 0];
    const LEVER: [i32; 3] = [4, 70, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.update_block(TORCH, &Block::new("minecraft:torch").with("torch_facing_direction=west"))));
    let sent = h.tick_all().to(1).to_vec();
    let shown: Vec<LevelEventEvent> = all::<LevelEvent>(&sent).into_iter().map(|event| event.event).collect();
    assert!(shown.contains(&LevelEventEvent::ParticleDestroy), "{shown:?}");
    h.tick_all();
    assert_eq!(name_at(&mut h, TORCH), "minecraft:air");
    assert_eq!(items(&mut h), 1);
    h.remote.run(|ctx| assert!(ctx.update_block(LEVER, &Block::new("minecraft:lever").with("lever_direction=up_north_south"))));
    (0..3).for_each(|_| drop(h.tick_all()));
    assert_eq!(name_at(&mut h, LEVER), "minecraft:lever");
}

#[test]
fn a_bare_redstone_torch_goes_and_a_rail_goes_unseen() {
    const TORCH: [i32; 3] = [2, 70, 0];
    const RAIL: [i32; 3] = [4, 70, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.update_block(TORCH, &Block::new("minecraft:redstone_torch")) && ctx.update_block(RAIL, &Block::new("minecraft:rail"))));
    let sent = h.tick_all().to(1).to_vec();
    let shown: Vec<f32> = all::<LevelEvent>(&sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleDestroy).map(|event| event.position.x).collect();
    assert_eq!(shown, [2.5], "the torch's particles alone");
    h.tick_all();
    assert_eq!((name_at(&mut h, TORCH), name_at(&mut h, RAIL)), ("minecraft:air".to_owned(), "minecraft:air".to_owned()));
}

#[test]
fn a_torch_beside_a_pushed_block_breaks_a_tick_into_the_push() {
    // research/captures/bds-unheld.txt: particles as the push began, the torch gone a tick later.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.update_block([4, 64, 2], &Block::new("minecraft:piston").with("facing_direction=1")));
        assert!(ctx.update_block([4, 65, 2], &Block::new("minecraft:slime")) && ctx.update_block([3, 65, 2], &Block::new("minecraft:stone")));
        assert!(ctx.update_block([2, 65, 2], &Block::new("minecraft:torch").with("torch_facing_direction=east")));
    });
    (0..3).for_each(|_| drop(h.tick_all()));
    assert_eq!(name_at(&mut h, [2, 65, 2]), "minecraft:torch");
    h.remote.run(|ctx| assert!(ctx.update_block([5, 64, 2], &Block::new("minecraft:redstone_block"))));
    // Per tick: whether the torch's particles came, and whether the stone had arrived before it.
    let mut ticks = Vec::new();
    for _ in 0..12 {
        let stone = std::sync::Arc::new(std::sync::Mutex::new(false));
        let put = stone.clone();
        h.remote.run(move |ctx| *put.lock().unwrap() = ctx.block([3, 66, 2]).is_some_and(|block| block.name == "minecraft:stone"));
        let sent = h.tick_all().to(1).to_vec();
        let shown = all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::ParticleDestroy);
        ticks.push((shown, *stone.lock().unwrap()));
    }
    let broke = ticks.iter().position(|(shown, _)| *shown).expect("the torch broke");
    assert!(!ticks[broke + 1].1, "broken with a tick of the push left: {ticks:?}");
    assert_eq!(name_at(&mut h, [2, 65, 2]), "minecraft:air");
}
