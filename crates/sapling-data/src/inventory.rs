//! A player's own inventory on the wire, in the shapes BDS sends at join (`extract-data --show
//! InventoryContent,PlayerHotbar`) and on the inventory screen (acacia
//! docs/research/vanilla-actions-2026-10-02.md).

use acacia_proto::packets::{ContainerOpen, InventoryContent, PlayerHotbar};
use acacia_proto::types::{BlockCoordinates, ContainerSlotType, FullContainerName, ItemV4, WindowID, WindowIDVarint, WindowType};

use crate::items::no_item;

/// Every slot of one of the player's windows: `Inventory` (36), `Armor` (4 and the body slot),
/// `Ui` (54, the first is the cursor) or `Offhand` (1).
pub fn inventory_content(window_id: WindowIDVarint, input: Vec<ItemV4>) -> InventoryContent {
    // BDS names no container here: the id is the zero one.
    let container = FullContainerName { container_id: ContainerSlotType::AnvilInput, dynamic_container_id: None };
    InventoryContent { window_id, input, container, storage_item: no_item() }
}

/// Which hotbar slot is in the hand; `select_slot` makes the client switch to it.
pub fn player_hotbar(selected_slot: u8, select_slot: bool) -> PlayerHotbar {
    PlayerHotbar { selected_slot: selected_slot.into(), window_id: WindowID::Inventory, select_slot }
}

/// The screen of the block at a position: `Container` for a chest or barrel, whose items follow
/// as `InventoryContent` for the same window, `Workbench` for a crafting table.
pub fn block_screen(window_id: WindowID, window_type: WindowType, [x, y, z]: [i32; 3]) -> ContainerOpen {
    ContainerOpen { window_id, window_type, coordinates: BlockCoordinates { x, y, z }, runtime_entity_id: -1 }
}

/// The answer to a client opening its inventory screen, where the player stands.
pub fn inventory_screen(window_id: WindowID, [x, y, z]: [i32; 3], runtime_entity_id: i64) -> ContainerOpen {
    ContainerOpen { window_id, window_type: WindowType::Inventory, coordinates: BlockCoordinates { x, y, z }, runtime_entity_id }
}
