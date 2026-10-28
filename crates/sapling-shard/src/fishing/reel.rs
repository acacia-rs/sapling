//! Reeling in, by BDS's `FishingHook::retrieve` (1429c6b50): a hooked body is pulled to the
//! fisher; a biting fish gives the hook's loot (sapling_data's `fishing_loot`, with the luck of
//! Luck of the Sea), each stack pulled to the fisher with 1 to 5 experience; a hook in the
//! ground just comes back. The rod wears by 3, 1 and 2 of those (2 whenever it lay in the ground).

use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_data::enchant::{self, Draws, Enchant};
use sapling_world::ItemStack;

use super::{Hook, bob};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, Position};
use crate::inventory::Inventory;
use crate::{drops, orbs, vitals};

const PULL: f32 = 0.1;
const LIFT: f32 = 0.08;
const ROD_LASTS: u16 = 384;
/// Experience a catch pays: 1 to this.
const MOST_EXPERIENCE: u32 = 5;
/// The fisher's head as the pull reads it: 0.1 under its eyes (three BDS catches, pulled from the
/// middle of their boxes, agree to 1e-6).
const HEAD: f32 = sapling_data::EYE_HEIGHT - 0.1;
/// A catch's experience comes out this far over the fisher's feet and half a block along its look.
const ORBS_UP: f32 = 0.7;
const ORBS_AHEAD: f32 = 0.5;
/// The middle of an item's box over its bottom.
const ITEM_MIDDLE: f32 = 0.125;

struct Rolls<'a>(&'a mut Dice);

impl Draws for Rolls<'_> {
    fn int(&mut self, bound: i32) -> i32 {
        if bound < 1 { 0 } else { self.0.roll(bound as u32) as i32 }
    }

    fn float(&mut self) -> f32 {
        self.0.fraction()
    }
}

/// The fisher's head a block up, less `at`, scaled to a tenth: the motion that pulls a thing at
/// `at` to it (`_pullCloser` 1429c69b0).
fn pull(head: [f32; 3], at: [f32; 3]) -> [f32; 3] {
    let way: [f32; 3] = std::array::from_fn(|axis| (head[axis] + if axis == 1 { 1.0 } else { 0.0 } - at[axis]) * PULL);
    let long = way.iter().map(|c| c * c).sum::<f32>().sqrt();
    [way[0], way[1] + long.sqrt() * LIFT, way[2]]
}

/// The fisher reels in its hook.
pub(super) fn retrieve(world: &mut World, fisher: Entity, hook: Entity) {
    let (Some(state), Some(at), Some(layer)) = (world.get::<Hook>(hook), world.get::<Body>(hook).map(|body| body.at), world.get::<InLayer>(hook).map(|layer| layer.0)) else { return super::remove(world, hook) };
    let (target, nibble, in_ground, jungle) = (state.target, state.nibble, state.in_ground, state.jungle);
    let Some(position) = world.get::<Position>(fisher).copied() else { return super::remove(world, hook) };
    let head = [position.feet[0], position.feet[1] + HEAD, position.feet[2]];
    let mut wear = 0;
    if let Some(target) = target.filter(|target| bob::alive(world, *target).is_some()) {
        let feet = bob::alive(world, target).map_or(at, |(feet, _)| feet);
        let motion = pull(head, feet);
        match world.get::<Position>(target) {
            Some(_) => vitals::pushed(world, target, motion),
            None => {
                if let Some(mut body) = world.get_mut::<Body>(target) {
                    body.throw(motion);
                }
            }
        }
        wear = 3;
    } else if nibble > 0 {
        catch(world, fisher, layer, at, head, jungle);
        wear = 1;
    }
    if in_ground {
        wear = 2;
    }
    super::remove(world, hook);
    if wear > 0 && world.get::<Mode>(fisher).is_none_or(|mode| mode.0 != GameMode::Creative) {
        crate::inventory::wear_hand_by(world, fisher, wear, ROD_LASTS);
    }
}

/// The hook's loot flies from `at` to the fisher, with experience at its feet.
fn catch(world: &mut World, fisher: Entity, layer: Entity, at: [f32; 3], head: [f32; 3], jungle: bool) {
    let rod = world.get::<Inventory>(fisher).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.tag.clone());
    let luck = i32::from(enchant::level(rod.flatten().as_ref(), Enchant::LuckOfTheSea));
    let mut dice = std::mem::take(&mut *world.get_resource_or_insert_with(Dice::default));
    let caught = sapling_data::fishing_loot(jungle, luck, &mut Rolls(&mut dice));
    let points: Vec<i32> = caught.iter().map(|_| dice.roll(MOST_EXPERIENCE) as i32 + 1).collect();
    *world.resource_mut::<Dice>() = dice;
    let Some(position) = world.get::<Position>(fisher).copied() else { return };
    let feet = position.feet;
    let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
    let orbs_at = [feet[0] - yaw.sin() * pitch.cos() * ORBS_AHEAD, feet[1] + ORBS_UP, feet[2] + yaw.cos() * pitch.cos() * ORBS_AHEAD];
    let middle = [at[0], at[1] + ITEM_MIDDLE, at[2]];
    for (caught, points) in caught.into_iter().zip(points) {
        let item = sapling_api::Item { damage: caught.data, ..sapling_api::Item::new(caught.name, caught.count) };
        let Some(stack) = world.get::<Layer>(layer).and_then(|level| crate::inventory::stored(&item, level.world.registry())) else { continue };
        drops::reeled(world, layer, ItemStack { tag: caught.tag, ..stack }, at, pull(head, middle));
        orbs::spawn(world, layer, orbs_at, points);
    }
}
