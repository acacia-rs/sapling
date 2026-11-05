//! Shriekers shrieking, and one that can summon warning the players near it and the fourth time
//! calling up a warden, as BDS's did in the `t2warden` drill (research/captures/bds-t2warden.txt;
//! sculk.md S19-S20). A player standing on one, or a sensor a player set off near it, sets it off.
//! One that can summon raises the warning level of the players within 16 by one, to 4, and shrieks
//! only if none may raise it for 200 ticks yet and no warden is within 48. As the shriek ends
//! (90 ticks), the survival players within 40 get 13 s of darkness, and the level's warning is
//! heard from a spot up to 10 blocks off each way, or at 4 a warden emerges near it.
// TODO: the level falling back a step every 12000 ticks is not saved with the player, and where
// the warden comes up is Java's search (5 across, 6 up or down), not decompiled.

use std::collections::HashMap;

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Effect};

use super::queued::{self, Job};
use super::{SHRIEKER, catalyst, level, name_at, number, send, set_number};
use crate::blocks::Dice;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::vitals;

/// Ticks a shrieker shrieks.
const SHRIEK: u64 = 90;
/// `LevelEventGeneric` 2035 `ParticlesSculkShriek`, zigzagged as the catalyst's.
const PARTICLES: u32 = 2035 << 1;
const SHARED: f32 = 16.0;
const WARDEN_NEAR: f32 = 48.0;
const COOLDOWN: u64 = 200;
const FALLS_BACK: u64 = 12000;
const TOP: u8 = 4;
const DARKNESS_REACH: f32 = 40.0;
const DARKNESS: u32 = 260;
const SOUND_SPREAD: i32 = 10;
/// Heard from the block's corner a tick after the particles, with nothing in `extra_data`; a
/// warning comes a tick after the darkness.
const SHRIEK_SOUND: &str = "block.sculk_shrieker.shriek";
const WARNINGS: [&str; 3] = ["nearby_close", "nearby_closer", "nearby_closest"];
const SEARCH: [i32; 2] = [5, 6];
const SEARCH_TRIES: usize = 20;

#[derive(Clone, Copy, Default)]
struct Tracker {
    warning: u8,
    /// The tick it was last raised.
    raised: u64,
}

impl Tracker {
    fn warning_at(self, now: u64) -> u8 {
        self.warning.saturating_sub((now.saturating_sub(self.raised) / FALLS_BACK).min(u64::from(TOP)) as u8)
    }
}

/// Each player's warning level, and the level each shrieking shrieker answers with.
#[derive(Resource, Default)]
struct Warnings {
    trackers: HashMap<Entity, Tracker>,
    answers: HashMap<(Entity, BlockPos), u8>,
    /// Those whose darkness came, answering the tick after, with the spot of a warden called up.
    darkened: HashMap<(Entity, BlockPos), (u8, Option<[f32; 3]>)>,
}

fn feet(world: &World, player: Entity) -> Option<[f32; 3]> {
    world.get::<Position>(player).map(|position| position.feet)
}

fn within(a: [f32; 3], b: [f32; 3], reach: f32) -> bool {
    squared(a, b) <= reach * reach
}

fn squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>()
}

fn players_near(world: &World, layer: Entity, at: [f32; 3], reach: f32) -> Vec<Entity> {
    in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && feet(world, *player).is_some_and(|feet| within(feet, at, reach))).collect()
}

/// `player` (or, for a sensor's, the player nearest) set off the shrieker at `pos` (S19-S20).
pub(super) fn shriek(world: &mut World, layer: Entity, pos: BlockPos, player: Option<Entity>) {
    let Some((_, properties, _)) = name_at(world, layer, pos) else { return };
    if number(properties, "active") != Some(0) {
        return;
    }
    let peaceful = world.resource::<crate::components::Settings>().0.difficulty == sapling_api::Difficulty::Peaceful;
    if number(properties, "can_summon") == Some(1) && !peaceful && !warn(world, layer, pos, player) {
        return;
    }
    set_number(world, layer, pos, "active", 1);
    queued::after(world, layer, pos, SHRIEK, Job::Sculk);
    let [x, y, z] = pos;
    let shrieked = catalyst::generic(PARTICLES, vec![catalyst::int("originX", x), catalyst::int("originY", y), catalyst::int("originZ", z)]);
    let heard = send::noise(SHRIEK_SOUND, Vec3f { x: x as f32, y: y as f32, z: z as f32 }, -1);
    send::to_viewers(world, layer, pos, &[shrieked]);
    crate::late_sounds::play_later(world, 1, layer, pos, heard);
}

/// Raises the warning of the players near a shrieker that can summon: false where it may not.
fn warn(world: &mut World, layer: Entity, pos: BlockPos, player: Option<Entity>) -> bool {
    let centre = pos.map(|c| c as f32 + 0.5);
    if crate::creatures::warden_near(world, layer, centre, WARDEN_NEAR) {
        return false;
    }
    let now = world.resource::<Tick>().0;
    let mut near = players_near(world, layer, centre, SHARED);
    let far = |player: Entity| feet(world, player).map_or(f32::MAX, |at| squared(at, centre));
    near.sort_by(|a, b| far(*a).total_cmp(&far(*b)));
    if let Some(player) = player.filter(|player| !near.contains(player)) {
        near.push(player);
    }
    if near.is_empty() {
        return false;
    }
    let mut warnings = world.get_resource_or_insert_with(Warnings::default);
    let highest = near.iter().map(|player| warnings.trackers.get(player).copied().unwrap_or_default()).max_by_key(|tracker| tracker.warning_at(now)).unwrap_or_default();
    if highest.warning > 0 && now < highest.raised + COOLDOWN {
        return false;
    }
    let raised = Tracker { warning: (highest.warning_at(now) + 1).min(TOP), raised: now };
    for player in near {
        warnings.trackers.insert(player, raised);
    }
    warnings.answers.insert((layer, pos), raised.warning);
    true
}

/// The shriek at `pos` ended: darkness, and a tick later it falls quiet with the warning or a
/// warden, as BDS showed all five ends in bds-t2warden.txt; one that answers nothing at once.
pub(super) fn ended(world: &mut World, layer: Entity, pos: BlockPos) {
    let (darkened, answering) = {
        let mut warnings = world.get_resource_or_insert_with(Warnings::default);
        let darkened = warnings.darkened.remove(&(layer, pos));
        let answering = darkened.is_none().then(|| warnings.answers.remove(&(layer, pos))).flatten();
        (darkened, answering)
    };
    let Some(warning) = answering else {
        set_number(world, layer, pos, "active", 0);
        return darkened.into_iter().for_each(|(warning, spot)| answer(world, layer, pos, warning, spot));
    };
    queued::after(world, layer, pos, 1, Job::Sculk);
    let centre = pos.map(|c| c as f32 + 0.5);
    for player in players_near(world, layer, centre, DARKNESS_REACH) {
        if vitals::is_mortal(world, player) {
            vitals::give_effect(world, player, Effect::Darkness, DARKNESS, 0, false);
        }
    }
    // BDS's warden comes at the darkness, casting its own, and is shown the tick after.
    let spot = (warning >= TOP).then(|| warden_spot(world, layer, pos)).flatten();
    if let Some(spot) = spot {
        crate::creatures::warden_darkens(world, layer, spot);
    }
    world.get_resource_or_insert_with(Warnings::default).darkened.insert((layer, pos), (warning, spot));
}

fn answer(world: &mut World, layer: Entity, pos: BlockPos, warning: u8, spot: Option<[f32; 3]>) {
    if warning >= TOP {
        if let Some(spot) = spot {
            crate::creatures::summon_warden(world, layer, spot);
        }
        return;
    }
    let Some(name) = WARNINGS.get(usize::from(warning.saturating_sub(1))) else { return };
    let offset = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        [0; 3].map(|_| dice.roll((2 * SOUND_SPREAD + 1) as u32) as i32 - SOUND_SPREAD)
    };
    let Some(level) = level(world, layer) else { return };
    let at = Vec3f { x: (pos[0] + offset[0]) as f32, y: (pos[1] + offset[1]) as f32, z: (pos[2] + offset[2]) as f32 };
    let sound = send::sound_at(name, level.registry(), at, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[sound]);
}

/// A spot within 5 across and 6 up or down with ground under it and room for a warden.
fn warden_spot(world: &mut World, layer: Entity, pos: BlockPos) -> Option<[f32; 3]> {
    for _ in 0..SEARCH_TRIES {
        let [dx, dz] = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            [0; 2].map(|_| dice.roll((2 * SEARCH[0] + 1) as u32) as i32 - SEARCH[0])
        };
        let level = level(world, layer)?;
        let (x, z) = (pos[0] + dx, pos[2] + dz);
        let solid = |y: i32| level.in_height(y) && crate::creatures::is_full_block(level, [x, y, z]);
        if let Some(y) = (pos[1] - SEARCH[1]..=pos[1] + SEARCH[1]).rev().find(|y| solid(y - 1) && !(0..3).any(|up| solid(y + up))) {
            return Some([x as f32 + 0.5, y as f32, z as f32 + 0.5]);
        }
    }
    None
}

/// Players on the ground standing on a shrieker set it off (`onStandOn`).
pub(super) fn stood_on(world: &mut World) {
    let standing: Vec<(Entity, Entity, BlockPos)> = in_world(world)
        .into_iter()
        .filter_map(|player| {
            let position = world.get::<Position>(player).filter(|position| position.on_ground)?;
            Some((player, world.get::<InLayer>(player)?.0, [position.feet[0].floor() as i32, (position.feet[1] - 0.5).floor() as i32, position.feet[2].floor() as i32]))
        })
        .filter(|(_, layer, under)| name_at(world, *layer, *under).is_some_and(|(name, ..)| name == SHRIEKER))
        .collect();
    for (player, layer, under) in standing {
        shriek(world, layer, under, Some(player));
    }
}
