//! A stalactite hanging from a dripstone block under a water source grows: its tip a piece
//! longer, or a stalagmite under it (`PointedDripstoneBlock` 14660e6b0 and its helpers, as
//! decompiled; much as the Java game's `growStalactiteOrStalagmiteIfPossible`).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::{ground, level, send, updates};
use super::{DRIPSTONE, Level, source_over, state};
use crate::body::Liquid;

/// The share of a root's random ticks that grow, drawn before its drip.
pub(super) const ODDS: f32 = 0.011_377_778;
/// The longest stalactite that grows (the block's constructor's 7), and how far under its tip a
/// stalagmite may start.
const LONGEST: i32 = 7;
const BELOW_TIP: i32 = 10;

#[derive(Clone, Copy, PartialEq)]
enum Way {
    Down,
    Up,
}

impl Way {
    fn step(self) -> i32 {
        if self == Way::Down { -1 } else { 1 }
    }

    fn hanging(self) -> &'static str {
        if self == Way::Down { "1" } else { "0" }
    }
}

fn at([x, y, z]: BlockPos, dy: i32) -> BlockPos {
    [x, y + dy, z]
}

fn property(properties: &'static str, key: &str) -> Option<&'static str> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value)
}

/// Whether `pos` holds a piece pointing `way` that is its tip, unmerged.
fn is_tip(level: &Level, pos: BlockPos, way: Way) -> bool {
    state(level, pos).is_some_and(|(name, properties)| name == DRIPSTONE && property(properties, "hanging") == Some(way.hanging()) && property(properties, "dripstone_thickness") == Some("tip"))
}

fn points(level: &Level, pos: BlockPos, way: Way) -> bool {
    state(level, pos).is_some_and(|(name, properties)| name == DRIPSTONE && property(properties, "hanging") == Some(way.hanging()))
}

fn is_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && [level.block(x, y, z), level.liquid(x, y, z)].into_iter().any(|id| level.registry().get(id).is_some_and(|state| Liquid::of(state.name) == Some(Liquid::Water)))
}

fn is_air(level: &Level, pos: BlockPos) -> bool {
    state(level, pos).is_some_and(|(name, _)| name == "minecraft:air")
}

/// Whether a tip pointing `way` at `tip` has room to grow: air past it, or a tip meeting it.
fn has_room(level: &Level, tip: BlockPos, way: Way) -> bool {
    let next = at(tip, way.step());
    !is_water(level, next) && (is_air(level, next) || is_tip(level, next, if way == Way::Down { Way::Up } else { Way::Down }))
}

/// The tip of the stalactite whose top piece is `root`, within [`LONGEST`].
fn tip_under(level: &Level, root: BlockPos) -> Option<BlockPos> {
    (0..LONGEST).map(|down| at(root, -down)).take_while(|pos| points(level, *pos, Way::Down)).find(|pos| is_tip(level, *pos, Way::Down))
}

/// The random tick of the stalactite whose top piece is `root`, once the growth is drawn: `under`
/// grows a stalagmite under it, else its own tip.
pub(super) fn grow_from(world: &mut World, layer: Entity, root: BlockPos, under: bool) {
    let Some(level) = level(world, layer) else { return };
    let Some(tip) = tip_under(level, root) else { return };
    let fed = source_over(level, root) == Some(Liquid::Water) && state(level, at(root, 1)).is_some_and(|(name, _)| name == "minecraft:dripstone_block");
    if !fed || is_water(level, tip) || !has_room(level, tip, Way::Down) {
        return;
    }
    if under { grow_under(world, layer, tip) } else { grow(world, layer, tip, Way::Down) }
}

/// The first cell under `tip`, within [`BELOW_TIP`], over a stalagmite or a whole top grows one.
fn grow_under(world: &mut World, layer: Entity, tip: BlockPos) {
    for down in 0..BELOW_TIP {
        let Some(level) = level(world, layer) else { return };
        let (cell, below) = (at(tip, -down), at(tip, -down - 1));
        if is_water(level, cell) {
            return;
        }
        if is_tip(level, cell, Way::Up) && has_room(level, cell, Way::Up) {
            return grow(world, layer, cell, Way::Up);
        }
        if (points(level, below, Way::Up) || ground::has_top(level, below, true)) && !is_water(level, below) {
            return grow(world, layer, below, Way::Up);
        }
    }
}

/// The tip at `pos` grows `way`: into air or water a new tip, onto a tip meeting it the two merge.
fn grow(world: &mut World, layer: Entity, pos: BlockPos, way: Way) {
    let Some(level) = level(world, layer) else { return };
    let next = at(pos, way.step());
    let meets = if way == Way::Down { Way::Up } else { Way::Down };
    if is_tip(level, next, meets) {
        let hanging = if meets == Way::Down { next } else { at(next, 1) };
        put(world, layer, hanging, Way::Down, "merge");
        put(world, layer, at(hanging, -1), Way::Up, "merge");
    } else if is_air(level, next) || is_water(level, next) {
        put(world, layer, next, way, "tip");
    }
}

fn put(world: &mut World, layer: Entity, pos: BlockPos, way: Way, thickness: &str) {
    let Some(id) = level(world, layer).and_then(|level| level.registry().find(DRIPSTONE, &format!("dripstone_thickness={thickness},hanging={}", way.hanging()))) else { return };
    if send::set(world, layer, pos, id) {
        updates::settle(world, layer, pos);
    }
}
