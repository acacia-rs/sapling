//! A turtle's nesting (ED `turtle.json`; docs/research/leftovers.md "Turtles"): bred, it becomes
//! pregnant instead of having a baby (`causes_pregnancy`, group `minecraft:pregnant`, flag
//! `PREGNANT`). `behavior.go_home` sets off one tick in 700 and walks it home until within 4; then
//! (`minecraft:wants_to_lay_egg`) `behavior.lay_egg` finds open sand within 16 across and 4 up or
//! down, walks to within 1.5 of it, digs 10 s (`LAYING_EGG`) and lays 1 to 4 eggs there, heard as
//! `lay_egg`. Its home is the cell it hatched in, or the one it was first seen in (`minecraft:home`).

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Swap;

use super::{Raised, regroup, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::entities::Stored;

const TURTLE: &str = "minecraft:turtle";
const PREGNANT: &str = "minecraft:pregnant";
const WANTS_TO_LAY: &str = "minecraft:wants_to_lay_egg";
const HOME_ODDS: u32 = 700;
const HOME_NEAR: f32 = 4.0;
const ACROSS: i32 = 16;
const UP: i32 = 4;
const NEST_NEAR: f32 = 1.5;
const DIGS_FOR: u64 = 200;
const MOST_EGGS: u32 = 4;
/// Ticks between looks for sand while it wants to lay (not in the pack).
const LOOKS_EVERY: u64 = 20;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum Nesting {
    /// Pregnant, and on its way home once it set off.
    Pregnant { homeward: bool },
    /// Home: the sand it found to lay on, and the tick its digging there ends.
    Laying { nest: Option<BlockPos>, until: Option<u64> },
}

/// The cell a turtle calls home.
#[derive(Component, Clone, Copy)]
struct Home(BlockPos);

fn middle(cell: BlockPos) -> [f32; 3] {
    [cell[0] as f32 + 0.5, cell[1] as f32, cell[2] as f32 + 0.5]
}

fn across(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

fn swap(removed: &[&'static str], added: &[&'static str]) -> Swap {
    Swap { removed: removed.to_vec(), added: added.to_vec() }
}

pub(super) fn is_nesting(world: &World, entity: Entity) -> bool {
    world.get::<Nesting>(entity).is_some()
}

/// A bred turtle becomes pregnant; false for a mob of another kind.
pub(super) fn conceives(world: &mut World, entity: Entity, kind: &str) -> bool {
    if kind != TURTLE {
        return false;
    }
    world.entity_mut(entity).insert(Nesting::Pregnant { homeward: false });
    shown::flag(world, entity, Flags::PREGNANT, true);
    regroup(world, entity, &swap(&[], &[PREGNANT]));
    true
}

/// A stored turtle carries on nesting as its kept component groups say.
pub(super) fn adopt(world: &mut World, entity: Entity, kind: &str, compound: &[u8]) {
    if kind != TURTLE {
        return;
    }
    let groups = sapling_world::actor_groups(compound);
    let nesting = match () {
        _ if groups.iter().any(|group| group == WANTS_TO_LAY) => Nesting::Laying { nest: None, until: None },
        _ if groups.iter().any(|group| group == PREGNANT) => Nesting::Pregnant { homeward: false },
        _ => return,
    };
    world.entity_mut(entity).insert(nesting);
    shown::flag(world, entity, Flags::PREGNANT, true);
}

/// The turtle's home: kept in its compound, or made where it is now.
fn home(world: &mut World, entity: Entity, body: &Body) -> BlockPos {
    if let Some(Home(home)) = world.get::<Home>(entity) {
        return *home;
    }
    let kept = world.get::<Stored>(entity).and_then(|stored| sapling_world::actor_home(&stored.compound)).map(|(at, _)| at.map(|c| c.floor() as i32));
    let home = kept.unwrap_or_else(|| body.at.map(|c| c.floor() as i32));
    if kept.is_none() {
        crate::creatures::make_home(world, entity, home);
    }
    world.entity_mut(entity).insert(Home(home));
    home
}

/// A tick of a turtle: it learns its home, and a nesting one sets off, arrives, finds sand,
/// digs and lays.
pub(super) fn tick(world: &mut World, entity: Entity, kind: &str, now: u64) {
    if kind != TURTLE {
        return;
    }
    let Some((layer, _, _, body)) = shown::of(world, entity).filter(|(layer, ..)| crate::rules::on(world, *layer, sapling_data::MOB_AI)) else { return };
    let home = home(world, entity, &body);
    let Some(nesting) = world.get::<Nesting>(entity).copied().filter(|_| world.get::<Raised>(entity).is_some_and(|raised| !raised.is_baby())) else { return };
    let next = match nesting {
        Nesting::Pregnant { homeward: false } if world.get_resource_or_insert_with(Dice::default).roll(HOME_ODDS) == 0 => Nesting::Pregnant { homeward: true },
        Nesting::Pregnant { homeward: true } if across(body.at, middle(home)) <= HOME_NEAR => {
            regroup(world, entity, &swap(&[PREGNANT], &[WANTS_TO_LAY]));
            Nesting::Laying { nest: None, until: None }
        }
        Nesting::Laying { nest: None, .. } if now.is_multiple_of(LOOKS_EVERY) => Nesting::Laying { nest: crate::blocks::turtle_nest_near(world, layer, body.at, ACROSS, UP), until: None },
        Nesting::Laying { nest: Some(nest), until: None } if across(body.at, middle(nest)) <= NEST_NEAR => {
            shown::flag(world, entity, Flags::LAYING_EGG, true);
            Nesting::Laying { nest: Some(nest), until: Some(now + DIGS_FOR) }
        }
        Nesting::Laying { nest: Some(nest), until: Some(until) } if now >= until => return lay(world, entity, layer, nest),
        unchanged => unchanged,
    };
    world.entity_mut(entity).insert(next);
}

fn lay(world: &mut World, entity: Entity, layer: Entity, nest: BlockPos) {
    let eggs = 1 + world.get_resource_or_insert_with(Dice::default).roll(MOST_EGGS) as usize;
    if crate::blocks::lay_turtle_eggs(world, layer, nest, eggs) {
        shown::sound(world, entity, "lay_egg");
    }
    world.entity_mut(entity).remove::<Nesting>();
    shown::flag(world, entity, Flags::LAYING_EGG, false);
    shown::flag(world, entity, Flags::PREGNANT, false);
    regroup(world, entity, &swap(&[WANTS_TO_LAY], &[]));
}

/// Before a nesting turtle's move: it walks home or to its sand, and stands while it digs. False
/// when nesting does not move it now.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(nesting) = world.get::<Nesting>(entity).copied() else { return false };
    let (Some(body), Some(Home(home))) = (world.get::<Body>(entity).copied(), world.get::<Home>(entity).copied()) else { return false };
    let to = match nesting {
        Nesting::Pregnant { homeward: true } => (middle(home), HOME_NEAR),
        Nesting::Laying { nest: Some(nest), until: None } => (middle(nest), NEST_NEAR),
        Nesting::Laying { until: Some(_), .. } => {
            super::super::legs::rest(world, entity);
            return true;
        }
        _ => return false,
    };
    let pace = sapling_data::husbandry(kind).map_or(0.1, |rules| rules.walks);
    super::super::legs::walk(world, entity, layer, &body, [to.0[0], to.0[2]], pace, to.1 / 2.0);
    true
}
