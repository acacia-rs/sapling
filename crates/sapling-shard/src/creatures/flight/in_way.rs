//! Who is in a flying shot's way: players, living mobs and primed TNT whose boxes its own meets.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Position, in_world};
use crate::creatures::{Creature, Dying, Health};
use crate::vitals;

const PLAYER: [f32; 2] = [0.6, 1.8];

/// The player in `layer` whose box the shot's, of `size` with its bottom middle at `at`, reaches
/// into: BDS's arrow flew over a player 8 blocks off, a hand above its head. Not `by`, which shot it.
fn met(world: &World, layer: Entity, by: Entity, at: [f32; 3], size: [f32; 2], passed: &[Entity]) -> Option<Entity> {
    in_world(world).into_iter().filter(|player| *player != by && !passed.contains(player)).find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        let across = (PLAYER[0] + size[0]) / 2.0;
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && vitals::is_mortal(world, *player) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + size[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

/// The living mob of `layer` whose box the shot's reaches into.
fn met_mob(world: &mut World, layer: Entity, by: Entity, at: [f32; 3], size: [f32; 2], passed: &[Entity]) -> Option<Entity> {
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>, Without<Dying>)>();
    mobs.iter(world).filter(|(mob, ..)| *mob != by && !passed.contains(mob)).find(|(_, in_layer, body)| in_layer.0 == layer && reaches(body, at, size)).map(|(mob, ..)| mob)
}

/// The primed TNT of `layer` whose box the shot's reaches into: BDS's arrow bounced off the one it
/// set off (research/captures/bds-g10flame.txt).
fn met_primed(world: &mut World, layer: Entity, at: [f32; 3], size: [f32; 2]) -> Option<Entity> {
    let mut lit = world.query_filtered::<(Entity, &InLayer, &Body), With<crate::tnt::Primed>>();
    lit.iter(world).find(|(_, in_layer, body)| in_layer.0 == layer && reaches(body, at, size)).map(|(tnt, ..)| tnt)
}

/// Whether a shot of `size` with its bottom middle at `at` reaches into `body`'s box.
fn reaches(body: &Body, at: [f32; 3], size: [f32; 2]) -> bool {
    let across = (body.size[0] + size[0]) / 2.0;
    (at[0] - body.at[0]).abs() < across && (at[2] - body.at[2]).abs() < across && at[1] + size[1] > body.at[1] && at[1] < body.at[1] + body.size[1]
}

/// The first player or mob (or primed TNT, for a shot that hurts: `tnt`) on the shot's way from `at`
/// by `motion`, where it met it, and whether a player.
pub(super) fn first_met(world: &mut World, layer: Entity, by: Entity, at: [f32; 3], motion: [f32; 3], size: [f32; 2], passed: &[Entity], tnt: bool) -> Option<([f32; 3], Entity, bool)> {
    let speed = motion.iter().map(|c| c * c).sum::<f32>().sqrt();
    let looks = (speed / 0.25).ceil().max(1.0) as usize;
    let on_way = |look: usize| -> [f32; 3] { std::array::from_fn(|axis| at[axis] + motion[axis] * look as f32 / looks as f32) };
    (1..=looks).find_map(|look| {
        let point = on_way(look);
        met(world, layer, by, point, size, passed).map(|player| (point, player, true)).or_else(|| met_mob(world, layer, by, point, size, passed).map(|mob| (point, mob, false))).or_else(|| tnt.then(|| met_primed(world, layer, point, size)).flatten().map(|lit| (point, lit, false)))
    })
}
