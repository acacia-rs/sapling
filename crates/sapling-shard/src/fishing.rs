//! Fishing by BDS's `FishingRodItem::use` (143700ef0) and `FishingHook` (docs/research/fishing.md):
//! a rod's use casts a hook (cast.rs) or reels the one out back in (reel.rs). The hook flies, bobs
//! in water and hooks what it meets (bob.rs) while fish come and bite (bite.rs). It goes when its
//! fisher dies, leaves its world, lets go of the rod or is 32 blocks off.

mod bite;
mod bob;
mod cast;
mod reel;

use acacia_proto::packets::{CompletedUsingItem, CompletedUsingItemUseMethod};
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::components::{InLayer, NetId, Outgoing, Position, Tick};
use crate::inventory::Inventory;
use crate::vitals::eating;
use crate::{entities, mobs};

const ROD: &str = "minecraft:fishing_rod";
const HOOK: &str = "minecraft:fishing_hook";
/// The hook goes when its fisher's feet are farther than this (squared: 32 blocks).
const REACH_SQUARED: f32 = 1024.0;

/// A fishing hook out, and what it is doing.
#[derive(Component, Debug)]
pub(crate) struct Hook {
    fisher: Entity,
    /// The tick it was cast: it first moves on the next, once its viewers know it.
    cast: u64,
    /// `mBobTimer`, and the fish's way in degrees times 57.3 (`mFishAngle`).
    bob: f32,
    angle: f32,
    /// Ticks until a fish comes (`mTimeUntilLured`), until it reaches the hook (`mTimeUntilHooked`)
    /// and while it is on (`mTimeUntilNibble`).
    lured: i32,
    hooked: i32,
    nibble: i32,
    /// It hit a block (`mInGround`).
    in_ground: bool,
    /// What it hooked.
    target: Option<Entity>,
    /// It was cast in a jungle: the jungle's loot.
    jungle: bool,
}

/// A player's hook out.
#[derive(Component, Clone, Copy)]
pub(crate) struct Line(Entity);

/// The tick a player used a rod: it shows using an item until the next (bds-fish.txt).
#[derive(Component)]
struct RodUsed(u64);

/// A click in the air with a rod: a hook out comes back, else one is cast; true where the hand
/// held a rod. Either way BDS ends the use and shows the player its whole inventory.
pub(crate) fn use_rod(world: &mut World, player: Entity, stack: &ItemStack) -> bool {
    if stack.name != ROD {
        return false;
    }
    let Some(id) = world.get::<Inventory>(player).map(|inventory| crate::inventory::wire(inventory.in_hand(), false).network_id) else { return false };
    eating::show(world, player, true);
    let now = world.resource::<Tick>().0;
    world.entity_mut(player).insert(RodUsed(now));
    match world.get::<Line>(player).copied() {
        Some(Line(hook)) => reel::retrieve(world, player, hook),
        None => cast::cast(world, player),
    }
    let used = CompletedUsingItem { used_item_id: id, use_method: CompletedUsingItemUseMethod::Unknown(-1) };
    let mut query = world.query::<(&Inventory, &mut Outgoing)>();
    if let Ok((inventory, mut out)) = query.get_mut(world, player) {
        out.push(&used);
        inventory.send_all(&mut out, false);
    }
    true
}

/// The hook leaves the world, and its fisher holds none.
fn remove(world: &mut World, hook: Entity) {
    // A fisher that left the shard is gone already.
    if let Some(fisher) = world.get::<Hook>(hook).map(|hook| hook.fisher)
        && let Ok(mut fisher) = world.get_entity_mut(fisher)
    {
        fisher.remove::<Line>();
    }
    mobs::remove(world, hook);
}

/// Whether the fisher still fishes with the hook: alive, in its world, holding a rod, near.
fn still_fishing(world: &World, fisher: Entity, layer: Entity, at: [f32; 3]) -> bool {
    let holds_rod = world.get::<Inventory>(fisher).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| held.stack.name == ROD);
    let (Some(feet), Some(in_layer)) = (world.get::<Position>(fisher).map(|position| position.feet), world.get::<InLayer>(fisher)) else { return false };
    let far = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>() > REACH_SQUARED;
    holds_rod && in_layer.0 == layer && !far && !crate::vitals::is_dead(world, fisher)
}

/// A tick of every hook out.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let used: Vec<Entity> = world.query::<(Entity, &RodUsed)>().iter(world).filter(|(_, used)| used.0 < now).map(|(player, _)| player).collect();
    for player in used {
        world.entity_mut(player).remove::<RodUsed>();
        eating::show(world, player, false);
    }
    let hooks: Vec<(Entity, Entity, Entity, u64)> = world.query::<(Entity, &Hook, &InLayer)>().iter(world).filter(|(_, hook, _)| hook.cast < now).map(|(entity, hook, layer)| (entity, hook.fisher, layer.0, hook.cast)).collect();
    for (hook, fisher, layer, cast) in hooks {
        let (Some(at), Some(id)) = (world.get::<crate::body::Body>(hook).map(|body| body.at), world.get::<NetId>(hook).map(|id| id.0)) else { continue };
        if !still_fishing(world, fisher, layer, at) {
            remove(world, hook);
            continue;
        }
        let appearing = cast::appearing(id, now - cast, at);
        if !appearing.is_empty() {
            entities::tell_viewers(world, hook, layer, at.map(|c| c.floor() as i32), appearing);
        }
        bob::tick(world, hook, layer);
    }
}
