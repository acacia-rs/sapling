//! Game data a client needs before it can spawn: StartGame defaults, the registry packets and the
//! player's own state packets. All of it is recorded from BDS and patched; see README.md.

mod block_defaults;
mod block_loot;
mod clock;
mod entity;
mod game_rules;
mod inventory;
mod item_entity;
mod items;
mod liquid_reaction;
mod mob_health;
mod mob_loot;
mod player;
mod recipes;
mod spawn;
mod start_game;
mod templates;

pub use block_defaults::default_properties;
pub use block_loot::{BlockLoot, Fortune, LootRow, StateTest, Tooling, block_loot, java_block, ripe_age};
pub use clock::{clock_state, world_clocks};
pub use entity::{AddEntityOptions, add_entity, add_experience_orb, add_falling_block, add_painting, mob_health, mob_size};
pub use mob_health::{is_fire_immune, mob_fire_flags, mob_health_update};
pub use liquid_reaction::{LiquidDetection, LiquidReaction, liquid_detection};
pub use mob_loot::mob_loot;
pub use inventory::{block_screen,inventory_content, inventory_screen, player_hotbar};
pub use item_entity::{add_item_entity, mob_armor, mob_hand};
pub use items::{CreativeItem, Stack, creative_item, is_item, item, item_name, max_stack, no_item, set_shield_item_id};
pub use recipes::{Grid, Ingredient, Recipe, Shape, Station, has_tag, recipe, smelted};
pub use spawn::{no_player_spawn, world_spawn};
pub use player::{AddPlayerOptions, add_player, entity_data, experience_attributes, player_metadata, update_abilities, update_attributes};
pub use game_rules::{DAYLIGHT_CYCLE, GameRules, RuleValue};
pub use start_game::{EYE_HEIGHT, StartGameOptions, start_game};
pub use templates::{pre_start_packets, registry_packets};
