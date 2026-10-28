//! Bundles, as BDS kept them (research/captures/bds-bundles.txt; docs/research/bundles.md). A bundle
//! is numbered (`bundle_id`, counting up in the level) the tick after it comes into a player's
//! items; requests then name its contents as the `Dynamic` container of that number, one stack per
//! slot of the client's choosing. What it holds may weigh 64: a stack weighs 64 over the item's own
//! stack size each, so a sword fills it; no bundle or shulker box goes in. A request that would
//! overfill it is refused whole with status 55, one taking from an empty slot with 49. Each changed
//! slot is then shown as an InventorySlot of window 125. The contents stay in the bundle's own tag,
//! as BDS saves them, and never go out with it (sapling_data::items): a numbered bundle the client
//! was not shown, as after a rejoin, has them sent as window 125's content.
// TODO: bundle numbers start again from 0 with the shard, as BDS's did (bds-bundlekeep2.txt); a
// player's own bundles are skipped over.

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{InventoryContent, InventorySlot};
use acacia_proto::types::{ContainerSlotType, FullContainerName, ItemStackRequest as Request, ItemStackRequestActionsItemContent as Action, StackRequestSlotInfo, WindowIDVarint};
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

fn is_shown(inventory: &Inventory, id: i32) -> bool {
    inventory.bundle.as_ref().is_some_and(|(lent, _)| *lent == id) || inventory.bundles_shown.iter().any(|(shown, _)| *shown == id)
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
    if let Some(lent) = inventory.bundle.take() {
        inventory.bundles_shown.retain(|(shown, _)| *shown != lent.0);
        inventory.bundles_shown.push(lent);
    }
    let known = inventory.bundles_shown.iter().position(|(shown, _)| *shown == id).map(|at| inventory.bundles_shown.swap_remove(at).1);
    let held = match known.filter(|held| holding(&bundle.stack, held) == bundle.stack) {
        Some(held) => held,
        None => contents(&bundle.stack).into_iter().map(|stack| Some(Held { stack: stack?, id: inventory.new_id() })).collect(),
    };
    inventory.bundle = Some((id, held));
}

/// Whether a request takes out of a slot of the lent bundle that holds nothing.
pub(super) fn takes_nothing(inventory: &Inventory, request: &Request) -> bool {
    let empty = |source: &StackRequestSlotInfo| {
        source.slot_type.container_id == ContainerSlotType::Dynamic && inventory.bundle.as_ref().is_some_and(|(_, held)| held.get(usize::from(source.slot)).is_some_and(Option::is_none))
    };
    request.actions.iter().any(|action| match &action.content {
        Action::Take(take) => empty(&take.source),
        Action::Place(place) => empty(&place.source),
        _ => false,
    })
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

pub(super) fn container(id: i32) -> FullContainerName {
    FullContainerName { container_id: ContainerSlotType::Dynamic, dynamic_container_id: Some(id as u32) }
}

/// The bundle's changed slots, as BDS showed them after its answer.
pub(super) fn shown_slots(inventory: &Inventory, slots: &[usize]) -> Vec<InventorySlot> {
    let Some((id, held)) = &inventory.bundle else { return Vec::new() };
    let storage = find(inventory, *id).and_then(|place| inventory.slot(place).cloned()).map(|bundle| wire(&bundle, false));
    let shown = |slot: usize| InventorySlot { window_id: WindowIDVarint::from_raw(WINDOW.into()), slot: slot as u32, container: Some(container(*id)), storage_item: storage.clone(), item: wire(&held[slot], true) };
    slots.iter().map(|&slot| shown(slot)).collect()
}

fn content(id: i32, bundle: &Option<Held>, held: &[Option<Held>]) -> InventoryContent {
    let input = held.iter().map(|held| wire(held, true)).collect();
    InventoryContent { window_id: WindowIDVarint::from_raw(WINDOW.into()), input, container: container(id), storage_item: wire(bundle, false) }
}

/// Numbers each bundle in a player's slots that has no number yet, and shows the player its
/// windows again and the bundle's own, as BDS did the tick after a bundle was given. A numbered
/// bundle the client was not shown has its contents shown.
pub(crate) fn number(world: &mut World) {
    for player in in_world(world) {
        let Some(inventory) = world.get::<Inventory>(player).filter(|inventory| inventory.loaded) else { continue };
        let unnumbered: Vec<usize> = (0..MAIN).filter(|slot| inventory.main[*slot].as_ref().is_some_and(|held| is_bundle(&held.stack.name) && bundle_id(&held.stack).is_none())).collect();
        if !unnumbered.is_empty() {
            let own = (0..MAIN).filter_map(|slot| inventory.main[slot].as_ref().and_then(|held| bundle_id(&held.stack))).max();
            let next = world.get_resource_or_insert_with(Numbered::default).0.max(own.map_or(0, |id| id + 1));
            world.resource_mut::<Numbered>().0 = next + unnumbered.len() as i32;
            number_slots(world, player, &unnumbered, next);
        }
        show_unshown(world, player);
    }
}

fn number_slots(world: &mut World, player: Entity, slots: &[usize], first: i32) {
    let Some(mut inventory) = world.get_mut::<Inventory>(player) else { return };
    let (mut shown, empty) = (Vec::new(), vec![None; SLOTS]);
    for (id, &slot) in (first..).zip(slots) {
        let Some(held) = inventory.main[slot].clone() else { continue };
        let tag = sapling_data::gear::with_field(held.stack.tag.clone(), BUNDLE_ID, Some(Value::Int(id)));
        inventory.put(Place::Main(slot), Some(ItemStack { tag, ..held.stack }));
        inventory.bundles_shown.push((id, empty.clone()));
        shown.push(content(id, &inventory.main[slot].clone(), &empty));
    }
    super::show_windows(world, player, true);
    show_contents(world, player, &shown);
}

/// Sends the bundles' `shown` contents to `player`; their numbers are registered (bundle_cleanup.rs).
fn show_contents(world: &mut World, player: Entity, shown: &[InventoryContent]) {
    for content in shown {
        super::bundle_cleanup::register(world, content.container.dynamic_container_id.unwrap_or_default() as i32);
    }
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        shown.iter().for_each(|content| out.push(content));
    }
}

/// Shows the contents of each numbered bundle in a player's slots its client was not shown, and
/// forgets those of bundles it no longer holds.
fn show_unshown(world: &mut World, player: Entity) {
    let Some(mut inventory) = world.get_mut::<Inventory>(player).filter(|inventory| inventory.sent) else { return };
    let held: Vec<i32> = (0..MAIN).map(Place::Main).chain([Place::Offhand, Place::Cursor]).filter_map(|place| bundle_id(&inventory.slot(place)?.as_ref()?.stack)).collect();
    inventory.bundles_shown.retain(|(id, _)| held.contains(id));
    let mut shown = Vec::new();
    for slot in 0..MAIN {
        let Some(bundle) = inventory.main[slot].clone() else { continue };
        let Some(id) = bundle_id(&bundle.stack).filter(|id| !is_shown(&inventory, *id)) else { continue };
        let held: Vec<Option<Held>> = contents(&bundle.stack).into_iter().map(|stack| Some(Held { stack: stack?, id: inventory.new_id() })).collect();
        shown.push(content(id, &Some(bundle), &held));
        inventory.bundles_shown.push((id, held));
    }
    show_contents(world, player, &shown);
}
