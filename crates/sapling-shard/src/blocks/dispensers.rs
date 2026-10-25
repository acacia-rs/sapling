//! Dispensers and droppers firing on a rising signal (docs/research/dispensers.md): 4 ticks after
//! it one picks a filled slot at random. A dropper puts one of it into the container it faces, or
//! throws it out; a dispenser does what the item does (dispensers/behaviours.rs) or throws it out.
//! A crafter fires the same way and crafts (crafter.rs).

mod behaviours;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ItemStack;

use super::{Dice, level, send};
use crate::{containers, drops, hoppers};

const DISPENSER: &str = "minecraft:dispenser";
const DROPPER: &str = "minecraft:dropper";
const CRAFTER: &str = "minecraft:crafter";
/// How far out of the block things leave it, from 0.3 up its middle (D4).
const OUT: f32 = 0.7;
const UP: f32 = 0.3;
/// How much lower a thrown item starts out of a side, and out of the top or bottom (D5).
const SIDEWAYS_DROP: f32 = 0.15625;
const VERTICAL_DROP: f32 = 0.125;
/// A thrown item's speed along the face, the part of it drawn, its rise, and its spread (D5).
const THROW_SPEED: f32 = 0.2;
const THROW_DRAWN: f32 = 0.1;
const THROW_UP: f32 = 0.2;
const THROW_SPREAD: f32 = 0.045;

/// What an item's own behaviour did; all but `Thrown` and `Becomes` leave the slot to it.
#[derive(Debug, PartialEq)]
pub(super) enum Dispensed {
    /// It has none here: one is thrown out.
    Thrown,
    /// One was used.
    Spent,
    /// It wore by one, of the uses it lasts (flint and steel).
    Worn(u16),
    /// The one used became this (a bucket filled or emptied).
    Becomes(ItemStack),
    /// It did nothing and stays, though the dispenser fired (smoke and all).
    Stays,
}

impl Dispensed {
    /// `done` if the behaviour worked, else the failing click and the item stays (D13-D14).
    fn or_fails(world: &mut World, layer: Entity, pos: BlockPos, worked: bool, done: Self) -> Self {
        if worked {
            return done;
        }
        click_fails(world, layer, pos);
        Self::Stays
    }
}

/// The way the block at `pos` faces, and its name.
fn facing(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&'static str, Face)> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    let state = level.registry().get(level.block(x, y, z))?;
    let face = state.properties.split(',').find_map(|pair| pair.strip_prefix("facing_direction="))?.parse().ok().and_then(Face::from_index)?;
    Some((state.name, face))
}

/// Where something leaves the block at `pos` through `face`.
pub(super) fn mouth(pos: BlockPos, face: Face) -> [f32; 3] {
    let offset = face.offset();
    let base = [0.5, UP, 0.5];
    std::array::from_fn(|axis| pos[axis] as f32 + base[axis] + offset[axis] as f32 * OUT)
}

fn told(world: &mut World, layer: Entity, pos: BlockPos, event: LevelEventEvent, [x, y, z]: [f32; 3], data: i32) {
    let told = LevelEvent { event, position: Vec3f { x, y, z }, data };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&told)]);
}

/// The sound `name` of the block at `pos`, from `at`.
pub(super) fn sound(world: &mut World, layer: Entity, pos: BlockPos, name: &str, [x, y, z]: [f32; 3]) {
    let Some(level) = level(world, layer) else { return };
    let noise = send::sound_at(name, level.registry(), Vec3f { x, y, z }, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[noise]);
}

/// The click from the mouth of the block at `pos`, of the block it opens into (D6).
pub(super) fn click(world: &mut World, layer: Entity, pos: BlockPos, face: Face) {
    let at = mouth(pos, face);
    sound(world, layer, at.map(|c| c.floor() as i32), "block.click", at);
}

/// The failing click, from the block's corner as BDS sent it (research/captures/bds-conductors.txt).
pub(super) fn click_fails(world: &mut World, layer: Entity, pos: BlockPos) {
    sound(world, layer, pos, "block.click.fail", pos.map(|c| c as f32));
}

/// `event` out of the mouth, with the face's way as BDS numbers it for smoke (D8).
pub(super) fn smoke(world: &mut World, layer: Entity, pos: BlockPos, face: Face, event: LevelEventEvent) {
    let [sx, _, sz] = face.offset();
    told(world, layer, pos, event, mouth(pos, face), (sx + 1) + 3 * (sz + 1));
}

/// A shot's launch sound, or a TNT's fuse, out of the mouth (D9 table, D19).
pub(super) fn launched(world: &mut World, layer: Entity, pos: BlockPos, face: Face, event: LevelEventEvent) {
    const LAUNCH_DATA: i32 = 78642;
    told(world, layer, pos, event, mouth(pos, face), LAUNCH_DATA);
}

/// Throws `stack` out of the block at `pos` through `face` (D5).
pub(super) fn spit(world: &mut World, layer: Entity, pos: BlockPos, face: Face, stack: ItemStack) {
    let mut at = mouth(pos, face);
    at[1] -= if face == Face::Up || face == Face::Down { VERTICAL_DROP } else { SIDEWAYS_DROP };
    let offset = face.offset().map(|c| c as f32);
    let motion = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let along = dice.fraction() * THROW_DRAWN + THROW_SPEED;
        let base = [offset[0] * along, THROW_UP, offset[2] * along];
        base.map(|c| c + dice.gaussian() * THROW_SPREAD)
    };
    drops::eject(world, layer, stack, at, motion);
}

/// A filled slot of the container at `pos` picked at random, and what it holds (D3).
fn pick(world: &mut World, layer: Entity, pos: BlockPos) -> Option<(usize, ItemStack)> {
    let filled: Vec<(usize, ItemStack)> = containers::contents(world, layer, pos)?.into_iter().enumerate().filter_map(|(slot, held)| Some((slot, held?.stack))).collect();
    if filled.is_empty() {
        return None;
    }
    let picked = world.get_resource_or_insert_with(Dice::default).roll(filled.len() as u32) as usize;
    filled.into_iter().nth(picked)
}

/// Takes one out of `slot` of the container at `pos`, putting `becomes` in its place if given:
/// where the slot still holds more, `becomes` goes to the first empty slot or is thrown out (D12).
fn use_one(world: &mut World, layer: Entity, pos: BlockPos, face: Face, slot: usize, becomes: Option<ItemStack>) {
    let left = containers::with_slots(world, layer, pos, |world, slots| {
        let Some(held) = slots.get_mut(slot).and_then(Option::as_mut) else { return becomes };
        if held.stack.count == 1 {
            slots[slot] = becomes.map(|stack| containers::new_held(world, stack));
            return None;
        }
        held.stack.count -= 1;
        let becomes = becomes?;
        let Some(free) = slots.iter().position(Option::is_none) else { return Some(becomes) };
        slots[free] = Some(containers::new_held(world, becomes));
        None
    });
    if let Some(Some(stack)) = left {
        spit(world, layer, pos, face, stack);
    }
    super::container_changed(world, layer, pos);
}

/// Wears the tool in `slot`, which lasts `lasts` uses, by one; a worn-out one breaks.
fn wear(world: &mut World, layer: Entity, pos: BlockPos, slot: usize, lasts: u16) {
    containers::with_slots(world, layer, pos, |_, slots| {
        let Some(held) = slots.get_mut(slot).and_then(Option::as_mut) else { return };
        held.stack.damage += 1;
        if held.stack.damage as u16 >= lasts {
            slots[slot] = None;
        }
    });
}

/// The block at `pos` fires, if it is a dispenser, dropper or crafter.
pub(super) fn fire(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((name, face)) = facing(world, layer, pos).or_else(|| super::crafter::facing(world, layer, pos).map(|face| (CRAFTER, face))) else { return };
    if name == CRAFTER {
        return super::crafter::craft(world, layer, pos, face);
    }
    if name != DISPENSER && name != DROPPER {
        return;
    }
    let Some((slot, stack)) = pick(world, layer, pos) else {
        click_fails(world, layer, pos);
        if name == DISPENSER {
            super::vibrate_at(world, layer, pos, super::GameEvent::DispenseFail, false);
        }
        return;
    };
    if name == DROPPER {
        return drop_one(world, layer, pos, face, slot, stack);
    }
    match behaviours::dispense(world, layer, pos, face, &stack) {
        Dispensed::Thrown => {
            spit(world, layer, pos, face, ItemStack { count: 1, ..stack });
            click(world, layer, pos, face);
            use_one(world, layer, pos, face, slot, None);
        }
        Dispensed::Spent => use_one(world, layer, pos, face, slot, None),
        Dispensed::Becomes(into) => use_one(world, layer, pos, face, slot, Some(into)),
        Dispensed::Worn(lasts) => wear(world, layer, pos, slot, lasts),
        Dispensed::Stays => {}
    }
    smoke(world, layer, pos, face, LevelEventEvent::ParticleShoot);
}

/// A dropper puts one into the container it faces, nothing when that is full; else it throws one
/// out with a click from its mouth and one from itself, and no smoke (D10-D11).
fn drop_one(world: &mut World, layer: Entity, pos: BlockPos, face: Face, slot: usize, stack: ItemStack) {
    let front = face.neighbour(pos);
    if hoppers::takes_items(world, layer, front) {
        if hoppers::put_one(world, layer, front, super::opposite(face), &stack) {
            use_one(world, layer, pos, face, slot, None);
        }
        return;
    }
    spit(world, layer, pos, face, ItemStack { count: 1, ..stack });
    click(world, layer, pos, face);
    sound(world, layer, pos, "block.click", pos.map(|c| c as f32 + 0.5));
    use_one(world, layer, pos, face, slot, None);
}
