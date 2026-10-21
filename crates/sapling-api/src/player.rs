use uuid::Uuid;

/// One stay of a player on the server: a rejoin gets a new id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u64);

/// Who a player is, as verified at login.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub uuid: Uuid,
    /// Empty unless Xbox Live vouches for the player (always set in online mode).
    pub xuid: String,
}

impl Player {
    pub fn new(id: PlayerId, name: String, uuid: Uuid, xuid: String) -> Self {
        Self { id, name, uuid, xuid }
    }
}

/// Where a player stands and looks. Angles are in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Position {
    pub feet: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl Position {
    pub fn new(feet: [f32; 3], yaw: f32, pitch: f32) -> Self {
        Self { feet, yaw, pitch }
    }
}

/// What a player may do with commands. Set with [`crate::ShardCtx::set_permission`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Permission {
    #[default]
    Member,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}
