//! The slots an ItemStackRequest names, and what a request did to the inventory it was tried on.

use acacia_proto::types::{ContainerSlotType, FullContainerName, StackRequestSlotInfo};
use sapling_world::ItemStack;

use super::super::forge::{self, Forged};
use super::super::{ARMOR, Inventory, MAIN, Place, bundles};

/// The `Ui` slot of what a creative pick or a craft made.
const CREATED_SLOT: u8 = 50;
/// The `Ui` slots of the crafting grids: 28 to 31 on the inventory screen, 32 to 40 at a table.
const GRIDS: std::ops::RangeInclusive<usize> = 28..=40;
const STONECUTTER_INPUT: usize = 3;

/// What a request did to the inventory it was tried on.
#[derive(Default)]
pub(super) struct Applied {
    /// The slots it named, as it named them: the answer says what each holds now.
    pub touched: Vec<(FullContainerName, u8, Place)>,
    pub dropped: Vec<ItemStack>,
    pub forged: Option<Forged>,
}

pub(super) fn place(info: &StackRequestSlotInfo) -> Option<Place> {
    let slot = usize::from(info.slot);
    match info.slot_type.container_id {
        // Which slots there are is the open container's to say: none while nothing is open.
        // `Container` is the protocol's `LevelEntity`: chests, hoppers, dispensers and droppers alike.
        // A mount's tack slots are its inventory's 0 and 1, its chest's from 2 (riding/packs.rs).
        ContainerSlotType::Container | ContainerSlotType::Barrel | ContainerSlotType::Shulker | ContainerSlotType::Crafter | ContainerSlotType::HorseEquip => Some(Place::Container(slot)),
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
        ContainerSlotType::Dynamic if slot < bundles::SLOTS => Some(Place::Bundle(slot)),
        kind if forge::ui_slot(kind) == Some(slot) => Some(Place::Ui(slot)),
        _ => None,
    }
}

impl Applied {
    /// The slot a request names, if it holds the stack the client thinks it does.
    pub fn named(&mut self, inventory: &Inventory, info: &StackRequestSlotInfo) -> Result<Place, ()> {
        let place = place(info).ok_or(())?;
        let lent = inventory.bundle.as_ref().and_then(|(id, _)| u32::try_from(*id).ok());
        if matches!(place, Place::Bundle(_)) && info.slot_type.dynamic_container_id != lent {
            return Err(());
        }
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

    pub fn transfer(&mut self, inventory: &mut Inventory, count: u8, source: &StackRequestSlotInfo, destination: &StackRequestSlotInfo) -> Result<(), ()> {
        let (from, to) = (self.named(inventory, source)?, self.named(inventory, destination)?);
        inventory.transfer(from, to, count)
    }
}
