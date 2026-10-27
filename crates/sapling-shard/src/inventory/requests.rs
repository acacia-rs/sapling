//! What a client asks of its inventory (ItemStackRequest): each request is done whole on the
//! server's copy and confirmed with the slots it changed, or refused, and then the client is sent
//! its windows as they are.

use acacia_proto::packets::{ItemStackRequest, ItemStackResponse};
use acacia_proto::types::{
    ContainerSlotType, FullContainerName, ItemStackRequest as Request, ItemStackRequestActionsItemContent as Action, ItemStackResponsesItem,
    ItemStackResponsesItemContainersItem as Container, ItemStackResponsesItemContainersItemSlotsItem as SlotInfo, ItemStackResponsesItemStatus as Status,
    StackRequestSlotInfo,
};
use bevy_ecs::prelude::*;
use sapling_api::{DropEvent, GameMode, Player};
use sapling_data::Station;
use sapling_world::ItemStack;

use super::crafting::Craft;
use super::forge::{self, Bench, Forge, Forged};
use super::{ARMOR, Inventory, MAIN, MAX_COUNT, Place, described};
use crate::components::{Mode, Outgoing, PacketQueues, PerTick, in_world_identity};
use crate::{containers, drops, workstations};
use crate::plugins;

/// Requests looked at per player per tick; a client sends one per click.
const MAX_REQUESTS_PER_TICK: usize = 32;
/// The `Ui` slot of what a creative pick or a craft made.
const CREATED_SLOT: u8 = 50;
/// The `Ui` slots of the crafting grids: 28 to 31 on the inventory screen, 32 to 40 at a table.
const GRIDS: std::ops::RangeInclusive<usize> = 28..=40;
const STONECUTTER_INPUT: usize = 3;

/// What a request did to the inventory it was tried on.
#[derive(Default)]
struct Applied {
    /// The slots it named, as it named them: the answer says what each holds now.
    touched: Vec<(FullContainerName, u8, Place)>,
    dropped: Vec<ItemStack>,
    forged: Option<Forged>,
}

/// Where a request is made: the station whose recipes it may craft, and the workstation it may
/// craft at by the server's reckoning.
struct At {
    station: Station,
    bench: Option<Bench>,
}

fn place(info: &StackRequestSlotInfo) -> Option<Place> {
    let slot = usize::from(info.slot);
    match info.slot_type.container_id {
        // Which slots there are is the open container's to say: none while nothing is open.
        // `Container` is the protocol's `LevelEntity`: chests, hoppers, dispensers and droppers alike.
        ContainerSlotType::Container | ContainerSlotType::Barrel | ContainerSlotType::Shulker | ContainerSlotType::Crafter => Some(Place::Container(slot)),
        ContainerSlotType::FurnaceIngredient | ContainerSlotType::BlastFurnaceIngredient | ContainerSlotType::SmokerIngredient => Some(Place::Container(0)),
        ContainerSlotType::FurnaceFuel => Some(Place::Container(1)),
        ContainerSlotType::FurnaceOutput => Some(Place::Container(2)),
        // A brewing stand: the ingredient, three bottles, the blaze powder.
        ContainerSlotType::BrewingInput => Some(Place::Container(0)),
        ContainerSlotType::BrewingResult if (1..=3).contains(&slot) => Some(Place::Container(slot)),
        ContainerSlotType::BrewingFuel => Some(Place::Container(4)),
        ContainerSlotType::CraftingInput if GRIDS.contains(&slot) => Some(Place::Ui(slot)),
        ContainerSlotType::StonecutterInput if slot == STONECUTTER_INPUT => Some(Place::Ui(slot)),
        ContainerSlotType::Hotbar | ContainerSlotType::Inventory | ContainerSlotType::HotbarAndInventory if slot < MAIN => Some(Place::Main(slot)),
        ContainerSlotType::Armor if slot < ARMOR => Some(Place::Armor(slot)),
        ContainerSlotType::Offhand => Some(Place::Offhand),
        ContainerSlotType::Cursor => Some(Place::Cursor),
        ContainerSlotType::CreativeOutput if info.slot == CREATED_SLOT => Some(Place::Created),
        kind if forge::ui_slot(kind) == Some(slot) => Some(Place::Ui(slot)),
        _ => None,
    }
}

impl Applied {
    /// The slot a request names, if it holds the stack the client thinks it does.
    fn named(&mut self, inventory: &Inventory, info: &StackRequestSlotInfo) -> Result<Place, ()> {
        let place = place(info).ok_or(())?;
        let id = inventory.slot(place).and_then(Option::as_ref).map_or(0, |held| held.id);
        // An id below 1 names an empty slot or a stack this request made (its own request id).
        if info.stack_id > 0 && place != Place::Created && info.stack_id != id {
            return Err(());
        }
        if place != Place::Created && !self.touched.iter().any(|(_, _, touched)| *touched == place) {
            self.touched.push((info.slot_type.clone(), info.slot, place));
        }
        Ok(place)
    }

    fn transfer(&mut self, inventory: &mut Inventory, count: u8, source: &StackRequestSlotInfo, destination: &StackRequestSlotInfo) -> Result<(), ()> {
        let (from, to) = (self.named(inventory, source)?, self.named(inventory, destination)?);
        inventory.transfer(from, to, count)
    }
}

/// Does every action of `request`, or fails at the first that cannot be done as asked, or at
/// the end over a craft that did not use up what its recipe needs or, out of creative, over a
/// piece with Curse of Binding that left its slot.
fn apply(inventory: &mut Inventory, request: &Request, creative: bool, at: &At) -> Result<Applied, ()> {
    let bound = if creative { [None; ARMOR] } else { inventory.bound() };
    let mut applied = Applied::default();
    let mut craft: Option<Craft> = None;
    let mut forge: Option<Forge> = None;
    let station = at.station;
    for action in &request.actions {
        let unstarted = craft.is_none() && forge.is_none();
        match (&action.content, &at.bench) {
            (Action::CraftRecipe(asked), Some(bench)) if unstarted => forge = Some(Forge::by_recipe(inventory, bench, asked.recipe_network_id, creative)?),
            (Action::Optional(asked), Some(Bench::Anvil)) if unstarted => {
                let name = usize::try_from(asked.filtered_string_index).ok().and_then(|index| request.custom_names.get(index));
                forge = Some(Forge::repair(inventory, name.map(String::as_str), creative)?);
            }
            (Action::CraftGrindstoneRequest(asked), Some(Bench::Grindstone)) if unstarted => forge = Some(Forge::grind(inventory, asked.recipe_network_id, asked.cost)?),
            (Action::CraftLoomRequest(asked), Some(Bench::Loom)) if unstarted => forge = Some(Forge::loom(inventory, &asked.pattern)?),
            (Action::BeaconPayment(paid), Some(Bench::Beacon(level))) if unstarted => forge = Some(Forge::beacon(inventory, *level, paid.primary_effect, paid.secondary_effect)?),
            (Action::CraftRecipe(asked), None) if unstarted => craft = Some(Craft::start(inventory, asked.recipe_network_id, asked.times_crafted, station, true)?),
            (Action::CraftRecipeAuto(asked), None) if unstarted => craft = Some(Craft::start(inventory, asked.recipe_network_id, asked.times_crafted, station, false)?),
            (Action::Create(asked), _) => craft.as_mut().ok_or(())?.create(inventory, asked.result_slot_id.into())?,
            (Action::Consume(used), _) => {
                let from = applied.named(inventory, &used.source)?;
                let taken = inventory.take(from, used.count)?;
                match (craft.as_mut(), forge.as_mut()) {
                    (Some(craft), _) => craft.use_up(from, taken)?,
                    (_, Some(forge)) => forge.use_up(from, used.count),
                    _ => return Err(()),
                }
            }
            (Action::Take(take), _) => applied.transfer(inventory, take.count, &take.source, &take.destination)?,
            (Action::Place(place), _) => applied.transfer(inventory, place.count, &place.source, &place.destination)?,
            (Action::Swap(swap), _) => {
                let (a, b) = (applied.named(inventory, &swap.source)?, applied.named(inventory, &swap.destination)?);
                inventory.swap(a, b)?;
            }
            (Action::Drop(thrown), _) => {
                let from = applied.named(inventory, &thrown.source)?;
                applied.dropped.push(inventory.take(from, thrown.count)?);
            }
            // A beacon's payment is destroyed, not consumed.
            (Action::Destroy(destroy), _) if forge.is_some() => {
                let from = applied.named(inventory, &destroy.source)?;
                inventory.take(from, destroy.count)?;
                forge.as_mut().expect("matched above").use_up(from, destroy.count);
            }
            (Action::Destroy(destroy), _) if creative => {
                let from = applied.named(inventory, &destroy.source)?;
                inventory.take(from, destroy.count)?;
            }
            (Action::CraftCreative(pick), _) if creative => {
                let item = sapling_data::creative_item(pick.item_id).ok_or(())?;
                let made = ItemStack { name: item.name.to_owned(), count: MAX_COUNT, damage: item.damage, block: item.block, tag: item.tag.clone() };
                inventory.put(Place::Created, Some(made));
            }
            // Follows a pick with what the client expects of it; the server's own result counts.
            (Action::ResultsDeprecated(_), _) => {}
            _ => return Err(()),
        }
        if let Some(craft) = craft.as_mut() {
            craft.refill(inventory);
        }
    }
    inventory.put(Place::Created, None);
    let paid = craft.is_none_or(|craft| craft.is_paid()) && forge.as_ref().is_none_or(Forge::is_paid);
    applied.forged = forge.map(|forge| forge.done);
    (paid && inventory.keeps(&bound)).then_some(applied).ok_or(())
}

/// What BDS answers a request a container refused to hold an item in (a furnace's fuel slot asked
/// to take stone): this status, and nothing resent (research/captures/containers-findings.md D4).
const CANNOT_HOLD: Status = Status::Unknown(55);

fn refusal(request_id: i32, status: Status) -> ItemStackResponse {
    ItemStackResponse { responses: vec![ItemStackResponsesItem { status, request_id, containers: None }] }
}

fn response(inventory: &Inventory, request_id: i32, applied: &Applied) -> ItemStackResponse {
    let mut containers: Vec<Container> = Vec::new();
    for (name, slot, place) in &applied.touched {
        let held = inventory.slot(*place).and_then(Option::as_ref);
        let info = SlotInfo {
            slot: *slot,
            hotbar_slot: *slot,
            count: held.map_or(0, |held| held.stack.count),
            item_stack_id: held.map(|held| held.id),
            custom_name: String::new(),
            filtered_custom_name: String::new(),
            durability_correction: 0,
        };
        match containers.iter_mut().find(|container| container.slot_type == *name) {
            Some(container) => container.slots.push(info),
            None => containers.push(Container { slot_type: name.clone(), slots: vec![info] }),
        }
    }
    ItemStackResponse { responses: vec![ItemStackResponsesItem { status: Status::Ok, request_id, containers: Some(containers) }] }
}

/// Whether the handlers let every stack a request throws out go.
fn may_drop(world: &mut World, player: &Player, dropped: &[ItemStack]) -> bool {
    dropped.iter().all(|stack| {
        let event = plugins::offer(world,DropEvent::new(player.clone(), described(stack)));
        !event.is_cancelled()
    })
}

fn answer(world: &mut World, entity: Entity, player: &Player, request: &Request) {
    let creative = world.get::<Mode>(entity).is_some_and(|mode| mode.0 == GameMode::Creative);
    let Some(mut trial) = world.get::<Inventory>(entity).filter(|inventory| inventory.loaded).cloned() else { return };
    let open = containers::slots_of(world, entity);
    let at = &At { station: containers::station(world, entity), bench: workstations::bench(world, entity) };
    let refuses = containers::refusal(world, entity);
    (trial.container, trial.refuses) = (open.clone().unwrap_or_default(), refuses);
    let mut unrefused = Inventory { refuses: None, ..trial.clone() };
    // Tried on a copy first: the handlers see a drop before anything has left the inventory.
    let tried = apply(&mut trial, request, creative, at);
    let failed = tried.is_err();
    let allowed = tried.is_ok_and(|applied| may_drop(world, player,&applied.dropped));
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, entity) else { return };
    (inventory.container, inventory.refuses) = (open.unwrap_or_default(), refuses);
    let applied = if allowed { apply(&mut inventory, request, creative, at).ok() } else { None };
    // Made before the container's slots are taken back out (the answer names them), sent after
    // the container's slots are shown, as BDS does.
    let answer = applied.as_ref().map(|applied| response(&inventory, request.request_id, applied));
    let (container, _) = (std::mem::take(&mut inventory.container), inventory.refuses.take());
    match (applied, answer) {
        (Some(applied), Some(answer)) => {
            containers::store(world, entity, container);
            if let Some(mut out) = world.get_mut::<Outgoing>(entity) {
                out.push(&answer);
            }
            applied.dropped.into_iter().for_each(|stack| drops::throw(world, entity, stack));
            workstations::answered(world, entity, applied.forged);
        }
        // Only the container's refusal stood in the way: the client undoes it by itself.
        _ if failed && apply(&mut unrefused, request, creative, at).is_ok() => out.push(&refusal(request.request_id, CANNOT_HOLD)),
        _ => {
            tracing::debug!(player = %player.name, ?request, "refusing an inventory request");
            out.push(&refusal(request.request_id, Status::Error));
            inventory.send_all(&mut out, false);
            containers::resend(world, entity);
        }
    }
}

/// Answers this tick's inventory requests, in arrival order.
pub(crate) fn dispatch(world: &mut World) {
    let packets = world.resource_mut::<PacketQueues>().take::<ItemStackRequest>();
    let mut budget = PerTick::new(MAX_REQUESTS_PER_TICK);
    for (entity, packet) in packets {
        let Some(player) = in_world_identity(world, entity) else { continue };
        for request in packet.requests.iter().filter(|_| budget.allow(entity)) {
            answer(world, entity,&player, request);
        }
    }
}
