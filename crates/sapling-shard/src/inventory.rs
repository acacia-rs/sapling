//! A player's items: the server's own copy, which every client request is checked against
//! (docs/DESIGN.md "Inventories"). Stacks carry the ids clients name them by.

mod crafting;
mod equipment;
mod requests;
mod store;
mod wear;

use acacia_proto::types::{ItemV4, WindowIDVarint};
use bevy_ecs::prelude::*;
use sapling_api::{Item, Slot};
use sapling_data::Stack;
use sapling_world::ItemStack;

use crate::components::Outgoing;
use crate::experience::Experience;

pub(crate) use crafting::clear_grid;
pub(crate) use equipment::{announce, in_hand, screens, select, spawn_packets};
pub(crate) use requests::dispatch;
pub(crate) use store::{keep_here, load, save, save_all, save_changed, send_own};
pub(crate) use wear::wear_tool_in_hand;

pub(crate) const MAIN: usize = 36;
pub(crate) const HOTBAR: usize = 9;
const ARMOR: usize = 4;
/// Slots of the container every ender chest opens for its player.
pub(crate) const ENDER: usize = 27;
/// The `Ui` window's slots; the first is the cursor.
const UI: usize = 54;
/// The most a client stacks of anything; an item's own limit is the client's to keep
/// (`sapling_data::max_stack` only guides what the server itself hands out).
const MAX_COUNT: u8 = 64;

/// A stack in a slot, under the id the client knows it by.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Held {
    pub stack: ItemStack,
    pub id: i32,
}

/// A slot a request can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Place {
    Main(usize),
    Armor(usize),
    Offhand,
    Cursor,
    /// What a creative pick made, until the request that made it ends.
    Created,
    /// A slot of the open container, while a request is answered (containers.rs).
    Container(usize),
    /// A slot of the `Ui` window past the cursor, where screens keep what is being worked on:
    /// the crafting grids' cells, a stonecutter's input.
    Ui(usize),
}

#[derive(Component, Clone)]
pub(crate) struct Inventory {
    main: Vec<Option<Held>>,
    armor: Vec<Option<Held>>,
    offhand: Option<Held>,
    cursor: Option<Held>,
    created: Option<Held>,
    /// The `Ui` window's slots by number; the first is never used (the cursor is its own field).
    grid: Vec<Option<Held>>,
    /// The open container's slots, lent for the length of a request.
    pub container: Vec<Option<Held>>,
    /// What the open container refuses in a slot, while it is lent.
    pub refuses: Option<fn(usize, &ItemStack) -> bool>,
    /// What the player's ender chests hold.
    pub ender: Vec<Option<Held>>,
    pub experience: Experience,
    /// The hotbar slot in the hand.
    pub selected: u8,
    next_id: i32,
    /// Storage answered: until then the player has no items and may not ask for any.
    pub loaded: bool,
    /// Whether storage may be written: not over a record it could not read.
    pub keep: bool,
    pub unsaved: bool,
    /// The client was sent its windows.
    pub sent: bool,
    /// What viewers were last told of the hand, the armour and the offhand.
    pub shown: Option<[i32; 6]>,
    /// The id of the last inventory screen opened.
    pub window: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            main: vec![None; MAIN],
            armor: vec![None; ARMOR],
            offhand: None,
            cursor: None,
            created: None,
            grid: vec![None; UI],
            container: Vec::new(),
            refuses: None,
            ender: vec![None; ENDER],
            experience: Experience::default(),
            selected: 0,
            next_id: 1,
            loaded: false,
            keep: false,
            unsaved: false,
            sent: false,
            shown: None,
            window: 1,
        }
    }
}

pub(crate) fn stack(stored: &ItemStack) -> Stack<'_> {
    Stack { name: &stored.name, count: stored.count, damage: stored.damage, block: stored.block, tag: stored.tag.as_ref() }
}

pub(crate) fn wire(slot: &Option<Held>, with_id: bool) -> ItemV4 {
    let shown = |held: &Held| sapling_data::item(&stack(&held.stack), with_id.then_some(held.id));
    slot.as_ref().and_then(shown).unwrap_or_else(sapling_data::no_item)
}

/// Whether two stacks are of one kind, so that one can go onto the other.
pub(crate) fn stacks_with(a: &ItemStack, b: &ItemStack) -> bool {
    (&a.name, a.damage, a.block, &a.tag) == (&b.name, b.damage, b.block, &b.tag)
}

impl Inventory {
    pub fn slot(&self, place: Place) -> Option<&Option<Held>> {
        match place {
            Place::Main(slot) => self.main.get(slot),
            Place::Armor(slot) => self.armor.get(slot),
            Place::Offhand => Some(&self.offhand),
            Place::Cursor => Some(&self.cursor),
            Place::Created => Some(&self.created),
            Place::Container(slot) => self.container.get(slot),
            Place::Ui(slot) => self.grid.get(slot),
        }
    }

    fn slot_mut(&mut self, place: Place) -> Option<&mut Option<Held>> {
        self.unsaved = true;
        match place {
            Place::Main(slot) => self.main.get_mut(slot),
            Place::Armor(slot) => self.armor.get_mut(slot),
            Place::Offhand => Some(&mut self.offhand),
            Place::Cursor => Some(&mut self.cursor),
            Place::Created => Some(&mut self.created),
            Place::Container(slot) => self.container.get_mut(slot),
            Place::Ui(slot) => self.grid.get_mut(slot),
        }
    }

    /// Puts a stack into a slot under a new id, or empties it; false for a slot that is not there.
    pub fn put(&mut self, place: Place, stack: Option<ItemStack>) -> bool {
        let held = stack.filter(|stack| stack.count > 0).map(|stack| Held { stack, id: self.new_id() });
        self.slot_mut(place).map(|slot| *slot = held).is_some()
    }

    fn new_id(&mut self) -> i32 {
        self.next_id += 1;
        self.next_id - 1
    }

    /// Moves `count` of the stack at `from` to `to`: into an empty slot, or onto a stack of the
    /// same kind. A whole stack keeps its id; a part gets a new one.
    pub fn transfer(&mut self, from: Place, to: Place, count: u8) -> Result<(), ()> {
        let source = self.slot(from).and_then(Option::as_ref).filter(|held| count > 0 && held.stack.count >= count).cloned().ok_or(())?;
        if from != to && self.is_refused(to, &source.stack) {
            return Err(());
        }
        let target = self.slot(to).ok_or(())?.clone();
        let moved = match target {
            None if source.stack.count == count => source.clone(),
            None => Held { stack: ItemStack { count, ..source.stack.clone() }, id: self.new_id() },
            Some(held) if from != to && stacks_with(&held.stack, &source.stack) && held.stack.count <= MAX_COUNT - count => {
                Held { stack: ItemStack { count: held.stack.count + count, ..held.stack }, id: held.id }
            }
            Some(_) => return Err(()),
        };
        self.take(from, count)?;
        *self.slot_mut(to).ok_or(())? = Some(moved);
        Ok(())
    }

    pub fn swap(&mut self, a: Place, b: Place) -> Result<(), ()> {
        let (first, second) = (self.slot(a).ok_or(())?.clone(), self.slot(b).ok_or(())?.clone());
        let refused = |place, held: &Option<Held>| held.as_ref().is_some_and(|held| self.is_refused(place, &held.stack));
        if refused(a, &second) || refused(b, &first) {
            return Err(());
        }
        *self.slot_mut(a).ok_or(())? = second;
        *self.slot_mut(b).ok_or(())? = first;
        Ok(())
    }

    /// Whether the open container will not have `stack` in `place`.
    fn is_refused(&self, place: Place, stack: &ItemStack) -> bool {
        matches!((place, self.refuses), (Place::Container(slot), Some(refuses)) if refuses(slot, stack))
    }

    /// Removes `count` of the stack at `place` and returns them.
    pub fn take(&mut self, place: Place, count: u8) -> Result<ItemStack, ()> {
        let slot = self.slot_mut(place).ok_or(())?;
        let held = slot.as_mut().filter(|held| count > 0 && held.stack.count >= count).ok_or(())?;
        let taken = ItemStack { count, ..held.stack.clone() };
        held.stack.count -= count;
        if held.stack.count == 0 {
            *slot = None;
        }
        Ok(taken)
    }

    /// Adds a stack to the hotbar and inventory, onto stacks of its kind first, then into empty
    /// slots; returns how many did not fit.
    pub fn add(&mut self, stack: &ItemStack) -> u8 {
        let (limit, mut left) = (sapling_data::max_stack(&stack.name), stack.count);
        for slot in 0..MAIN {
            let Some(held) = self.main[slot].as_mut().filter(|held| stacks_with(&held.stack, stack) && held.stack.count < limit) else { continue };
            let fits = left.min(limit - held.stack.count);
            (held.stack.count, left) = (held.stack.count + fits, left - fits);
        }
        for slot in 0..MAIN {
            if left > 0 && self.main[slot].is_none() {
                let fits = left.min(limit);
                self.put(Place::Main(slot), Some(ItemStack { count: fits, ..stack.clone() }));
                left -= fits;
            }
        }
        self.unsaved |= left < stack.count;
        left
    }

    /// Whether [`Inventory::add`] would take any of the stack.
    pub fn has_room_for(&self, stack: &ItemStack) -> bool {
        let limit = sapling_data::max_stack(&stack.name);
        self.main.iter().any(|slot| slot.as_ref().is_none_or(|held| stacks_with(&held.stack, stack) && held.stack.count < limit))
    }

    /// Empties every slot; how many items went.
    pub fn clear(&mut self) -> u32 {
        let slots = self.main.iter().chain(&self.armor).chain([&self.offhand, &self.cursor]);
        let removed = slots.flatten().map(|held| u32::from(held.stack.count)).sum();
        (self.main, self.armor) = (vec![None; MAIN], vec![None; ARMOR]);
        (self.offhand, self.cursor, self.unsaved) = (None, None, true);
        removed
    }

    pub fn in_hand(&self) -> &Option<Held> {
        &self.main[usize::from(self.selected) % HOTBAR]
    }

    /// The ids of what viewers see: hand, helmet, chestplate, leggings, boots, offhand.
    pub fn worn(&self) -> [i32; 6] {
        let id = |slot: &Option<Held>| slot.as_ref().map_or(0, |held| held.id);
        [id(self.in_hand()), id(&self.armor[0]), id(&self.armor[1]), id(&self.armor[2]), id(&self.armor[3]), id(&self.offhand)]
    }

    /// One of the player's own windows as its client is sent it.
    fn window(&self, window: WindowIDVarint) -> Vec<ItemV4> {
        match window {
            WindowIDVarint::Inventory => self.main.iter().map(|slot| wire(slot, true)).collect(),
            // The fifth is the body slot, which players leave empty.
            WindowIDVarint::Armor => self.armor.iter().map(|slot| wire(slot, true)).chain([sapling_data::no_item()]).collect(),
            WindowIDVarint::Offhand => vec![wire(&self.offhand, true)],
            _ => {
                let shown = |slot: usize| if slot == 0 { Some(&self.cursor) } else { self.grid.get(slot) };
                (0..UI).map(|slot| shown(slot).map_or_else(sapling_data::no_item, |held| wire(held, true))).collect()
            }
        }
    }

    /// Sends the client every window, as BDS does at join and after `/give`.
    pub fn send_all(&self, out: &mut Outgoing) {
        for window in [WindowIDVarint::Inventory, WindowIDVarint::Armor, WindowIDVarint::Ui, WindowIDVarint::Offhand] {
            out.push(&sapling_data::inventory_content(window, self.window(window)));
        }
    }
}

fn place(slot: Slot) -> Place {
    match slot {
        Slot::Main(slot) => Place::Main(slot.into()),
        Slot::Armor(slot) => Place::Armor(slot.into()),
        Slot::Offhand => Place::Offhand,
    }
}

/// A plugin's item as a stack: a block item places its block's first state.
pub(crate) fn stored(item: &Item, registry: &sapling_world::BlockRegistry) -> Option<ItemStack> {
    let block = registry.states_of(&item.name).next().map(|(id, _)| registry.network_hash(id));
    sapling_data::is_item(&item.name).then(|| ItemStack { name: item.name.clone(), count: item.count, damage: item.damage, block, tag: None })
}

pub(crate) fn described(stack: &ItemStack) -> Item {
    Item { name: stack.name.clone(), count: stack.count, damage: stack.damage }
}

/// What a slot holds, for a plugin.
pub(crate) fn item_at(world: &World, player: Entity, slot: Slot) -> Option<Item> {
    world.get::<Inventory>(player)?.slot(place(slot))?.as_ref().map(|held| described(&held.stack))
}

/// Runs `change` on a player's inventory and shows the client the result.
pub(crate) fn change<T>(world: &mut World, player: Entity, change: impl FnOnce(&mut Inventory) -> T) -> Option<T> {
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let (mut inventory, mut out) = query.get_mut(world, player).ok()?;
    let result = change(&mut inventory);
    if inventory.sent {
        inventory.send_all(&mut out);
    }
    Some(result)
}

/// Sets a slot for a plugin; false for a slot that is not there.
pub(crate) fn set_item(world: &mut World, player: Entity, slot: Slot, stack: Option<ItemStack>) -> bool {
    change(world, player, |inventory| inventory.put(place(slot), stack)) == Some(true)
}
