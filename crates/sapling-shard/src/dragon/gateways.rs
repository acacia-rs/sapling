//! The gateways the kills open (research/dragon-fight-decompile.md 4), as BDS opened the first two
//! of seed 20261018 (research/captures/bds-dragon-again.txt): twenty places on a circle of 96 at
//! y 75, taken from the end of a list shuffled for the world. Each is a job that waits for the
//! columns round the gateway and round its exit, then builds the gateway (gateway_shape.rs) with
//! its exit in its block entity. A loaded gateway with no exit queues one that builds only the far
//! end; one with an exit queues a relink (relinks.rs). The jobs run one at a time, in order.

use std::collections::VecDeque;

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Fight, exits, gateway_shape, relinks};
use crate::components::Layer;

const COUNT: i32 = 20;
const RADIUS: f32 = 96.0;
const HEIGHT: i32 = 75;
/// Chunks round each end of a relink it reads.
const RELINK_READS: i32 = 3;

/// The gateway jobs a fight has yet to do, the one under way first.
#[derive(Default)]
pub(super) struct Jobs(VecDeque<Job>);

struct Job {
    at: BlockPos,
    work: Work,
}

enum Work {
    /// A far gateway and its island, and this one too when `place_main`.
    Build { place_main: bool, search: exits::Search },
    /// A relink with the gateway's exit, which the walk finds anew first when there is one.
    Relink { exit: BlockPos, walk: Option<exits::Search> },
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

fn push(world: &mut World, layer: Entity, job: Job) {
    if let Some(mut fight) = super::fight_of(world, layer) {
        fight.gateways.0.push_back(job);
    }
}

/// Has gateway `index` built once its exit is found.
pub(super) fn queue(world: &mut World, layer: Entity, index: i32) {
    let at = place(index);
    push(world, layer, Job { at, work: Work::Build { place_main: true, search: exits::Search::new(at) } });
}

/// Gives the gateway at `pos`, loaded with no exit, a far end (`_spawnNewGateway(pos, 0, 1)` from its tick 1487fcae0).
pub(crate) fn open_far(world: &mut World, layer: Entity, pos: BlockPos) {
    push(world, layer, Job { at: pos, work: Work::Build { place_main: false, search: exits::Search::new(pos) } });
}

/// Relinks the gateway at `pos`, loaded with `exit`; `walk` finds its exit anew first.
pub(crate) fn relink(world: &mut World, layer: Entity, pos: BlockPos, exit: BlockPos, walk: bool) {
    push(world, layer, Job { at: pos, work: Work::Relink { exit, walk: walk.then(|| exits::Search::new(pos)) } });
}

/// Goes on with the first queued job; it stays first until it has what it reads.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let Some(mut job) = world.get_mut::<Fight>(layer).and_then(|mut fight| fight.gateways.0.pop_front()) else { return };
    let at = job.at;
    let done = match &mut job.work {
        Work::Build { place_main, search } => {
            let found = world.get_mut::<Layer>(layer).and_then(|mut level| search.step(&mut level.world, at));
            if let Some(found) = &found {
                if *place_main {
                    build(world, layer, at, found.landing);
                } else {
                    set_exit(world, layer, at, found.landing, true);
                }
                build(world, layer, found.far, found.back);
            }
            found.is_some()
        }
        Work::Relink { exit, walk } => relink_step(world, layer, at, exit, walk),
    };
    if !done {
        if let Some(mut fight) = world.get_mut::<Fight>(layer) {
            fight.gateways.0.push_front(job);
        }
    }
}

/// A step of a relink; whether it is over.
fn relink_step(world: &mut World, layer: Entity, at: BlockPos, exit: &mut BlockPos, walk: &mut Option<exits::Search>) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return true };
    if let Some(search) = walk {
        let Some([x, z]) = search.point(&mut level.world) else { return false };
        *exit = [x.floor() as i32, 0, z.floor() as i32];
        *walk = None;
        drop(level);
        set_exit(world, layer, at, *exit, false);
        return false;
    }
    if !exits::loaded(&mut level.world, at, RELINK_READS) || !exits::loaded(&mut level.world, *exit, RELINK_READS) {
        return false;
    }
    let near = relinks::link(&mut level.world, at, *exit, false);
    let far = relinks::link(&mut level.world, *exit, at, true);
    drop(level);
    for (gateway, new_exit) in near.into_iter().chain(far) {
        set_exit(world, layer, gateway, new_exit, true);
    }
    true
}

/// Puts `exit` in the gateway entity at `pos`, shown as BDS shows a changed gateway when `shown`.
fn set_exit(world: &mut World, layer: Entity, pos: BlockPos, exit: BlockPos, shown: bool) {
    let Some(entity) = world.get::<Layer>(layer).and_then(|level| level.world.block_entity(pos)).map(|entity| relinks::with_exit(&entity.nbt, exit)) else { return };
    let kept = world.get_mut::<Layer>(layer).is_some_and(|mut level| level.world.set_block_entity(pos, &entity));
    if !kept || !shown {
        return;
    }
    let Some(packets) = world.get::<Layer>(layer).map(|level| relinks::shown(&level.world, pos)) else { return };
    crate::view::to_viewers(world, layer, pos, &packets);
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
