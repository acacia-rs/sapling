//! What a dispenser does with each kind of item (docs/research/dispensers.md, D9-D20); anything not
//! named here is thrown out. Shears, minecarts and boats are not dispensed (no sheep shearing or
//! vehicles here).

use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face};
use sapling_world::ItemStack;

use super::super::liquid::{self, Cell, Liquid};
use super::super::{GameEvent, bonemeal, fire, level, send, vibrate_at};
use super::{Dispensed, click, launched, mouth};
use crate::components::{InLayer, Position};
use crate::inventory::{self, Inventory, Place};

const BUCKET: &str = "minecraft:bucket";
const POWDER_SNOW: &str = "minecraft:powder_snow";
const POWDER_SNOW_BUCKET: &str = "minecraft:powder_snow_bucket";
const FLINT_AND_STEEL_LASTS: u16 = 65;
const ARMOUR_PIECES: [&str; 4] = ["_helmet", "_chestplate", "_leggings", "_boots"];
/// A player's box, for armour put on whoever stands in front.
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

fn item(name: &str) -> ItemStack {
    ItemStack { name: name.to_owned(), count: 1, damage: 0, block: None, tag: None }
}

/// What the dispenser at `pos`, facing `face`, does with one of `stack`.
pub(super) fn dispense(world: &mut World, layer: Entity, pos: BlockPos, face: Face, stack: &ItemStack) -> Dispensed {
    let front = face.neighbour(pos);
    let name = stack.name.as_str();
    let done = match name {
        _ if shoot(world, layer, pos, face, stack) => return Dispensed::Spent,
        "minecraft:water_bucket" => pour(world, layer, front, Liquid::Water),
        "minecraft:lava_bucket" => pour(world, layer, front, Liquid::Lava),
        BUCKET => fill(world, layer, front),
        POWDER_SNOW_BUCKET => match is_air(world, layer, front) && put(world, layer, front, POWDER_SNOW) {
            true => Dispensed::Becomes(item(BUCKET)),
            false => Dispensed::Thrown,
        },
        "minecraft:bone_meal" => {
            let fed = bonemeal::feed(world, layer, front, false);
            Dispensed::or_fails(world, layer, pos, fed, Dispensed::Spent)
        }
        "minecraft:flint_and_steel" => {
            let lit = crate::tnt::light(world, layer, front) || fire::light(world, layer, front);
            Dispensed::or_fails(world, layer, pos, lit, Dispensed::Worn(FLINT_AND_STEEL_LASTS))
        }
        crate::tnt::KIND if crate::rules::on(world, layer, crate::tnt::EXPLODES) => {
            let [x, y, z] = front.map(|c| c as f32);
            crate::tnt::prime(world, layer, [x + 0.5, y, z + 0.5], crate::tnt::FUSE);
            launched(world, layer, pos, face, LevelEventEvent::SoundIgnite);
            vibrate_at(world, layer, front, GameEvent::PrimeFuse, false);
            Dispensed::Spent
        }
        _ if crate::spawning::hatch(world, layer, name, front) => Dispensed::Spent,
        _ if crate::containers::is_shulker_box(name) => return place_shulker_box(world, layer, front, face, stack),
        _ if ARMOUR_PIECES.iter().any(|piece| name.ends_with(piece)) || name == "minecraft:turtle_helmet" || name == "minecraft:elytra" => equip(world, layer, front, stack),
        _ => Dispensed::Thrown,
    };
    if matches!(done, Dispensed::Spent | Dispensed::Becomes(_) | Dispensed::Worn(_)) {
        click(world, layer, pos, face);
    }
    done
}

/// Shoots one of `stack` if it is a shot, with its launch sound; arrows click as well (D9 table).
fn shoot(world: &mut World, layer: Entity, pos: BlockPos, face: Face, stack: &ItemStack) -> bool {
    if !crate::shots::dispensed::shoot(world, layer, mouth(pos, face), face.offset().map(|c| c as f32), stack) {
        return false;
    }
    let name = stack.name.as_str();
    let event = if name == "minecraft:fire_charge" { LevelEventEvent::SoundGhastShoot } else { LevelEventEvent::SoundShoot };
    launched(world, layer, pos, face, event);
    if name.ends_with("arrow") {
        click(world, layer, pos, face);
    }
    vibrate_at(world, layer, pos, GameEvent::ProjectileShoot, false);
    true
}

fn is_air(world: &World, layer: Entity, pos: BlockPos) -> bool {
    liquid::cell(world, layer, pos) == Cell::Air
}

fn put(world: &mut World, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let Some(id) = level(world, layer).and_then(|level| send::resolve(level.registry(), &Block::new(name))) else { return false };
    send::set(world, layer, pos, id)
}

/// A full bucket empties into air or a flowing liquid in front; otherwise it is thrown out.
fn pour(world: &mut World, layer: Entity, front: BlockPos, liquid: Liquid) -> Dispensed {
    let open = match liquid::cell(world, layer, front) {
        Cell::Air => true,
        Cell::Liquid(_, depth) => depth > 0,
        _ => false,
    };
    if !(open && liquid::pour(world, layer, front, liquid, false)) {
        return Dispensed::Thrown;
    }
    vibrate_at(world, layer, front, GameEvent::FluidPlace, false);
    Dispensed::Becomes(item(BUCKET))
}

/// An empty bucket takes a liquid source or powder snow in front; otherwise it is thrown out.
fn fill(world: &mut World, layer: Entity, front: BlockPos) -> Dispensed {
    if let Cell::Liquid(liquid, 0) = liquid::cell(world, layer, front) {
        liquid::take(world, layer, front);
        vibrate_at(world, layer, front, GameEvent::FluidPickup, false);
        let full = if liquid == Liquid::Water { "minecraft:water_bucket" } else { "minecraft:lava_bucket" };
        return Dispensed::Becomes(item(full));
    }
    let is_snow = level(world, layer).and_then(|level| level.registry().get(level.block(front[0], front[1], front[2]))).is_some_and(|state| state.name == POWDER_SNOW);
    match is_snow && put(world, layer, front, "minecraft:air") {
        true => Dispensed::Becomes(item(POWDER_SNOW_BUCKET)),
        false => Dispensed::Thrown,
    }
}

/// A shulker box is placed into air or water in front, opening away from the dispenser, without a
/// sound; a blocked one stays (D20).
fn place_shulker_box(world: &mut World, layer: Entity, front: BlockPos, face: Face, stack: &ItemStack) -> Dispensed {
    let free = matches!(liquid::cell(world, layer, front), Cell::Air | Cell::Liquid(Liquid::Water, _));
    let Some(id) = stack.block.filter(|_| free) else { return Dispensed::Stays };
    if !send::set(world, layer, front, id) {
        return Dispensed::Stays;
    }
    crate::containers::furnish(world, layer, front, &stack.name, stack.tag.as_ref(), face);
    Dispensed::Spent
}

/// Armour goes onto a player standing in front whose slot for it is empty; else it is thrown out.
fn equip(world: &mut World, layer: Entity, front: BlockPos, stack: &ItemStack) -> Dispensed {
    let slot = match stack.name.as_str() {
        "minecraft:turtle_helmet" => 0,
        "minecraft:elytra" => 1,
        name => ARMOUR_PIECES.iter().position(|piece| name.ends_with(piece)).unwrap_or(0),
    };
    let [x, y, z] = front.map(|c| c as f32);
    let [half, tall] = [PLAYER_SIZE[0] / 2.0, PLAYER_SIZE[1]];
    let touches = |feet: [f32; 3]| feet[0] + half > x && feet[0] - half < x + 1.0 && feet[1] + tall > y && feet[1] < y + 1.0 && feet[2] + half > z && feet[2] - half < z + 1.0;
    let mut players = world.query_filtered::<(Entity, &Position, &InLayer, &Inventory), ()>();
    let wearer = players.iter(world).find(|(_, position, at, inventory)| at.0 == layer && touches(position.feet) && inventory.slot(Place::Armor(slot)).is_some_and(Option::is_none)).map(|(player, ..)| player);
    let Some(player) = wearer else { return Dispensed::Thrown };
    inventory::change(world, player, |inventory| inventory.put(Place::Armor(slot), Some(ItemStack { count: 1, ..stack.clone() })));
    Dispensed::Spent
}
