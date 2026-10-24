//! Game data a client needs before it can spawn: StartGame defaults, the registry packets and the
//! player's own state packets. All of it is recorded from BDS and patched; see README.md.

mod armour;
mod block_defaults;
mod block_loot;
mod clock;
mod entity;
mod food;
mod game_rules;
mod inventory;
mod item_entity;
mod items;
mod liquid_reaction;
mod attack;
mod mob_health;
mod mob_loot;
mod names;
mod player;
mod recipes;
mod spawn;
mod start_game;
mod templates;

pub use block_defaults::default_properties;
pub use block_loot::{BlockLoot, Fortune, LootRow, StateTest, Tooling, block_loot, java_block, ripe_age};
pub use clock::{clock_state, world_clocks};
pub use names::{entity_display_name, item_display_name};
pub use entity::{AddEntityOptions, add_entity, add_experience_orb, add_falling_block, add_painting, announced_flags, mob_health, mob_size};
pub use attack::item_attack;
pub use armour::{armour, armour_lasts};
pub use food::{Food, Gives, consumable, food, potion_effects};
pub use mob_health::{MobTraits, mob_air, mob_breath,grown_flags, mob_health_update, mob_traits, Hunts, mob_charge, mob_flags_set, mob_fuse, mob_spell};
pub use liquid_reaction::{LiquidDetection, LiquidReaction, liquid_detection};
pub use mob_loot::mob_loot;
pub use inventory::{block_screen,inventory_content, inventory_screen, player_hotbar};
pub use item_entity::{add_item_entity, mob_armor, mob_hand};
pub use items::{CreativeItem, Stack, creative_item, is_item, item, item_name, max_stack, no_item, set_shield_item_id};
pub use recipes::{Grid, Ingredient, Recipe, Shape, Station, has_tag, recipe, smelted};
pub use spawn::{no_player_spawn, world_spawn};
pub use player::{AddPlayerOptions, add_player, entity_data, experience_attributes, player_metadata, update_abilities, update_attributes};
pub use game_rules::{DAYLIGHT_CYCLE, GameRules, HUNGER, LIQUID_FLOW, MOB_AI, REDSTONE, RuleValue};
pub use start_game::{EYE_HEIGHT, StartGameOptions, start_game};
pub use templates::{pre_start_packets, registry_packets};
