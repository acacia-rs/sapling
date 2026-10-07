//! sapling: a framework for Minecraft: Bedrock Edition servers. A server is a binary that builds a
//! [`Server`] and adds handlers; see `examples/hub.rs` and docs/DESIGN.md.
//!
//! ```no_run
//! # async fn run() -> Result<(), sapling::StartError> {
//! let server = sapling::Server::builder().bind("0.0.0.0:19132".parse().unwrap()).start().await?;
//! println!("listening on {}", server.local_addr());
//! # server.shutdown().await;
//! # Ok(()) }
//! ```

mod layout;
mod server;
pub mod vanilla;

pub use sapling_api::{
    Block, BlockPos, BossBar, BossBarColour, BreakEvent, ChatEvent, Command, CommandEnum, CommandEvent, Custom, Difficulty, DropEvent, EntityId, EntityInfo, Face, Form, FormResponse, GameMode,
    Handler, InteractEvent, Item, Menu, Modal, Output, Permission, PlaceEvent, Player, PlayerId, Position, Priority, RuleValue, Sender, ShardCtx, Sidebar, Slot, Title, command, form,
};
pub use sapling_net::{Advert, Lan, NetherNet, host_key};
pub use sapling_world::{ChunkPos, Level, LevelDbStorage, MemoryStorage, Template, World, WorldFolder, WorldStorage};
pub use server::{Server, ServerBuilder, StartError};
