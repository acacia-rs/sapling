//! What a player's click on a mob or knot does with what it holds, first that applies: a knot lets
//! its mobs go; a mob on the player's lead is let go; a named name tag names it; a lead ties it;
//! shears, a brush, a bucket, a bowl, a flower or a dye take or give what [`super::products`] says;
//! its food heals a tamed one that is hurt, grows a baby, puts a grown one in love, tames a wild
//! one or calms a mount; and its owner's click with anything else sits it down or stands it up.
// TODO: a click does not swing the player's arm for its viewers.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;

use super::products::{self, spend};
use super::{Raised, ageing, breeding, keep_for_good, knots, leads, owners, shown};
use crate::creatures::{Creature, Health};
use crate::entities::Kind;
use crate::inventory::Inventory;

const NAME_TAG: &str = "minecraft:name_tag";

/// The name a stack was given at an anvil.
fn given_name(world: &World, player: Entity) -> Option<String> {
    let held = world.get::<Inventory>(player)?.in_hand().as_ref()?;
    let Some(Value::String(name)) = held.stack.tag.as_ref()?.get("display")?.get("Name") else { return None };
    (!name.is_empty()).then(|| name.to_string())
}

/// The mob takes `name` and stays for good.
pub(super) fn name(world: &mut World, entity: Entity, name: String) {
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return };
    raised.kept.name = name;
    keep_for_good(world, entity);
    shown::name(world, entity);
    crate::creatures::reshow(world, entity);
}

/// The player clicked the mob or knot `entity`.
pub(crate) fn interact(world: &mut World, player: Entity, entity: Entity) {
    if knots::undo(world, entity) || world.get::<Raised>(entity).is_none() {
        return;
    }
    if world.get::<Raised>(entity).is_some_and(|raised| raised.holder == Some(player)) {
        return leads::untie(world, entity, "lead.unleash");
    }
    let Some(kind) = world.get::<Kind>(entity).map(|kind| kind.0.clone()) else { return };
    let item = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone())).unwrap_or_default();
    let variant = world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref()).map_or(0, |look| look.variant);
    if item == NAME_TAG {
        if let Some(given) = given_name(world, player) {
            name(world, entity, given);
            spend(world, player);
        }
        return;
    }
    if item == leads::LEAD {
        if leads::leash(world, entity, &kind, player) {
            spend(world, player);
        }
        return;
    }
    if products::take(world, entity, &kind, &item, player, variant) {
        return;
    }
    let fed = heal(world, entity, &kind, &item)
        || ageing::feed(world, entity, &kind, &item)
        || breeding::fall_in_love(world, entity, &kind, &item, player)
        || owners::try_tame(world, entity, &kind, &item, player)
        || owners::calm(world, entity, &kind, &item);
    if fed {
        spend(world, player);
        crate::creatures::reshow(world, entity);
        return;
    }
    if owners::toggle_sitting(world, entity, &kind, player) {
        crate::creatures::reshow(world, entity);
    }
}

/// A tamed mob that is hurt eats one of its healing foods.
fn heal(world: &mut World, entity: Entity, kind: &str, item: &str) -> bool {
    let Some(amount) = sapling_data::husbandry(kind).and_then(|rules| rules.heal.iter().find(|(food, _)| *food == item).map(|(_, amount)| *amount)) else { return false };
    if !world.get::<Raised>(entity).is_some_and(Raised::is_tamed) {
        return false;
    }
    let Some(mut health) = world.get_mut::<Health>(entity).filter(|health| health.left() < health.max()) else { return false };
    health.mend(amount);
    true
}
