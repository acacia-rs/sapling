//! Illager captains, as the pillager's and vindicator's `illager_squad_captain` and `patrol_captain`
//! groups make one (events `spawn_as_illager_captain`, `promote_to_*_captain`): the ominous banner
//! (`set_banner_details` type 1) worn in its chest slot and dropped whatever killed it
//! (`slot_drop_chance` 1), and its `pillager_captain` loot, an ominous bottle of amplifier 0 to 4
//! when a player killed it. BDS makes captains of patrols and outposts only: `_createRaid`
//! (0x142a50270) spawns its groups with `spawn_for_raid` alone.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::Creature;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities;

const EVENTS: [&str; 4] = ["minecraft:spawn_as_illager_captain", "minecraft:promote_to_illager_captain", "minecraft:promote_to_patrol_captain", "minecraft:spawn_as_patrol_captain"];
const KINDS: [&str; 2] = ["minecraft:pillager", "minecraft:vindicator"];
const BANNER: &str = "minecraft:banner";
/// The ominous banner's data value, and its `Type`.
const OMINOUS_DATA: i16 = 15;
const OMINOUS: i32 = 1;
const BOTTLE: &str = "minecraft:ominous_bottle";
const AMPLIFIERS: u32 = 5;

#[derive(Component)]
pub(super) struct Captain;

/// The ominous banner.
pub(crate) fn banner() -> ItemStack {
    ItemStack { name: BANNER.into(), count: 1, damage: OMINOUS_DATA, block: None, tag: Some(Value::Compound(vec![("Type".into(), Value::Int(OMINOUS))])) }
}

fn is_ominous(stack: &ItemStack) -> bool {
    stack.name == BANNER && matches!(&stack.tag, Some(Value::Compound(fields)) if fields.iter().any(|(key, value)| key == "Type" && *value == Value::Int(OMINOUS)))
}

/// A mob of `kind` that came with spawn `event`: a captain if the event makes one, shown with its banner.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if !KINDS.contains(&kind) || !event.is_some_and(|event| EVENTS.contains(&event)) {
        return;
    }
    world.entity_mut(entity).insert(Captain);
    super::reshow(world, entity);
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).copied()) else { return };
    let worn = worn(world, entity, id);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), worn);
}

/// A stored mob wearing the ominous banner is a captain.
pub(super) fn adopt(world: &mut World, entity: Entity) {
    let wears = world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref()).and_then(|look| look.armor.get(1).cloned().flatten()).is_some_and(|chest| is_ominous(&chest));
    if wears {
        world.entity_mut(entity).insert(Captain);
    }
}

/// What shows a captain's banner.
pub(super) fn worn(world: &World, entity: Entity, id: u64) -> Vec<Bytes> {
    if world.get::<Captain>(entity).is_none() {
        return Vec::new();
    }
    let banner = banner();
    let stack = sapling_data::Stack { name: &banner.name, count: 1, damage: banner.damage, block: None, tag: banner.tag.as_ref() };
    vec![sapling_net::encode(&sapling_data::mob_armor(id, &[None, Some(stack)]))]
}

/// What a dead captain lets go of: its banner, and an ominous bottle if a player killed it.
pub(super) fn let_go(world: &mut World, entity: Entity, by_player: bool) -> Vec<ItemStack> {
    if world.get::<Captain>(entity).is_none() {
        return Vec::new();
    }
    let mut dropped = vec![banner()];
    if by_player {
        let amplifier = world.get_resource_or_insert_with(Dice::default).roll(AMPLIFIERS) as i16;
        dropped.push(ItemStack { name: BOTTLE.into(), count: 1, damage: amplifier, block: None, tag: None });
    }
    dropped
}

/// Writes a kept captain's banner into its stored compound, as BDS keeps it in `Armor`.
pub(super) fn keep(world: &mut World) {
    let captains: Vec<Entity> = world.query_filtered::<Entity, (With<Captain>, With<entities::Stored>)>().iter(world).collect();
    for captain in captains {
        entities::restore(world, captain, |compound, registry| {
            let stored = sapling_world::stored_actors(compound).ok()?.into_iter().next()?.0?;
            let mut armor: [Option<ItemStack>; 4] = std::array::from_fn(|slot| stored.armor.get(slot).cloned().flatten());
            armor[1] = Some(banner());
            let hands = [stored.mainhand.clone(), stored.offhand.clone()];
            sapling_world::geared_actor(compound, &armor, &hands, sapling_world::actor_pose(compound).unwrap_or(0), registry).ok()
        });
    }
}
