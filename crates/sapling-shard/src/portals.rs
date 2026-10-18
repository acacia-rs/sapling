//! Travel by Nether portal, as BDS times it (`PortalCooldownDurationComponent`, ticked in
//! 140d2e670) and carries it out (`PlayerDimensionTransferer`, `PortalForcer`): standing in a
//! portal long enough starts a dimension change to where the record nearest the scaled place
//! is, or to the scaled place itself, and the traveller is put in a portal there once its
//! columns are loaded, one being made if none is near (portals/arrive.rs;
//! docs/research/nether-dimension.md). The portal blocks themselves: blocks/portals.rs.

mod arrive;
mod draws;
mod records;

use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_data::EYE_HEIGHT;
use sapling_world::{NETHER, OVERWORLD};

use crate::blocks::portals::PORTAL;
use crate::components::{InLayer, Layer, Mode, Position, in_world};
use crate::dimensions::{self, Arriving, Trip};

pub(crate) use arrive::arrive;

/// A player's cooldown after a trip, held at this while it stays in a portal.
const COOLDOWN: u32 = 10;
/// The cooldown a trip to or from the Nether starts with.
const AFTER_TRIP: u32 = 300;
/// Ticks in a portal before a trip, by game mode: the trip goes on the tick after these.
const SURVIVAL_WAIT: u32 = 80;
const CREATIVE_WAIT: u32 = 0;
/// What a tick out of a portal takes off the time in one.
const DECAY: u32 = 4;
/// Half a player's width, and its height, for the cells it is in.
const HALF_WIDTH: f32 = 0.3;
const HEIGHT: f32 = 1.8;

/// A player's time in portals.
#[derive(Component, Default)]
pub(crate) struct PortalTime {
    inside: u32,
    cooldown: u32,
}

/// Whether a player's body is in a portal block of its world.
fn is_inside(world: &World, player: Entity) -> bool {
    let (Some(feet), Some(level)) = (world.get::<Position>(player).map(|p| p.feet), world.get::<InLayer>(player).and_then(|l| world.get::<Layer>(l.0))) else { return false };
    let range = |low: f32, high: f32| (low.floor() as i32)..=(high.floor() as i32);
    let [x, y, z] = feet;
    range(x - HALF_WIDTH, x + HALF_WIDTH).any(|cx| {
        range(z - HALF_WIDTH, z + HALF_WIDTH).any(|cz| range(y, y + HEIGHT - 0.01).any(|cy| level.world.in_height(cy) && level.world.registry().get(level.world.block(cx, cy, cz)).is_some_and(|state| state.name == PORTAL)))
    })
}

/// Counts each player's time in portals and starts the trips that are due.
pub(crate) fn tick(world: &mut World) {
    let mut due = Vec::new();
    for player in in_world(world) {
        let wait = match world.get::<Mode>(player).map(|mode| mode.0) {
            Some(GameMode::Creative) => CREATIVE_WAIT,
            Some(GameMode::Survival | GameMode::Adventure) => SURVIVAL_WAIT,
            // TODO: BDS keeps a spectator's last wait; whether it is ever inside was not read.
            _ => continue,
        };
        let (inside, arriving) = (is_inside(world, player), world.get::<Arriving>(player).is_some());
        if world.get::<PortalTime>(player).is_none() {
            world.entity_mut(player).insert(PortalTime::default());
        }
        let Some(mut time) = world.get_mut::<PortalTime>(player) else { continue };
        match (inside, time.cooldown) {
            (true, 0) if !arriving => {
                let spent = time.inside;
                time.inside += 1;
                if spent >= wait {
                    (time.inside, time.cooldown) = (wait, COOLDOWN);
                    due.push(player);
                }
            }
            (true, _) => time.cooldown = COOLDOWN,
            (false, _) => time.inside = time.inside.saturating_sub(DECAY),
        }
        time.cooldown = time.cooldown.saturating_sub(1);
    }
    due.into_iter().for_each(|player| depart(world, player));
}

/// x and z scaled to the other dimension, toward zero (BDS `translatePosAcrossDimension`).
pub(crate) fn translate([x, y, z]: [f32; 3], to: i32) -> [f32; 3] {
    let scale = |c: f32| if to == NETHER { (c / 8.0).trunc() } else { (c * 8.0).trunc() };
    [scale(x), y, scale(z)]
}

/// Starts a player's trip through the portal it stands in.
fn depart(world: &mut World, player: Entity) {
    let (Some(layer), Some(feet)) = (world.get::<InLayer>(player).map(|l| l.0), world.get::<Position>(player).map(|p| p.feet)) else { return };
    let Some(from) = world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return };
    let to = match from {
        OVERWORLD => NETHER,
        NETHER => OVERWORLD,
        _ => return,
    };
    let Some(target) = dimensions::of(world, layer, to) else { return };
    let entrance = [feet[0], feet[1] + EYE_HEIGHT, feet[2]];
    let scaled = translate(entrance, to);
    let home = dimensions::home(world, layer);
    let found = world.get::<Layer>(home).and_then(|level| records::nearest(&level.world.level().portals, to, scaled.map(|c| c.floor() as i32)));
    // Where the record's nearest cell is, else the scaled place: what ChangeDimension names.
    let place = found.map_or(scaled, |(_, cell)| cell.map(|c| c as f32 + 0.5));
    dimensions::travel(world, player, target, place);
    if let Some(mut arriving) = world.get_mut::<Arriving>(player) {
        arriving.trip = Some(Trip { entrance, from });
    }
    if let Some(mut time) = world.get_mut::<PortalTime>(player) {
        time.cooldown = AFTER_TRIP;
    }
}
