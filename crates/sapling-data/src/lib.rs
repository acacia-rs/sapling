//! Game data a client needs before it can spawn: StartGame defaults, the registry packets and the
//! player's own state packets. All of it is recorded from BDS and patched; see README.md.

mod block_loot;
mod clock;
mod entity;
mod inventory;
mod item_entity;
mod items;
mod player;
mod recipes;
mod spawn;
mod start_game;
mod templates;

pub use block_loot::{BlockLoot, Fortune, LootRow, StateTest, Tooling, block_loot, java_block, ripe_age};
pub use clock::{clock_state, world_clocks};
pub use entity::{AddEntityOptions, add_entity, add_painting};
pub use inventory::{block_screen,inventory_content, inventory_screen, player_hotbar};
pub use item_entity::{add_item_entity, mob_armor, mob_hand};
pub use items::{CreativeItem, Stack, creative_item, is_item, item, max_stack, no_item, set_shield_item_id};
pub use recipes::{Ingredient, Recipe, Station, recipe};
pub use spawn::{no_player_spawn, world_spawn};
pub use player::{AddPlayerOptions, add_player, entity_data, player_metadata, update_abilities, update_attributes};
pub use start_game::{EYE_HEIGHT, StartGameOptions, game_rules, start_game};
pub use templates::{pre_start_packets, registry_packets};
