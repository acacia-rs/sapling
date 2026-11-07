//! Group AO: how commanded rails and a clone move are listed (docs/research/block-rules.md; research/captures
//! bds-ao2rails.txt), and where a summoned boat or mob is put (vehicles.md, water-movement.md; bds-ao2boat.txt,
//! bds-ao2push.txt).

use crate::common;

use acacia_proto::packets::AddEntity;
use common::asking::ask;
use common::{Harness, all, block_changes, hash};
use sapling_api::{Block, Copied};

/// Stone at y 64 over x -1 to 12, z -2 to 8, open air over it.
fn floored() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        for x in -1..=12 {
            for z in -2..=8 {
                ctx.set_block([x, 64, z], &Block::new("stone"));
                (65..=67).for_each(|y| _ = ctx.set_block([x, y, z], &Block::air()));
            }
        }
    });
    h.tick_all();
    h
}

fn rail(direction: u8) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find("minecraft:rail", &format!("rail_direction={direction}")).expect("a state"))
}

/// BDS listed a filled rail line as each write made it: a rail's own write after its join's, a cell
/// listed again whenever it turned (`2 [0]; 3 [1]; 2 [1]; 3 [1]; 4 [1]; 4 [1]`).
#[test]
fn rails_filled_in_a_line_are_listed_write_by_write() {
    let mut h = floored();
    h.remote.run(|ctx| _ = ctx.batch_blocks(|ctx| (2..=4).for_each(|x| assert!(ctx.update_block([x, 65, 0], &Block::new("rail"))))));
    let shown = block_changes(h.tick_all().to(1));
    let at = |x| [x, 65, 0];
    assert_eq!(shown, [(at(2), rail(0)), (at(3), rail(1)), (at(2), rail(1)), (at(3), rail(1)), (at(4), rail(1)), (at(4), rail(1))]);
}

/// A rail set north of a pair's east end keeps its north-south shape: BDS listed the end turned, then the
/// rail (`3 -5 [8]; 3 -6 [0]`).
#[test]
fn a_set_rail_that_keeps_its_shape_is_listed_after_the_rail_it_turned() {
    let mut h = floored();
    ask(&mut h, |ctx| (2..=3).for_each(|x| assert!(ctx.update_block([x, 65, 4], &Block::new("rail")))));
    h.remote.run(|ctx| assert!(ctx.update_block([3, 65, 3], &Block::new("rail"))));
    let shown = block_changes(h.tick_all().to(1));
    assert_eq!(shown, [([3, 65, 4], rail(8)), ([3, 65, 3], rail(0))]);
}

/// A move empties a cell of no full block before the rest, last first (water), and full blocks first to last.
#[test]
fn a_clone_move_empties_water_last_to_first_then_full_blocks_in_order() {
    let mut h = floored();
    ask(&mut h, |ctx| {
        ctx.set_block([2, 65, 6], &Block::new("red_wool"));
        ctx.set_block([3, 65, 6], &Block::new("water"));
        ctx.set_block([4, 65, 6], &Block::new("water"));
        ctx.set_block([5, 65, 6], &Block::new("blue_wool"));
    });
    h.remote.run(|ctx| {
        let copies: Vec<Copied> = (2..=5).map(|x| Copied { from: [x, 65, 6], to: [x, 66, 6], block: ctx.block([x, 65, 6]).unwrap(), entity: None }).collect();
        let cleared: Vec<[i32; 3]> = (2..=5).map(|x| [x, 65, 6]).collect();
        ctx.copy_blocks(&cleared, &copies);
    });
    let emptied: Vec<i32> = block_changes(h.tick_all().to(1)).into_iter().filter(|(at, block)| at[1] == 65 && *block == hash("minecraft:air")).map(|(at, _)| at[0]).collect();
    assert_eq!(emptied[..4], [4, 3, 2, 5]);
}

fn added(h: &mut Harness, kind: &'static str, feet: [f32; 3]) -> [f32; 3] {
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, kind, feet, 0.0, 0.0, None).is_some()));
    let shown = all::<AddEntity>(h.tick_all().to(1));
    let added = shown.into_iter().find(|added| added.entity_type == kind).expect("shown");
    [added.position.x, added.position.y, added.position.z]
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
}

/// BDS's boat summoned on the floor at a half block went 0.2 out of its low columns and showed 0.375 up;
/// one at 0.85 in a block stayed; one in the air showed where summoned.
#[test]
fn a_boat_summoned_on_the_floor_leaves_its_low_columns_and_one_in_the_air_stays() {
    let mut h = floored();
    assert!(near(added(&mut h, "minecraft:boat", [2.5, 65.0, 1.5]), [2.7, 65.375, 1.7]));
    assert!(near(added(&mut h, "minecraft:boat", [5.35, 65.0, 6.85]), [5.7, 65.375, 6.85]));
    assert!(near(added(&mut h, "minecraft:boat", [9.5, 70.0, 4.5]), [9.5, 70.0, 4.5]));
}

/// BDS's zombie summoned under a roof block went out beside it, 0.8 east; one 0.3 off its middle 0.6 west.
/// A villager (the harness is peaceful: no zombie), as wide and 0.05 lower, goes the same way.
#[test]
fn a_mob_summoned_with_its_head_in_a_roof_block_comes_out_beside_it() {
    let mut h = floored();
    ask(&mut h, |ctx| [[2, 66, 2], [8, 66, 2]].iter().for_each(|at| _ = ctx.set_block(*at, &Block::new("stone"))));
    assert!(near(added(&mut h, "minecraft:villager_v2", [2.5, 65.0, 2.5]), [3.3, 65.0, 2.5]));
    assert!(near(added(&mut h, "minecraft:villager_v2", [8.3, 65.0, 2.5]), [7.7, 65.0, 2.5]));
}
