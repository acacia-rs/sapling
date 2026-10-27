//! Bundles, as BDS kept them (research/captures/bds-bundles.txt; docs/research/bundles.md). A bundle
//! is numbered (`bundle_id`, counting up in the level) the tick after it comes into a player's
//! items; requests then name its contents as the `Dynamic` container of that number, one stack per
//! slot of the client's choosing. What it holds may weigh 64: a stack weighs 64 over the item's own
//! stack size each, so a sword fills it; no bundle or shulker box goes in. A request that would
//! overfill it is refused whole with status 55. Each changed slot is then shown as an
//! InventorySlot of window 125. The contents stay in the bundle's own tag, as BDS saves them, and
//! never go out with it (sapling_data::items).
// TODO: bundle numbers start again from 0 with the shard; a player's own bundles are skipped over.

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{InventoryContent, InventorySlot};
use acacia_proto::types::{ContainerSlotType, FullContainerName, ItemStackRequest as Request, ItemStackRequestActionsItemContent as Action, WindowIDVarint};
use bevy_ecs::prelude::*;
use sapling_data::STORAGE_CONTENT;
use sapling_world::{BlockRegistry, ItemStack};

use super::{Held, Inventory, MAIN, Place, wire};
use crate::components::{Outgoing, in_world};

pub(crate) const SLOTS: usize = 64;
const MAX_WEIGHT: u32 = 64;
const WINDOW: u32 = 125;
const BUNDLE_ID: &str = "bundle_id";
const COMPOUND: u8 = 10;

/// The level's next bundle number.
#[derive(Resource, Default)]
struct Numbered(i32);

fn is_bundle(name: &str) -> bool {
    name.ends_with("bundle")
}

/// The number a bundle's tag names it by.
pub(crate) fn bundle_id(stack: &ItemStack) -> Option<i32> {
    match stack.tag.as_ref()?.get(BUNDLE_ID)? {
        Value::Int(id) => Some(*id).filter(|_| is_bundle(&stack.name)),
        _ => None,
    }
}

fn contents(stack: &ItemStack) -> Vec<Option<ItemStack>> {
    let mut slots = vec![None; SLOTS];
    if let Some(Value::List(items)) = stack.tag.as_ref().and_then(|tag| tag.get(STORAGE_CONTENT)) {
        for item in &items.items {
            let Some(Value::Byte(slot)) = item.get("Slot") else { continue };
            if let Some(at) = slots.get_mut(*slot as u8 as usize) {
                *at = ItemStack::parse(item);
            }
        }
    }
    slots
}

/// `bundle` holding `held` in its tag; no list where it holds nothing.
fn holding(bundle: &ItemStack, held: &[Option<Held>]) -> ItemStack {
    let registry = BlockRegistry::vanilla();
    let stored = |(slot, held): (usize, &Held)| {
        let mut fields = held.stack.fields(registry);
        fields.push(("Slot".into(), Value::Byte(slot as i8)));
        Value::Compound(fields)
    };
    let items: Vec<Value> = held.iter().enumerate().filter_map(|(slot, held)| Some(stored((slot, held.as_ref()?)))).collect();
    let list = (!items.is_empty()).then(|| Value::List(List { tag: COMPOUND, items }));
    ItemStack { tag: sapling_data::gear::with_field(bundle.tag.clone(), STORAGE_CONTENT, list), ..bundle.clone() }
}

/// What a stack weighs in a bundle; `None` for what never goes in.
fn weight(stack: &ItemStack) -> Option<u32> {
    let banned = is_bundle(&stack.name) || stack.name.ends_with("shulker_box");
    (!banned).then(|| MAX_WEIGHT / u32::from(sapling_data::max_stack(&stack.name).max(1)) * u32::from(stack.count))
}

/// Whether what a bundle holds fits in it.
pub(super) fn fits(held: &[Option<Held>]) -> bool {
    held.iter().flatten().map(|held| weight(&held.stack)).sum::<Option<u32>>().is_some_and(|total| total <= MAX_WEIGHT)
}

/// Where the bundle numbered `id` lies among a player's own slots.
fn find(inventory: &Inventory, id: i32) -> Option<Place> {
    let places = (0..MAIN).map(Place::Main).chain([Place::Offhand, Place::Cursor]).chain((0..inventory.container.len()).map(Place::Container));
    places.into_iter().find(|place| inventory.slot(*place).and_then(Option::as_ref).is_some_and(|held| bundle_id(&held.stack) == Some(id)))
}

/// Lends a request the contents of the bundle it names, under the ids the client was told before
/// when they are the same stacks.
pub(super) fn lend(inventory: &mut Inventory, request: &Request) {
    let named = request.actions.iter().flat_map(|action| match &action.content {
        Action::Take(take) => [Some(&take.source), Some(&take.destination)],
        Action::Place(place) => [Some(&place.source), Some(&place.destination)],
        Action::Swap(swap) => [Some(&swap.source), Some(&swap.destination)],
        Action::Drop(drop) => [Some(&drop.source), None],
        _ => [None, None],
    });
    let id = named.flatten().find(|info| info.slot_type.container_id == ContainerSlotType::Dynamic).and_then(|info| info.slot_type.dynamic_container_id);
    let Some(id) = id.and_then(|id| i32::try_from(id).ok()) else { return };
    let Some(bundle) = find(inventory, id).and_then(|place| inventory.slot(place)?.clone()) else { return };
    let known = inventory.bundle.take().filter(|(known, held)| *known == id && holding(&bundle.stack, held) == bundle.stack);
    let held = match known {
        Some((_, held)) => held,
        None => contents(&bundle.stack).into_iter().map(|stack| Some(Held { stack: stack?, id: inventory.new_id() })).collect(),
    };
    inventory.bundle = Some((id, held));
}

/// Writes the lent contents back into their bundle; the slots that changed.
pub(super) fn keep(inventory: &mut Inventory, before: &[Option<Held>]) -> Vec<usize> {
    let Some((id, held)) = inventory.bundle.clone().filter(|(_, held)| held != before) else { return Vec::new() };
    let Some(place) = find(inventory, id) else { return Vec::new() };
    if let Some(Some(bundle)) = inventory.slot_mut(place) {
        bundle.stack = holding(&bundle.stack, &held);
    }
    (0..SLOTS).filter(|slot| before.get(*slot) != held.get(*slot)).collect()
}

fn container(id: i32) -> FullContainerName {
    FullContainerName { container_id: ContainerSlotType::Dynamic, dynamic_container_id: Some(id as u32) }
}

/// The bundle's changed slots, as BDS showed them after its answer.
pub(super) fn shown_slots(inventory: &Inventory, slots: &[usize]) -> Vec<InventorySlot> {
    let Some((id, held)) = &inventory.bundle else { return Vec::new() };
    let storage = find(inventory, *id).and_then(|place| inventory.slot(place).cloned()).map(|bundle| wire(&bundle, false));
    let shown = |slot: usize| InventorySlot { window_id: WindowIDVarint::from_raw(WINDOW.into()), slot: slot as u32, container: Some(container(*id)), storage_item: storage.clone(), item: wire(&held[slot], true) };
    slots.iter().map(|&slot| shown(slot)).collect()
}

/// Numbers each bundle in a player's slots that has no number yet, and shows the player its
/// windows again and the bundle's own, as BDS did the tick after a bundle was given.
pub(crate) fn number(world: &mut World) {
    for player in in_world(world) {
        let Some(inventory) = world.get::<Inventory>(player).filter(|inventory| inventory.loaded) else { continue };
        let unnumbered: Vec<usize> = (0..MAIN).filter(|slot| inventory.main[*slot].as_ref().is_some_and(|held| is_bundle(&held.stack.name) && bundle_id(&held.stack).is_none())).collect();
        if unnumbered.is_empty() {
            continue;
        }
        let own = (0..MAIN).filter_map(|slot| inventory.main[slot].as_ref().and_then(|held| bundle_id(&held.stack))).max();
        let mut next = world.get_resource_or_insert_with(Numbered::default).0.max(own.map_or(0, |id| id + 1));
        let mut shown = Vec::new();
        let Some(mut inventory) = world.get_mut::<Inventory>(player) else { continue };
        for slot in unnumbered {
            let Some(held) = inventory.main[slot].clone() else { continue };
            let tag = sapling_data::gear::with_field(held.stack.tag.clone(), BUNDLE_ID, Some(Value::Int(next)));
            inventory.put(Place::Main(slot), Some(ItemStack { tag, ..held.stack }));
            shown.push((next, inventory.main[slot].clone()));
            next += 1;
        }
        world.resource_mut::<Numbered>().0 = next;
        super::show_windows(world, player, true);
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            for (id, bundle) in shown {
                let input = vec![sapling_data::no_item(); SLOTS];
                out.push(&InventoryContent { window_id: WindowIDVarint::from_raw(WINDOW.into()), input, container: container(id), storage_item: wire(&bundle, false) });
            }
        }
    }
}
