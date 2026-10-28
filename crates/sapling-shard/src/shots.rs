//! What players loose and throw (docs/research/projectiles.md): a bow drawn and let go (bows.rs),
//! a crossbow loaded and shot (crossbows.rs), snowballs, eggs, ender pearls, splash and lingering
//! potions, bottles o' enchanting and wind charges thrown (throws.rs), a trident thrown or ridden
//! (tridents.rs). Their flight and what they do where they meet something: creatures/flight.rs.
//! Stuck ones are picked up (pickup.rs); a lingering potion leaves a cloud (clouds.rs). A crossbow's
//! rocket is the fireworks' (fireworks.rs), a Channeling trident's bolt the lightning's (lightning.rs).

mod aim;
mod ammo;
mod bows;
pub(crate) mod clouds;
mod crossbows;
pub(crate) mod dispensed;
mod kinds;
mod pickup;
mod throws;
mod tridents;

use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_world::ItemStack;

use crate::components::{InLayer, Mode, NetId, Position, Tick};
use crate::creatures::{Launch, Loosed, Shot};
use crate::inventory::{self, Inventory};
use crate::vitals::eating;

/// A player drawing a bow, loading a crossbow or readying a trident: the item, and the tick it began.
#[derive(Component, Clone, Debug)]
pub(crate) struct Drawing {
    item: String,
    since: u64,
}

/// What the player holds.
fn held(world: &World, player: Entity) -> Option<ItemStack> {
    Some(world.get::<Inventory>(player)?.in_hand().as_ref()?.stack.clone())
}

fn is_creative(world: &World, player: Entity) -> bool {
    world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative)
}

fn mode(world: &World, player: Entity) -> Option<GameMode> {
    world.get::<Mode>(player).map(|mode| mode.0)
}

/// A click in the air: a thrown thing goes at once, a loaded crossbow shoots, a bow, an unloaded
/// crossbow or a trident is drawn until let go. True where the hand held one of these.
pub(crate) fn use_in_air(world: &mut World, player: Entity) -> bool {
    let Some(stack) = held(world, player).filter(|_| mode(world, player) != Some(GameMode::Spectator)) else { return false };
    if crate::fishing::use_rod(world, player, &stack) || throws::throw(world, player, &stack) || crossbows::shoot(world, player, &stack) {
        return true;
    }
    let draws = bows::may_draw(world, player, &stack) || crossbows::may_load(world, player, &stack) || tridents::may_ready(world, player, &stack);
    if draws && world.get::<Drawing>(player).is_none() {
        let since = world.resource::<Tick>().0;
        world.entity_mut(player).insert(Drawing { item: stack.name.clone(), since });
        eating::show(world, player, true);
    }
    draws
}

/// Use let go: what was drawn is loosed, if it was drawn long enough.
pub(crate) fn release(world: &mut World, player: Entity) {
    let Some(drawing) = world.get::<Drawing>(player).cloned() else { return };
    world.entity_mut(player).remove::<Drawing>();
    eating::show(world, player, false);
    let Some(stack) = held(world, player).filter(|stack| stack.name == drawing.item) else { return };
    let ticks = world.resource::<Tick>().0.saturating_sub(drawing.since);
    match stack.name.as_str() {
        bows::BOW => bows::loose(world, player, &stack, ticks),
        crossbows::CROSSBOW => crossbows::load(world, player, &stack, ticks),
        tridents::TRIDENT => tridents::let_go(world, player, &stack, ticks),
        _ => {}
    }
}

/// A tick of what players loose: a draw whose item left the hand is over, stuck shots are picked
/// up, tridents come back.
pub(crate) fn tick(world: &mut World) {
    let drawing: Vec<(Entity, String)> = world.query::<(Entity, &Drawing)>().iter(world).map(|(player, drawing)| (player, drawing.item.clone())).collect();
    for (player, item) in drawing {
        if held(world, player).is_none_or(|stack| stack.name != item) || crate::vitals::is_dead(world, player) {
            world.entity_mut(player).remove::<Drawing>();
            eating::show(world, player, false);
        }
    }
    pickup::tick(world);
    tridents::tick(world);
}

/// Whether a world folder's actor of `kind` is a shot that lay where it stuck.
pub(crate) fn is_kept(kind: &str) -> bool {
    kinds::kept(kind).is_some()
}

/// A stuck shot a world folder kept lies in `layer` again; false for one no player may pick up.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: &sapling_world::Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    let Some(shot) = kinds::kept(&actor.identifier) else { return false };
    crate::creatures::shot_from_storage(world, layer, &actor.identifier, actor.position, [actor.yaw, actor.pitch], unique_id, compound, shot)
}

/// Where a player is and what it is: its world, place and name as a killer.
fn shooter(world: &World, player: Entity) -> Option<(Entity, Position, String)> {
    let name = world.get::<crate::components::Identity>(player)?.0.name.clone();
    Some((world.get::<InLayer>(player)?.0, *world.get::<Position>(player)?, format!("player:{name}")))
}

/// The player looses `shot` from its eyes at `power` along its look turned `tilt` up, carrying
/// `loosed`, its motion turned `turn` degrees about the up axis. The shot's entity.
#[allow(clippy::too_many_arguments)]
fn loose(world: &mut World, player: Entity, shot: Shot, power: f32, tilt: f32, uncertainty: f32, turn: f32, loosed: Loosed) -> Option<Entity> {
    let (layer, position, archer) = shooter(world, player)?;
    let (from, motion) = aim::from_eyes(world, &position, power, tilt, uncertainty);
    let motion = aim::turned(motion, turn);
    crate::creatures::launch(world, Launch { layer, by: player, archer, from, motion, shot, gives: None }, Some(loosed))
}

/// `sound` heard from the player, where BDS hears a player's (the middle of its box).
fn heard(world: &mut World, player: Entity, sound: &str) {
    let (Some(layer), Some(position), Some(id)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).copied(), world.get::<NetId>(player).map(|id| id.0)) else { return };
    let [x, y, z] = position.feet;
    let packet = acacia_proto::packets::LevelSoundEvent {
        sound_id: sound.into(),
        position: acacia_proto::types::Vec3f { x, y: y + 0.9, z },
        extra_data: -1,
        entity_type: "minecraft:player".into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: id as i64,
        fire_at_position: None,
    };
    crate::view::to_viewers(world, layer, position.feet.map(|c| c.floor() as i32), &[sapling_net::encode(&packet)]);
}

/// Puts `stack` in the hand the player holds, or empties it.
fn set_hand(world: &mut World, player: Entity, stack: Option<ItemStack>) {
    inventory::change(world, player, |inventory| {
        let slot = inventory::Place::Main(usize::from(inventory.selected) % inventory::HOTBAR);
        inventory.put(slot, stack);
    });
}
