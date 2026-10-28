//! Loads that put a template over time, as BDS's `layer_by_layer` and `block_by_block` did
//! (research/captures/bds-structanim.txt): `t` ticks after the load, the first `floor(t * N / T)`
//! of its N layers or blocks are in, T being the seconds asked times 20. Cells go in y, then x,
//! then z order of the template before it is turned; its entities come with the last.
// TODO: when BDS brings a template's entities into an animated load is not drilled.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::StructureTemplate;

use super::place::Placing;
use crate::components::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Animation {
    None,
    LayerByLayer,
    BlockByBlock,
}

struct Run {
    layer: Entity,
    placing: Placing,
    /// The cells of each step, in order.
    steps: Vec<Vec<usize>>,
    from: u64,
    ticks: f64,
    done: usize,
}

#[derive(Resource, Default)]
struct Animating(Vec<Run>);

/// Loads `template` at `at`, over `seconds` as `animation` asks; at once without one.
pub(crate) fn load(world: &mut World, layer: Entity, template: StructureTemplate, at: BlockPos, how: &sapling_api::StructureLoad, animation: Animation, seconds: f32) {
    let Some(placing) = Placing::new(world, layer, template, at, how) else { return };
    let ticks = f64::from(seconds) * 20.0;
    if animation == Animation::None || ticks < 1.0 {
        placing.place_cells(world, layer, &placing.kept);
        return placing.place_entities(world, layer);
    }
    let mut cells = placing.kept.clone();
    cells.sort_by_key(|&index| {
        let [x, y, z] = placing.template().offset(index);
        (y, x, z)
    });
    let steps = match animation {
        Animation::LayerByLayer => cells.chunk_by(|&a, &b| placing.template().offset(a)[1] == placing.template().offset(b)[1]).map(<[usize]>::to_vec).collect(),
        _ => cells.into_iter().map(|cell| vec![cell]).collect(),
    };
    let from = world.resource::<Tick>().0;
    world.get_resource_or_insert_with(Animating::default).0.push(Run { layer, placing, steps, from, ticks, done: 0 });
}

/// Puts what each animated load is due to have put by now.
pub(crate) fn tick(world: &mut World) {
    let Some(mut animating) = world.get_resource_mut::<Animating>().filter(|animating| !animating.0.is_empty()) else { return };
    let runs = std::mem::take(&mut animating.0);
    let now = world.resource::<Tick>().0;
    let mut going = Vec::new();
    for mut run in runs {
        let total = run.steps.len();
        let due = (((now - run.from) as f64 * total as f64 / run.ticks).floor() as usize).min(total);
        if due > run.done {
            let cells: Vec<usize> = run.steps[run.done..due].concat();
            run.placing.place_cells(world, run.layer, &cells);
            run.done = due;
        }
        if run.done == total {
            run.placing.place_entities(world, run.layer);
        } else {
            going.push(run);
        }
    }
    world.resource_mut::<Animating>().0.extend(going);
}
