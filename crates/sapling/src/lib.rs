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

mod console;
mod functions;
mod layout;
pub mod packs;
mod server;
pub mod vanilla;

pub use sapling_api::{
    Block, BlockPos, BossBar, BossBarColour, BreakEvent, ChatEvent, Command, CommandEnum, CommandEvent, Custom, Difficulty, DropEvent, EntityId, EntityInfo, Face, Form, FormResponse, GameMode,
    Handler, InteractEvent, Item, Menu, Modal, MovementPolicy, Output, Permission, PlaceEvent, Player, PlayerId, Position, Priority, RuleValue, Sender, ShardCtx, Sidebar, Slot, Title, command, form,
    Display, DisplaySlot, Objective, ScoreHolder, ScoreOperation, SortOrder, Tagged, ScriptEvent,
};
pub use sapling_api::{Mount, Target};
pub use sapling_api::{ArmourSlot, BlockBox, BlockDrop, Content, CreativeCategory, CustomBlock, CustomFood, CustomItem, Geometry, Material, Permutation, Plugin, PluginBuilder, RenderMethod, StateValues};
pub use sapling_net::{AccessLists, Admission, Advert, Compression, ConnKey, Framing, IceServer, IceUrl, Lan, NetherNet, PacketLimits, PortRange, Tls, Transport, TransportEvent, access, host_key};
pub use sapling_world::{ChunkPos, Generated, Level, END, LevelDbStorage, MemoryStorage, NETHER, OVERWORLD, Template, World, WorldFolder, WorldStorage};
pub use server::{ConfigError, Metrics, Server, ServerBuilder, ServerProperties, StartError};
pub use sapling_shard::{MovementCheck, Phase, TickStats, Watchdog};
