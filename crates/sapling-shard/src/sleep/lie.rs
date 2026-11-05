//! A click on a bed: whether its clicker lies down in it, in BDS's order of checks
//! (`BedBlock::use_sleep`, `Player::startSleepInBed`; docs/research/beds-and-containers.md, 2).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, GameMode};
use sapling_data::EYE_HEIGHT;

use super::{BED, BED_REACH, HEAD_AHEAD, LYING_HEIGHT, LYING_YAW, OCCUPIED, Sleeping, rest, set_occupied, show, show_box, spawn, stand, tell, tell_sleepers};
use crate::blocks::bed_head;
use crate::components::{Layer, Mode, Position, Settings};
use crate::movement;

const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];

/// A click on the block `name` at `pos`: a bed lies the player down in it if it may. False for
/// any other block.
pub(crate) fn lie_down(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let Some(head) = (name == BED).then(|| bed_head(world, layer, pos)).flatten() else { return name == BED };
    if crate::blocks::bed_bursts(world, layer, pos) {
        return true;
    }
    let occupied = world.get::<Layer>(layer).is_some_and(|level| {
        let id = level.world.block(head[0], head[1], head[2]);
        level.world.registry().get(id).and_then(|state| state.property(OCCUPIED)) == Some("1")
    });
    if occupied {
        let mut sleepers = world.query::<&Sleeping>();
        if sleepers.iter(world).any(|sleeping| sleeping.layer == layer && sleeping.head == head) {
            tell(world, player, "tile.bed.occupied");
            return true;
        }
        set_occupied(world, layer, head, false);
    }
    // Water in the bed's liquid layer refuses one that water chokes, silently (BDS result 4).
    if is_flooded(world, layer, head) && !crate::vitals::breathes_water(world, player) {
        return true;
    }
    let Some(position) = world.get::<Position>(player).copied() else { return true };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    if (0..3).any(|axis| (eye[axis] - head[axis] as f32).abs() > BED_REACH[axis]) {
        tell(world, player, "tile.bed.tooFar");
        return true;
    }
    if world.get::<Sleeping>(player).is_some() {
        return true;
    }
    let direction = world.get::<Layer>(layer).and_then(|level| level.world.registry().get(level.world.block(head[0], head[1], head[2]))?.property("direction")?.parse::<usize>().ok()).unwrap_or(0) % 4;
    let foot = [head[0] - HEAD_AHEAD[direction][0], head[1], head[2] - HEAD_AHEAD[direction][1]];
    let roomy = world.get::<Layer>(layer).is_some_and(|level| stand::stand_up_cell(&level.world, foot, direction, position.feet).is_some());
    if !roomy {
        tell(world, player, "tile.bed.obstructed");
        return true;
    }
    spawn::set_spawn(world, player, head);
    if !rest::is_resting_time(world, layer) {
        tell(world, player, "tile.bed.noSleep");
        return true;
    }
    let watched = world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) && world.resource::<Settings>().0.difficulty != Difficulty::Peaceful;
    if watched && rest::monster_near(world, layer, head) {
        tell(world, player, "tile.bed.notSafe");
        return true;
    }
    if !crate::life_events::may_sleep(world, player, head) {
        return true;
    }
    let yaw = LYING_YAW[direction];
    if let Some(mut lying) = world.get_mut::<Position>(player) {
        (lying.yaw, lying.head_yaw, lying.pitch) = (yaw, yaw, 0.0);
    }
    world.entity_mut(player).insert(Sleeping { layer, head, foot, direction, entered: position.feet, ticks: 0 });
    show(world, player, Some((head, position.feet)));
    movement::correct_unmoving(world, player, [head[0] as f32 + 0.5, head[1] as f32 + LYING_HEIGHT - EYE_HEIGHT, head[2] as f32 + 0.5]);
    show_box(world, player, true);
    set_occupied(world, layer, head, true);
    tell_sleepers(world, layer);
    true
}

fn is_flooded(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    world.get::<Layer>(layer).is_some_and(|level| level.world.registry().get(level.world.liquid(x, y, z)).is_some_and(|state| WATER.iter().any(|water| state.name == *water)))
}
