//! A frog's laying (`frog.json` group `pregnant`, `behavior.lay_egg`), as BDS's laid in
//! research/captures/bds-g14frog.txt: bred, it becomes `PREGNANT` instead of having a tadpole, looks for
//! an open cell over water within 10 across and 3 up or down, walks to it at its pace, and within 1.7
//! of it shows `LAYING_EGG` for 41 ticks while it walks on; then both flags go, and the next tick
//! frogspawn is set in the cell, heard `lay_spawn` from the frog's head and from the cell (eggs/frogspawn.rs).
// BDS's frog began laying 2.2 from the cell it laid in, past `goal_radius`; which cell it aimed at was not seen.

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Swap;

use super::{Raised, regroup, shown};
use crate::body::Body;

const FROG: &str = "minecraft:frog";
const PREGNANT: &str = "pregnant";
const ACROSS: i32 = 10;
const UP: i32 = 3;
const NEAR: f32 = 1.7;
const LAYS_FOR: u64 = 41;
/// Ticks between looks for water while it has none (not in the pack).
const LOOKS_EVERY: u64 = 20;
/// Its `lay_spawn` comes from this much of its height up, as BDS's did (0.4675 of 0.55).
const HEAD: f32 = 0.85;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum Spawning {
    /// Pregnant: the cell it lays in once found, and the tick its laying ends once begun.
    Pregnant { site: Option<BlockPos>, until: Option<u64> },
    /// Done laying: frogspawn comes at the site the next tick.
    Laid(BlockPos),
}

pub(super) fn is_spawning(world: &World, entity: Entity) -> bool {
    world.get::<Spawning>(entity).is_some()
}

fn pregnant(world: &mut World, entity: Entity) {
    world.entity_mut(entity).insert(Spawning::Pregnant { site: None, until: None });
    shown::flag(world, entity, Flags::PREGNANT, true);
}

/// A bred frog becomes pregnant; false for a mob of another kind.
pub(super) fn conceives(world: &mut World, entity: Entity, kind: &str) -> bool {
    if kind != FROG {
        return false;
    }
    pregnant(world, entity);
    regroup(world, entity, &Swap { removed: Vec::new(), added: vec![PREGNANT] });
    true
}

/// A stored frog keeps on being pregnant as its kept component groups say.
pub(super) fn adopt(world: &mut World, entity: Entity, kind: &str, compound: &[u8]) {
    if kind == FROG && sapling_world::actor_groups(compound).iter().any(|group| group == PREGNANT) {
        pregnant(world, entity);
    }
}

fn across(a: [f32; 3], [x, _, z]: BlockPos) -> f32 {
    (a[0] - (x as f32 + 0.5)).hypot(a[2] - (z as f32 + 0.5))
}

/// A tick of a pregnant frog: it finds water, lays when near it, and the spawn comes the tick after.
pub(super) fn tick(world: &mut World, entity: Entity, now: u64) {
    let Some(spawning) = world.get::<Spawning>(entity).copied().filter(|_| world.get::<Raised>(entity).is_some_and(|raised| !raised.is_baby())) else { return };
    let Some((layer, _, _, body)) = shown::of(world, entity).filter(|(layer, ..)| crate::rules::on(world, *layer, sapling_data::MOB_AI)) else { return };
    let next = match spawning {
        Spawning::Pregnant { site: None, .. } if now.is_multiple_of(LOOKS_EVERY) => Spawning::Pregnant { site: crate::blocks::frog_spawn_site_near(world, layer, body.at, ACROSS, UP), until: None },
        Spawning::Pregnant { site: Some(site), until: None } if across(body.at, site) <= NEAR => {
            shown::flag(world, entity, Flags::LAYING_EGG, true);
            Spawning::Pregnant { site: Some(site), until: Some(now + LAYS_FOR) }
        }
        Spawning::Pregnant { site: Some(site), until: Some(until) } if now >= until => {
            shown::flag(world, entity, Flags::LAYING_EGG, false);
            shown::flag(world, entity, Flags::PREGNANT, false);
            Spawning::Laid(site)
        }
        Spawning::Laid(site) => return lay(world, entity, layer, site),
        unchanged => unchanged,
    };
    world.entity_mut(entity).insert(next);
}

fn lay(world: &mut World, entity: Entity, layer: Entity, site: BlockPos) {
    if crate::blocks::lay_frog_spawn(world, layer, site) {
        shown::sound_from(world, entity, "lay_spawn", HEAD);
    }
    world.entity_mut(entity).remove::<Spawning>();
    regroup(world, entity, &Swap { removed: vec![PREGNANT], added: Vec::new() });
}

/// Before a pregnant frog's move: it walks to its water, laying or not. False while it has found none.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(Spawning::Pregnant { site: Some(site), .. }) = world.get::<Spawning>(entity).copied() else { return false };
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let pace = sapling_data::husbandry(kind).map_or(0.1, |rules| rules.walks);
    super::super::legs::walk(world, entity, layer, &body, [site[0] as f32 + 0.5, site[2] as f32 + 0.5], pace, 0.0);
    true
}
