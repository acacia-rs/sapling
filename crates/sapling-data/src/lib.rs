//! Game data a client needs before it can spawn: StartGame defaults, the registry packets and the
//! player's own state packets. All of it is recorded from BDS and patched; see README.md.

mod clock;
mod entity;
mod player;
mod start_game;
mod templates;

pub use clock::{clock_state, world_clocks};
pub use entity::{AddEntityOptions, add_entity};
pub use player::{AddPlayerOptions, add_player, entity_data, player_metadata, update_abilities, update_attributes};
pub use start_game::{EYE_HEIGHT, StartGameOptions, start_game};
pub use templates::{pre_start_packets, registry_packets};
