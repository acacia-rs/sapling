//! Pressure plates: down the tick something is on one, checked again while down and up on the
//! first check that finds nothing, by BDS's `BasePressurePlateBlock`
//! (docs/research/pressure-plates.md).
// TODO: no redstone behind them; stored mobs, which are statues outside entities.rs, press nothing.

use std::collections::HashMap;

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};

use super::place::{PLAYER_HALF_WIDTH, PLAYER_HEIGHT};
use super::{doors, level, send};
use crate::components::{InLayer, Mode, Position, Stage, Tick};
use crate::entities;

const ORB: &str = "minecraft:xp_orb";
/// Entities that press a plate but are no mob.
const NOT_MOBS: [&str; 2] = ["minecraft:item", "minecraft:falling_block"];

const SIGNAL: &str = "redstone_signal";
/// The part of a plate's cell that feels a press: 1/16 in from each side, a quarter high.
const INSET: f32 = 1.0 / 16.0;
const FELT_HEIGHT: f32 = 0.25;
const PLAIN_DELAY: u64 = 20;
const WEIGHTED_DELAY: u64 = 10;
const FULL: u32 = 15;
const SOUND_HEIGHT: f32 = 0.1;

#[derive(Clone, Copy)]
enum Plate {
    /// Pressed by anything.
    Wooden,
    /// Pressed by mobs and players.
    Stone,
    /// Its signal grows with how many things are on it, up to this many.
    Weighted(u32),
}

impl Plate {
    fn of(name: &str) -> Option<Plate> {
        Some(match name.strip_suffix("pressure_plate")? {
            "minecraft:light_weighted_" => Plate::Weighted(15),
            "minecraft:heavy_weighted_" => Plate::Weighted(150),
            "minecraft:stone_" | "minecraft:polished_blackstone_" => Plate::Stone,
            _ => Plate::Wooden,
        })
    }

    fn delay(self) -> u64 {
        if let Plate::Weighted(_) = self { WEIGHTED_DELAY } else { PLAIN_DELAY }
    }

    /// The signal of a plate whose block stores `stored`: a plain plate stores 1 for down.
    fn signal_of(self, stored: u32) -> u32 {
        match self {
            Plate::Weighted(_) => stored,
            _ if stored > 0 => FULL,
            _ => 0,
        }
    }

    fn stored(self, signal: u32) -> u32 {
        match self {
            Plate::Weighted(_) => signal,
            _ => u32::from(signal > 0),
        }
    }

    /// The signal while `on` lie on the plate.
    fn signal(self, on: &[&Body]) -> u32 {
        match self {
            Plate::Weighted(most) => {
                let count = (on.len() as u32).min(most);
                (count as f32 / most as f32 * FULL as f32).ceil() as u32
            }
            Plate::Wooden if on.iter().any(|body| !body.spectator) => FULL,
            Plate::Stone if on.iter().any(|body| body.is_mob && !body.spectator) => FULL,
            _ => 0,
        }
    }
}

/// Something that may press a plate.
struct Body {
    layer: Entity,
    min: [f32; 3],
    max: [f32; 3],
    is_mob: bool,
    spectator: bool,
}

impl Body {
    fn around(layer: Entity, feet: [f32; 3], half_width: f32, height: f32, is_mob: bool, spectator: bool) -> Body {
        let min = [feet[0] - half_width, feet[1], feet[2] - half_width];
        Body { layer, min, max: [feet[0] + half_width, feet[1] + height, feet[2] + half_width], is_mob, spectator }
    }

    fn presses(&self, layer: Entity, [x, y, z]: BlockPos) -> bool {
        let felt_min = [x as f32 + INSET, y as f32, z as f32 + INSET];
        let felt_max = [(x + 1) as f32 - INSET, y as f32 + FELT_HEIGHT, (z + 1) as f32 - INSET];
        self.layer == layer && (0..3).all(|axis| self.min[axis] < felt_max[axis] && self.max[axis] > felt_min[axis])
    }

    /// The cells whose plate this body could be on.
    fn cells(&self) -> impl Iterator<Item = BlockPos> + '_ {
        let span = |axis: usize, below: f32| (self.min[axis] - below).floor() as i32..=self.max[axis].floor() as i32;
        span(0, 0.0).flat_map(move |x| span(1, FELT_HEIGHT).flat_map(move |y| span(2, 0.0).map(move |z| [x, y, z])))
    }
}

/// Plates that are down and the tick each is checked again.
#[derive(Resource, Default)]
pub(crate) struct DownPlates(HashMap<(Entity, BlockPos), u64>);

fn bodies(world: &mut World) -> Vec<Body> {
    let mut players = world.query::<(&InLayer, &Stage, &Position, &Mode)>();
    let mut bodies: Vec<Body> = players
        .iter(world)
        .filter(|(_, stage, ..)| **stage == Stage::InWorld)
        .map(|(layer, _, position, mode)| Body::around(layer.0, position.feet, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, true, mode.0 == GameMode::Spectator))
        .collect();
    // Experience orbs press nothing; anything else but a mob only wooden and weighted plates.
    let others = entities::bodies(world).into_iter().filter(|(_, kind, _)| kind != ORB);
    bodies.extend(others.map(|(layer, kind, body)| Body::around(layer, body.at, body.size[0] / 2.0, body.size[1], !NOT_MOBS.contains(&kind.as_str()), false)));
    bodies
}

/// The plate at `pos` and the signal its block stores.
fn plate_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(Plate, u32)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let state = level.registry().get(level.block(x, y, z))?;
    let stored = state.properties.split(',').find_map(|pair| pair.strip_prefix("redstone_signal="))?.parse().ok()?;
    Some((Plate::of(state.name)?, stored))
}

/// Writes the plate's new signal, with a click between up and down.
fn set_signal(world: &mut World, layer: Entity, pos: BlockPos, plate: Plate, old: u32, new: u32) {
    let Some(level) = level(world, layer) else { return };
    let now = level.block(pos[0], pos[1], pos[2]);
    let Some(id) = doors::with(level.registry(), now, SIGNAL, &plate.stored(new).to_string()).filter(|id| *id != now) else { return };
    let click = ((old > 0) != (new > 0)).then(|| {
        let at = Vec3f { x: pos[0] as f32 + 0.5, y: pos[1] as f32 + SOUND_HEIGHT, z: pos[2] as f32 + 0.5 };
        send::sound_at(if new > 0 { "pressure_plate.click_on" } else { "pressure_plate.click_off" }, level.registry(), at, id)
    });
    if send::set(world, layer, pos, id) {
        send::to_viewers(world, layer, pos, &click.into_iter().collect::<Vec<_>>());
    }
}

/// BDS `checkPressed`: the new signal, and while down a check again after the plate's delay.
fn check(world: &mut World, layer: Entity, pos: BlockPos, plate: Plate, old: u32, new: u32) {
    set_signal(world, layer, pos, plate, old, new);
    let when = world.resource::<Tick>().0 + plate.delay();
    let mut down = world.get_resource_or_insert_with(DownPlates::default);
    match new > 0 {
        true => drop(down.0.insert((layer, pos), when)),
        false => drop(down.0.remove(&(layer, pos))),
    }
}

/// Presses the plates something is on, and lets up those whose check finds nothing.
pub(crate) fn press_plates(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let bodies = bodies(world);
    let on = |layer: Entity, pos: BlockPos| bodies.iter().filter(|body| body.presses(layer, pos)).collect::<Vec<_>>();
    let mut touched: Vec<(Entity, BlockPos)> = bodies.iter().flat_map(|body| body.cells().map(|cell| (body.layer, cell))).collect();
    touched.sort_unstable_by_key(|(layer, pos)| (*layer, *pos));
    touched.dedup();
    for (layer, pos) in touched {
        let Some((plate, stored)) = plate_at(world, layer, pos) else { continue };
        match (plate.signal_of(stored), plate.signal(&on(layer, pos))) {
            (0, new) if new > 0 => check(world, layer, pos, plate, 0, new),
            // A weighted plate changes weight quietly, keeping the check it has.
            (old, new) if new > 0 && old != new => set_signal(world, layer, pos, plate, old, new),
            _ => {}
        }
    }
    let Some(down) = world.get_resource::<DownPlates>() else { return };
    let due: Vec<_> = down.0.iter().filter(|(_, when)| **when <= tick).map(|(at, _)| *at).collect();
    for (layer, pos) in due {
        match plate_at(world, layer, pos) {
            Some((plate, stored)) => check(world, layer, pos, plate, plate.signal_of(stored), plate.signal(&on(layer, pos))),
            None => drop(world.resource_mut::<DownPlates>().0.remove(&(layer, pos))),
        }
    }
}
