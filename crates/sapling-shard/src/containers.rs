//! Blocks that hold items (chests, barrels, shulker boxes, ender chests, hoppers, brewing stands,
//! dispensers, droppers and the three furnaces), kept in the block's entity as the game keeps them, shown to the players who have
//! the block open (screens.rs) and changed by their inventory requests (inventory/requests.rs),
//! by a furnace's burning (furnaces.rs) and by hoppers (hoppers.rs). Two paired chests are one
//! container of 54 (pairs.rs); an ender chest opens its player's own 27 slots. What BDS does, and
//! the evidence: docs/research/containers-crafting.md, shulker-ender.md, hoppers.md, furnaces.md.

mod access;
mod brewing;
mod lid;
mod pairs;
mod screens;
mod shulker;
mod slots;

pub(crate) use access::close_where_gone;

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::types::{WindowIDVarint, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::{ItemStack, container_items, new_block_entity};

// TODO: a dispenser or dropper only holds its 9 slots: it throws nothing out when powered, and
// its screen was never opened by a client.

use crate::blocks::keep_entity;
use crate::components::Layer;
use crate::inventory::{Held, wire};

pub(crate) use lid::show_turned_barrels;
pub(crate) use screens::{close, dispatch_closes, open, show_carried, show_lids_again, station, workstation};
pub(crate) use shulker::is_shulker_box;
pub(crate) use slots::{carried_viewers, new_held, refusal, resend, set_slots, slots_at, slots_of, spill, store, tell_changed, viewers};

/// Stack ids of what a container held when it was opened; players' own count up from 1.
const FIRST_ID: i32 = 1 << 24;

/// What kind of container a block is.
#[derive(Clone, Copy)]
pub(crate) struct Kind {
    /// The `id` of its block entity.
    entity: &'static str,
    /// A double chest has them twice, the lead half's first.
    slots: usize,
    window: WindowType,
    /// Whether those near see it open and shut.
    has_lid: bool,
    /// What it will not hold in a slot.
    refuses: Option<fn(usize, &ItemStack) -> bool>,
    /// An ender chest: what it holds is its player's.
    is_per_player: bool,
}

const CHEST: Kind = Kind { entity: "Chest", slots: 27, window: WindowType::Container, has_lid: true, refuses: None, is_per_player: false };
pub(crate) const HOPPER: Kind = Kind { entity: "Hopper", slots: 5, window: WindowType::Hopper, has_lid: false, ..CHEST };
const SMELTER_SLOTS: usize = 3;

impl Kind {
    fn is_smelter(self) -> bool {
        self.slots == SMELTER_SLOTS
    }

    /// Whether it keeps its own slots, which hoppers can reach.
    pub fn holds_items(self) -> bool {
        !self.is_per_player
    }

    pub fn is_hopper(self) -> bool {
        self.entity == HOPPER.entity
    }

    /// Whether a hopper may put `item` into `slot` through the container's face `face`
    /// (BDS `canPushInItem`; docs/research/hoppers.md, "Container rules").
    pub fn takes_from_hopper(self, slot: usize, face: Face, item: &ItemStack) -> bool {
        if self.is_smelter() { crate::furnaces::takes_from_hopper(slot, face, item) } else { self.refuses.is_none_or(|refuses| !refuses(slot, item)) }
    }

    /// Whether a hopper may take `item` out of `slot` through the container's face `face`
    /// (BDS `canPullOutItem`).
    pub fn gives_to_hopper(self, slot: usize, face: Face, item: &ItemStack) -> bool {
        !self.is_smelter() || crate::furnaces::gives_to_hopper(slot, face, item)
    }
}

pub(crate) fn kind(block: &str) -> Option<Kind> {
    let smelter = |entity, window| Kind { entity, slots: SMELTER_SLOTS, window, has_lid: false, refuses: Some(crate::furnaces::refuses), is_per_player: false };
    Some(match block {
        "minecraft:chest" | "minecraft:trapped_chest" => CHEST,
        "minecraft:barrel" => Kind { entity: "Barrel", ..CHEST },
        "minecraft:ender_chest" => Kind { entity: "EnderChest", is_per_player: true, ..CHEST },
        "minecraft:hopper" => HOPPER,
        "minecraft:dispenser" => Kind { entity: "Dispenser", slots: 9, window: WindowType::Dispenser, has_lid: false, ..CHEST },
        "minecraft:dropper" => Kind { entity: "Dropper", slots: 9, window: WindowType::Dropper, has_lid: false, ..CHEST },
        "minecraft:brewing_stand" => Kind { entity: "BrewingStand", slots: 5, window: WindowType::BrewingStand, has_lid: false, ..CHEST },
        "minecraft:furnace" | "minecraft:lit_furnace" => smelter("Furnace", WindowType::Furnace),
        "minecraft:blast_furnace" | "minecraft:lit_blast_furnace" => smelter("BlastFurnace", WindowType::BlastFurnace),
        "minecraft:smoker" | "minecraft:lit_smoker" => smelter("Smoker", WindowType::Smoker),
        _ if is_shulker_box(block) => Kind { entity: "ShulkerBox", refuses: Some(shulker::refuses), ..CHEST },
        _ => return None,
    })
}

pub(crate) fn block_name(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = &world.get::<Layer>(layer)?.world;
    level.registry().get(level.block(x, y, z)).map(|state| state.name)
}

/// The kind of container the block at `pos` is.
pub(crate) fn kind_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Kind> {
    block_name(world, layer, pos).and_then(kind)
}

struct Chest {
    /// Its cells, the lead half first.
    halves: Vec<BlockPos>,
    per_half: usize,
    /// Empty for an ender chest, whose slots are each viewer's own.
    slots: Vec<Option<Held>>,
    viewers: Vec<Entity>,
    /// The tick its first viewer opened it, from which its open lid is shown again.
    opened_at: u64,
    /// Ticking (a furnace burning, a hopper): kept while nobody looks.
    pinned: bool,
}

/// The containers whose slots the shard holds: each while someone has it open or it ticks. By
/// their lead half.
#[derive(Resource, Default)]
pub(crate) struct OpenContainers {
    chests: HashMap<(Entity, BlockPos), Chest>,
    ids: i32,
}

fn opened(world: &mut World) -> Mut<'_, OpenContainers> {
    world.get_resource_or_insert_with(OpenContainers::default)
}

fn content(window: u8, slots: &[Option<Held>]) -> acacia_proto::packets::InventoryContent {
    sapling_data::inventory_content(WindowIDVarint::from_raw(window.into()), slots.iter().map(|slot| wire(slot, true)).collect())
}

/// Gives a container block that was just placed at `pos` its block entity: empty, or for a
/// shulker box what the placed item held (`item_tag`) and the face it was put against. A chest
/// gets its other half if one stands beside it; a block that has its entity keeps it, and other
/// blocks are left alone. A hopper starts ticking.
pub(crate) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, block: &str, item_tag: Option<&Value>, face: Face) {
    let Some(kind) = kind(block) else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    if level.block_entity(pos).is_none() {
        let mut entity = new_block_entity(kind.entity, pos);
        if is_shulker_box(block) {
            entity = shulker::placed(entity, item_tag, face, level.registry());
        }
        keep_entity(world, layer, pos, &entity);
        pairs::pair_placed(world, layer, pos);
    }
    if kind.is_hopper() {
        crate::hoppers::watch(world, layer, pos);
    }
}

/// The first `count` slots of what the block entity at `pos` holds, under new stack ids.
fn stored_slots(world: &mut World, layer: Entity, pos: BlockPos, count: usize) -> Vec<Option<Held>> {
    let items = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).map(|entity| container_items(&entity.nbt)).unwrap_or_default();
    let mut slots = vec![None; count];
    for (slot, stack) in items.into_iter().filter(|(slot, _)| usize::from(*slot) < count) {
        slots[usize::from(slot)] = Some(new_held(world, stack));
    }
    slots
}

/// Makes the container at `pos` one the shard holds, from its block entities, unless it is
/// already; returns its lead half. `None` for a block that is no container.
fn hold(world: &mut World, layer: Entity, pos: BlockPos) -> Option<BlockPos> {
    let kind = kind_at(world, layer, pos)?;
    let halves = if kind.entity == CHEST.entity { pairs::halves(world, layer, pos) } else { vec![pos] };
    let key = (layer, halves[0]);
    if !opened(world).chests.contains_key(&key) {
        let mut slots = Vec::new();
        for half in halves.iter().filter(|_| kind.holds_items()) {
            slots.extend(stored_slots(world, layer, *half, kind.slots));
        }
        opened(world).chests.insert(key, Chest { halves, per_half: kind.slots, slots, viewers: Vec::new(), opened_at: 0, pinned: false });
    }
    Some(key.1)
}

/// Keeps the container at `pos` held while it ticks; false for a block that is no container.
pub(crate) fn pin(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(lead) = hold(world, layer, pos) else { return false };
    if let Some(chest) = opened(world).chests.get_mut(&(layer, lead)) {
        chest.pinned = true;
    }
    true
}

/// Lets go of a container that stopped ticking, unless someone looks into it.
pub(crate) fn unpin(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(chest) = opened(world).chests.get_mut(&(layer, pos)) {
        chest.pinned = false;
    }
    slots::release(world, layer, pos);
}

/// Runs `change` on the slots of the container at `pos` (a double chest's 54) and keeps what it
/// leaves them as; `None` for a block that holds no items of its own.
pub(crate) fn with_slots<T>(world: &mut World, layer: Entity, pos: BlockPos, change: impl FnOnce(&mut World, &mut Vec<Option<Held>>) -> T) -> Option<T> {
    kind_at(world, layer, pos).filter(|kind| kind.holds_items())?;
    let lead = hold(world, layer, pos)?;
    let mut slots = slots_at(world, layer, lead)?;
    let result = change(world, &mut slots);
    set_slots(world, layer, lead, slots);
    slots::release(world, layer, lead);
    Some(result)
}
