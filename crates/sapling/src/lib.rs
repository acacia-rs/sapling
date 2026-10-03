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

mod server;

pub use sapling_api::{GameMode, Handler, Player, PlayerId, ShardCtx};
pub use sapling_world::{ChunkPos, LevelDbStorage, MemoryStorage, World, WorldFolder, WorldStorage};
pub use server::{Server, ServerBuilder, StartError};
