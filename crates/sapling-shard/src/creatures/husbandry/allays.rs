//! An allay handed an item (`allay.json` `minecraft:interact`, `give_item` and `take_item`), as BDS's
//! took one (research/captures/bds-ak1allay.txt, bds-r1bundle.txt): a click holding anything but a lead,
//! not sneaking, on an allay with an empty hand puts one of it in the allay's hand (MobEquipment) and
//! takes one from the player's in that tick; the next tick it is heard `item_given` and shown busy
//! (`ACTION`) and owned (`OwnerEid`, the giver). A click holding something on an allay that holds
//! something does nothing; an empty-handed one takes its item back (the pack's `take_item`, not drilled).
//! What it then fetches and to whom it brings it is collecting.rs and giving.rs. Its owner is kept
//! with the world as a tamed mob's (`OwnerNew`, the giver's unique id): BDS's allay loaded again
//! was announced with its `OwnerEid` and followed that player (research/captures/bds-as3allay2.txt).

mod collecting;
mod giving;

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, MobSlot};
use sapling_world::ItemStack;

use super::{products, shown};
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::entities::Kind;
use crate::inventory::{self, Inventory};
use crate::movement::Motion;

pub(in crate::creatures) use collecting::take_up;
pub(crate) use giving::hear_note;

const ALLAY: &str = "minecraft:allay";

/// An allay handed an item, shown so the tick after.
#[derive(Component)]
pub(super) struct Given {
    by: u64,
    due: u64,
}

/// What an allay fetches: what it has collected (its one-slot inventory, its hand unchanged), the
/// tick it may collect again from, and the note block it last heard (its layer, cell and the tick).
#[derive(Component, Default)]
struct Errands {
    carried: Option<ItemStack>,
    rests_until: u64,
    note: Option<(Entity, BlockPos, u64)>,
    staying: bool,
    following: bool,
    ambient_count: u32,
    sounds_tame: bool,
}

/// BDS's `AmbientSoundServerSystem` re-reads an allay's `ambient` event names when its counter
/// passes this, then starts it over: 50 ticks after it appears and every 51 after (an allay
/// read under gdb, bds-aq3exit; every allay handed an item switched 152 ticks after it appeared).
const AMBIENT_PERIOD: u32 = 50;

fn held_by_allay(world: &World, allay: Entity) -> Option<ItemStack> {
    super::super::command_gear::gear_of(world, allay).into_iter().find(|(slot, _)| *slot == MobSlot::MainHand).and_then(|(_, item)| item)
}

fn errands(world: &mut World, allay: Entity) -> Mut<'_, Errands> {
    if world.get::<Errands>(allay).is_none() {
        world.entity_mut(allay).insert(Errands::default());
    }
    world.get_mut::<Errands>(allay).expect("just inserted")
}

/// Writes what the allay carries where BDS saved it: `ChestItems` slot 0, its one-slot inventory
/// (the world of research/captures/bds-aq3allay.txt).
fn keep_carried(world: &mut World, allay: Entity) {
    let items: Vec<(u8, ItemStack)> = world.get::<Errands>(allay).and_then(|errands| errands.carried.clone()).map(|carried| (0, carried)).into_iter().collect();
    crate::entities::restore(world, allay, |compound, registry| sapling_world::chested_actor(compound, &items, registry).ok());
}

/// An allay a world folder kept carries its `ChestItems` slot 0 again.
pub(super) fn adopt(world: &mut World, allay: Entity, kind: &str, compound: &[u8]) {
    let kept = sapling_world::actor_chest(compound).into_iter().find(|(slot, _)| *slot == 0).map(|(_, stack)| stack);
    if let Some(carried) = kept.filter(|_| kind == ALLAY) {
        errands(world, allay).carried = Some(carried);
    }
}

/// The player clicked `entity`; true where it was an allay, which takes the click whatever it does.
pub(super) fn hand(world: &mut World, player: Entity, entity: Entity) -> bool {
    if world.get::<Kind>(entity).is_none_or(|kind| kind.0 != ALLAY) || world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking) {
        return false;
    }
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.clone()));
    match (held, held_by_allay(world, entity)) {
        (Some(held), _) if held.name == super::leads::LEAD => return false,
        (Some(held), None) => {
            let one = ItemStack { count: 1, ..held };
            if super::super::command_gear::set(world, entity, MobSlot::MainHand, Some(one)).is_ok() {
                products::spend(world, player);
                let owner = world.get::<crate::tags::Marks>(player).map(|marks| marks.unique_id);
                if let Some(mut raised) = world.get_mut::<super::Raised>(entity) {
                    raised.kept.owner = owner;
                }
                let (Some(&NetId(by)), now) = (world.get::<NetId>(player), world.resource::<Tick>().0) else { return true };
                world.entity_mut(entity).insert(Given { by, due: now + 1 });
            }
        }
        (None, Some(item)) => {
            if super::super::command_gear::set(world, entity, MobSlot::MainHand, None).is_ok() {
                inventory::give_and_pop(world, player, &item);
                shown::flag(world, entity, Flags::ACTION, false);
            }
        }
        _ => {}
    }
    true
}

/// Before its move: an allay holding something collects what matches it, or brings what it
/// collected to a note block it heard or its owner, or keeps near that note block.
pub(super) fn steer(world: &mut World, allay: Entity, layer: Entity, kind: &str) -> bool {
    let Some(body) = world.get::<Body>(allay).copied().filter(|_| kind == ALLAY) else { return false };
    held_by_allay(world, allay).is_some_and(|hand| collecting::steer(world, allay, layer, &body, &hand)) || giving::steer(world, allay, layer, &body)
}

/// A tick of the allay's ambient check: holding something it sounds `ambient.tame` (the pack's
/// `query.is_using_item`), else `ambient`; its viewers hear of a change alone (bds-aq3allay.txt).
pub(super) fn check_ambient(world: &mut World, allay: Entity, kind: &str) {
    if kind != ALLAY {
        return;
    }
    let appeared = world.get::<Errands>(allay).is_none();
    let mut counted = errands(world, allay);
    // BDS counted the tick it appeared in, which ran before sapling's first check.
    counted.ambient_count += 1 + u32::from(appeared);
    if counted.ambient_count <= AMBIENT_PERIOD {
        return;
    }
    counted.ambient_count = 0;
    let tame = held_by_allay(world, allay).is_some();
    let mut checked = errands(world, allay);
    if checked.sounds_tame == tame {
        return;
    }
    checked.sounds_tame = tame;
    let name = if tame { "ambient.tame" } else { "ambient" };
    shown::metadata(world, allay, vec![crate::creatures::properties::ambient_sound(name)]);
}

/// Shows the allays handed an item last tick.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, u64)> = world.query::<(Entity, &Given)>().iter(world).filter(|(_, given)| given.due <= now).map(|(allay, given)| (allay, given.by)).collect();
    for (allay, by) in due {
        world.entity_mut(allay).remove::<Given>();
        shown::sound(world, allay, "item_given");
        let owner = MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(by as i64)) };
        shown::flag_with(world, allay, Flags::ACTION, true, vec![owner]);
    }
}
