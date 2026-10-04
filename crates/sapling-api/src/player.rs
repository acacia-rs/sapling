use uuid::Uuid;

/// One stay of a player on the server: a rejoin gets a new id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u64);

/// Who a player is, as verified at login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub uuid: Uuid,
    /// Empty unless Xbox Live vouches for the player (always set in online mode).
    pub xuid: String,
}

/// Where a player stands and looks. Angles are in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub feet: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

/// What a player may do with commands. Set with [`crate::ShardCtx::set_permission`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Permission {
    #[default]
    Member,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}
