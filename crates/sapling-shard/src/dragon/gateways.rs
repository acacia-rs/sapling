//! The gateways the kills open (research/dragon-fight-decompile.md 4), as BDS opened the first two
//! of seed 20261018 (research/captures/bds-dragon-again.txt): twenty places on a circle of 96 at
//! y 75, taken from the end of a list shuffled for the world. Each is a job that waits for the
//! columns round the gateway and round its exit, then builds the gateway (gateway_shape.rs) with
//! its exit in its block entity.

use std::collections::VecDeque;

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Fight, exits, gateway_shape};
use crate::components::Layer;

const COUNT: i32 = 20;
const RADIUS: f32 = 96.0;
const HEIGHT: i32 = 75;

/// The gateways a fight has yet to build, the one under way first.
#[derive(Default)]
pub(super) struct Jobs(VecDeque<Job>);

struct Job {
    at: BlockPos,
    search: exits::Search,
}

/// The cell of gateway `index`: `96·cos(2(i·0.15707964 - π))` in f32, truncated (14829b719).
pub(super) fn place(index: i32) -> BlockPos {
    let angle = (index as f32 * 0.157_079_64 + -std::f32::consts::PI) * 2.0;
    [(angle.cos() * RADIUS) as i32, HEIGHT, (angle.sin() * RADIUS) as i32]
}

/// The twenty gateways in the order a world keeps them, the next last: shuffled forward on
/// MT19937 of the seed's low half, as the spikes' sizes are.
pub(super) fn order(seed: i64) -> Vec<i32> {
    let mut random = sapling_worldgen::random::Mt19937::new(seed as u32);
    let mut order: Vec<i32> = (0..COUNT).collect();
    for i in 1..order.len() {
        let j = (random.next_u32() % (i as u32 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// Has gateway `index` built once its exit is found.
pub(super) fn queue(world: &mut World, layer: Entity, index: i32) {
    let at = place(index);
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.gateways.0.push_back(Job { at, search: exits::Search::new(at) });
    }
}

/// Goes on with the first queued gateway's search; once its exit is found, builds it and its far
/// end, each leading to the other.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let Some(mut job) = world.get_mut::<Fight>(layer).and_then(|mut fight| fight.gateways.0.pop_front()) else { return };
    let found = world.get_mut::<Layer>(layer).and_then(|mut level| job.search.step(&mut level.world, job.at));
    let Some(found) = found else {
        if let Some(mut fight) = world.get_mut::<Fight>(layer) {
            fight.gateways.0.push_front(job);
        }
        return;
    };
    build(world, layer, job.at, found.landing);
    build(world, layer, found.far, found.back);
}

fn build(world: &mut World, layer: Entity, at: BlockPos, exit: BlockPos) {
    let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { return };
    let cells: Vec<(BlockPos, u32)> = gateway_shape::cells(at).into_iter().filter_map(|(pos, (name, state))| Some((pos, registry.find(name, state)?))).collect();
    crate::blocks::set_sectioned(world, layer, &cells);
    crate::blocks::keep_entity(world, layer, at, &block_entity(at, exit));
}

fn block_entity([x, y, z]: BlockPos, exit: BlockPos) -> Value {
    let exit = Value::List(List { tag: 3, items: exit.iter().map(|c| Value::Int(*c)).collect() });
    Value::Compound(vec![
        ("Age".into(), Value::Int(0)),
        ("BlockEntityVersion".into(), Value::Int(0)),
        ("EndGatewayBadPosChecked".into(), Value::Byte(1)),
        ("ExitPortal".into(), exit),
        ("id".into(), Value::String("EndGateway".into())),
        ("x".into(), Value::Int(x)),
        ("y".into(), Value::Int(y)),
        ("z".into(), Value::Int(z)),
    ])
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_first_two_gateways_are_where_bds_opened_them() {
        assert_eq!(super::place(15), [0, 75, -96]);
        assert_eq!(super::place(17), [56, 75, -77]);
    }

    #[test]
    fn the_order_is_the_one_bds_kept_for_the_drill_seed() {
        assert_eq!(super::order(20261018), [12, 10, 3, 1, 2, 6, 19, 13, 11, 16, 9, 5, 14, 18, 4, 7, 8, 0, 17, 15]);
    }
}
