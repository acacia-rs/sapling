//! Game data a client needs before it can spawn: StartGame defaults, the registry packets and the
//! player's own state packets. All of it is recorded from BDS and patched; see README.md.

mod armour;
mod block_defaults;
mod block_loot;
mod block_resistance;
mod clock;
mod command_texts;
pub mod enchant;
mod entity;
mod entity_properties;
mod families;
mod food;
mod game_rules;
pub mod gear;
mod inventory;
mod item_entity;
mod items;
mod liquid_reaction;
mod map_colours;
mod attack;
mod mob_health;
mod mob_loot;
mod names;
mod command_names;
mod player;
mod recipes;
pub mod shots;
mod spawn;
mod spawn_rules;
mod start_game;
mod strides;
mod templates;
mod husbandry;
mod raised_look;
pub mod trades;

pub use block_defaults::default_properties;
pub use block_resistance::blast_resistance;
pub use block_loot::{BlockLoot, Fortune, LootRow, StateTest, Tooling, block_loot, java_block, ripe_age};
pub use clock::{clock_state, world_clocks};
pub use command_texts::command_text;
pub use names::{entity_display_name, item_display_name};
pub use command_names::{command_takes_block, command_takes_item};
pub use entity::{AddEntityOptions, add_entity, add_experience_orb, add_falling_block, add_painting, announced_flags, mob_health, mob_size};
pub use entity_properties::{EntityProperty, PropertyShape, entity_properties, entity_property, property_registry_packets, property_values};
pub use attack::item_attack;
pub use families::{has_family, is_arthropod, is_undead, mob_families};
pub use armour::{armour, armour_lasts};
pub use food::{Food, Gives, consumable, food, potion_effects};
pub use mob_health::{MobTraits, mob_air, mob_breath, visible_effects, grown_flags, mob_health_update, mob_traits, Hunts, mob_charge, mob_flags_set, mob_fuse, mob_spell};
pub use liquid_reaction::{LiquidDetection, LiquidReaction, liquid_detection};
pub use map_colours::{MapColour, Tint, map_colour, tint_colour};
pub use mob_loot::{Caught, Killer, fishing_loot, has_loot_table, mob_loot, table_loot, table_loot_for};
pub use inventory::{block_screen,inventory_content, inventory_screen, player_hotbar};
pub use item_entity::{add_item_entity, mob_armor, mob_hand};
pub use items::{CreativeItem, STORAGE_CONTENT, Stack, creative_item, is_item, item, item_name, max_stack, no_item, set_shield_item_id, static_item_name};
pub use recipes::{
    Grid, Ingredient, Recipe, Shape, Station, UnlockContext, brewed, has_tag, initial_recipes, is_brewing_bottle, is_brewing_ingredient, is_special as is_special_recipe, laid_out_in, multi_recipe, recipe, recipe_ids,
    smelted, smithed, unlocked_by,
};
pub use gear::{Gear, Made, gear};
pub use spawn::{no_player_spawn, world_spawn};
pub use spawn_rules::{Medium, SpawnRule, Spot, biome_has_tag, biome_id, despawn_distances, spawn_category, spawn_rules};
pub use player::{AddPlayerOptions, add_player, entity_data, experience_attributes, player_flags_extended, player_metadata, update_abilities, update_attributes};
pub use game_rules::{DAYLIGHT_CYCLE, GameRules, HUNGER, LIQUID_FLOW, MOB_AI, REDSTONE, RuleValue};
pub use start_game::{EYE_HEIGHT, StartGameOptions, start_game};
pub use strides::{Strides, dress_strides, has_strides, kept_strides, roll_strides, stride_attributes};
pub use templates::{pre_start_packets, registry_packets};
pub use husbandry::{Husbandry, Swap, husbandry};
pub use raised_look::{mob_age_set, mob_colour_set, mob_links_set, mob_look, mob_name_set};
