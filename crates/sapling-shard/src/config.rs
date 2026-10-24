use std::time::Duration;

use sapling_api::{Difficulty, GameMode, MovementPolicy};

pub const TICK: Duration = Duration::from_millis(50);

/// Entity ids are unique across a server: a shard counts its own up from its index times this,
/// and a player keeps its id on every shard. MovePlayer carries 32 bits of an id.
pub(crate) const IDS_PER_SHARD: u64 = 1 << 24;

#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// The name clients show for the world (StartGame).
    pub world_name: String,
    /// The name handlers know the shard's first world by.
    pub world_id: String,
    pub game_mode: GameMode,
    pub difficulty: Difficulty,
    /// Where players appear (feet), in the shard's first world.
    pub spawn: [f32; 3],
    /// Cap on the view distance clients ask for, in chunks.
    pub max_view_radius: i32,
    /// Columns announced to one player per tick.
    pub columns_per_tick: usize,
    /// Serve chunk data through the client blob cache to clients that report one.
    pub blob_cache: bool,
    /// Mobs with nothing to do stroll, and flee when hit. Off, they stand where they are until
    /// something moves them.
    pub mobs_stroll: bool,
    pub movement_check: MovementCheck,
    /// Wall time between ticks: [`TICK`], or shorter for drills that compress time (drill_server.rs).
    pub tick: Duration,
}

/// What the shard makes of where clients say their players are (simulation.rs), under the names of
/// BDS's `server.properties` keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementCheck {
    /// Every player's, until a plugin gives one its own (`ShardCtx::set_movement_policy`).
    pub policy: MovementPolicy,
    /// Blocks the server's step and the client's may part by before it counts:
    /// `player-position-acceptance-threshold`.
    pub threshold: f32,
    /// Inputs after a correction before the next, while those in flight still carry the old
    /// prediction: `player-rewind-min-correction-delay-ticks`.
    pub correction_delay: u64,
}

impl Default for MovementCheck {
    fn default() -> Self {
        Self { policy: MovementPolicy::Observe, threshold: 0.5, correction_delay: 5 }
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            world_name: "sapling".into(),
            world_id: "default".into(),
            game_mode: GameMode::Survival,
            difficulty: Difficulty::Peaceful,
            spawn: [0.5, 65.0, 0.5],
            max_view_radius: 8,
            columns_per_tick: 8,
            blob_cache: true,
            mobs_stroll: true,
            movement_check: MovementCheck::default(),
            tick: TICK,
        }
    }
}
