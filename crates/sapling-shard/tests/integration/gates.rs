//! A fence gate opened by hand swings away from its user (research/captures/bds-n2gates.txt).

use crate::common;

use common::{Harness, click};
use sapling_api::Block;

const GATE: [i32; 3] = [0, 65, 2];

fn gate(facing: &str, open: u8) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    let properties = format!("in_wall_bit=0,minecraft:cardinal_direction={facing},open_bit={open}");
    registry.network_hash(registry.find("minecraft:fence_gate", &properties).expect("a gate state"))
}

/// The gate's state after Alex, looking south, opens one put facing `facing`.
fn opened_facing(facing: &'static str) -> Vec<([i32; 3], u32)> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.receive(1, &common::input_at([0.5, 65.0, 0.5]));
    h.remote.run(move |ctx| assert!(ctx.set_block(GATE, &Block::new("fence_gate").with(&format!("minecraft:cardinal_direction={facing}")))));
    h.tick_all();
    h.receive(1, &click(GATE, 2, None));
    common::block_changes(h.tick_all().to(1))
}

#[test]
fn a_gate_facing_its_user_turns_away_as_it_opens_and_one_across_keeps_its_way() {
    assert_eq!(opened_facing("north"), [(GATE, gate("south", 1))]);
    assert_eq!(opened_facing("east"), [(GATE, gate("east", 1))]);
}
