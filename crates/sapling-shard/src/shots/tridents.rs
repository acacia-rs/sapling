//! A trident held in use 10 ticks or more and let go (BDS's `TridentItem::releaseUsing` 141fa6b10):
//! thrown, worn by one, and gone from the hand outside creative; with Riptide, only in water or
//! rain, it throws its holder instead. Impaling goes with it (creatures/hits.rs). One with Loyalty
//! that hit something or stuck comes back to its thrower once it is in the same world, flying
//! through blocks, and is given back as it is picked up.
// TODO: Loyalty's pull and Riptide's throw are the Java game's (enchant/archery.rs); no sounds of either.

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_data::enchant::{self, Enchant};
use sapling_data::shots::{self, TRIDENT_READY, TRIDENT_SPEED};
use sapling_world::ItemStack;

use super::kinds::TRIDENT as THROWN;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position};
use crate::creatures::{Loosed, Pickup};
use crate::sync::Told;
use crate::{entities, inventory};

pub(super) const TRIDENT: &str = "minecraft:trident";
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// A returning trident is caught this near its thrower's eyes, and slows by this a tick.
const CAUGHT: f32 = 1.5;
const SLOWS: f32 = 0.95;

pub(super) fn may_ready(_world: &World, _player: Entity, stack: &ItemStack) -> bool {
    stack.name == TRIDENT
}

/// Whether the player stands in water or rain.
fn is_wet(world: &World, player: Entity) -> bool {
    let (Some(layer), Some(position)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player)) else { return false };
    let raining = crate::weather::of(world, layer) != sapling_api::Weather::Clear;
    raining || world.get::<Layer>(layer).is_some_and(|level| crate::creatures::holds(&level.world, position.feet.map(|c| c.floor() as i32), WATER))
}

/// The trident `stack` let go after `ticks` held.
pub(super) fn let_go(world: &mut World, player: Entity, stack: &ItemStack, ticks: u64) {
    if ticks < TRIDENT_READY {
        return;
    }
    let tag = stack.tag.as_ref();
    let creative = super::is_creative(world, player);
    let lasts = shots::shooter_lasts(TRIDENT).unwrap_or(u16::MAX);
    let riptide = enchant::level(tag, Enchant::Riptide);
    if riptide > 0 {
        if !is_wet(world, player) {
            return;
        }
        let Some(position) = world.get::<Position>(player).copied() else { return };
        let push = super::aim::look(&position, 0.0).map(|c| c * enchant::riptide(riptide));
        crate::vitals::pushed(world, player, push);
        if !creative {
            inventory::wear_hand_by(world, player, 1, lasts);
        }
        super::heard(world, player, "item.trident.riptide_1");
        return;
    }
    if !creative {
        inventory::wear_hand_by(world, player, 1, lasts);
    }
    let Some(thrown) = super::held(world, player).filter(|held| held.name == TRIDENT) else { return };
    let pickup = if creative { Pickup::Creative } else { Pickup::Anyone };
    let loosed = Loosed { impaling: enchant::level(tag, Enchant::Impaling), loyalty: enchant::level(tag, Enchant::Loyalty), ..Loosed::new(player, pickup, Some(thrown)) };
    if super::loose(world, player, THROWN, TRIDENT_SPEED, 0.0, 1.0, 0.0, loosed).is_none() {
        return;
    }
    if !creative {
        super::set_hand(world, player, None);
    }
    super::heard(world, player, THROWN.sound);
}

/// Tridents with Loyalty that are done come back.
pub(super) fn tick(world: &mut World) {
    let lying: Vec<Entity> = crate::creatures::lying_shots(world).into_iter().map(|(trident, ..)| trident).collect();
    let loyal: Vec<(Entity, Entity, Entity, u64, Loosed)> = world
        .query::<(Entity, &Loosed, &InLayer, &NetId)>()
        .iter(world)
        .filter(|(trident, loosed, ..)| loosed.loyalty > 0 && (loosed.spent || loosed.returning || lying.contains(trident)))
        .filter_map(|(trident, loosed, layer, id)| Some((trident, loosed.player?, layer.0, id.0, loosed.clone())))
        .collect();
    for (trident, player, layer, id, loosed) in loyal {
        come_back(world, trident, player, layer, id, loosed);
    }
}

/// A tick of a trident coming back to `player`.
fn come_back(world: &mut World, trident: Entity, player: Entity, layer: Entity, id: u64, loosed: Loosed) -> Option<()> {
    let home = world.get::<InLayer>(player).filter(|on| on.0 == layer).and(world.get::<Position>(player)).map(|position| position.feet);
    let Some(feet) = home.filter(|_| !crate::vitals::is_dead(world, player)) else {
        world.get_mut::<Loosed>(trident)?.returning = false;
        return None;
    };
    if !loosed.returning {
        world.get_mut::<Loosed>(trident)?.returning = true;
        crate::creatures::unstick(world, trident);
    }
    let mut body = *world.get::<Body>(trident)?;
    let eyes = [feet[0], feet[1] + EYE_HEIGHT, feet[2]];
    let toward: [f32; 3] = std::array::from_fn(|axis| eyes[axis] - body.at[axis]);
    let far = toward.iter().map(|c| c * c).sum::<f32>().sqrt();
    if far < CAUGHT {
        let player_id = world.get::<NetId>(player)?.0;
        let item = (loosed.pickup == Pickup::Anyone).then_some(loosed.item).flatten();
        super::pickup::take(world, trident, layer, body.at, id, player, player_id, item);
        return Some(());
    }
    let pull = enchant::loyalty_pull(loosed.loyalty);
    body.motion = std::array::from_fn(|axis| body.motion[axis] * SLOWS + toward[axis] / far * pull);
    body.at = std::array::from_fn(|axis| body.at[axis] + body.motion[axis]);
    *world.get_mut::<Body>(trident)? = body;
    let packets = world.get_mut::<Told>(trident)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, trident, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}
