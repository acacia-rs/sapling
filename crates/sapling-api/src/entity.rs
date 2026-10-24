use crate::{Effect, ShardCtx};

/// An entity on a shard: a mob, a lying item, a falling block or an experience orb. Unique on
/// its shard while it exists; it goes when removed, picked up or landed, or with its column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// What an entity is and where, as plain data.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EntityInfo {
    pub id: EntityId,
    /// Namespaced: `minecraft:pig`, `minecraft:item`, `minecraft:falling_block`, `minecraft:xp_orb`.
    pub kind: String,
    /// The middle of the bottom of its box.
    pub feet: [f32; 3],
    /// Blocks a tick.
    pub velocity: [f32; 3],
    pub on_ground: bool,
    /// Where its body faces and its pitch, in degrees: yaw -180 to 180, pitch -90 (up) to 90.
    pub yaw: f32,
    pub pitch: f32,
}

impl EntityInfo {
    /// One facing yaw 0 and pitch 0.
    pub fn new(id: EntityId, kind: String, feet: [f32; 3], velocity: [f32; 3], on_ground: bool) -> Self {
        Self { id, kind, feet, velocity, on_ground, yaw: 0.0, pitch: 0.0 }
    }
}

/// Entities are simulated by the shard: each falls and slides through the blocks of its world,
/// and mobs act and take harm as vanilla's do. The calls that name no world act on the world of
/// the event being handled.
impl ShardCtx<'_> {
    /// Spawns a mob of `kind` (e.g. `minecraft:pig`) with its feet at `feet`; `minecraft:xp_orb`
    /// is an orb worth 1, thrown as `/summon` throws one. `None` for a kind the game does not have
    /// or the game alone makes (items, falling blocks), and where the world is not loaded.
    pub fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
        self.backend.spawn_entity(kind, feet)
    }

    /// An entity of this shard; `None` once it is gone.
    pub fn entity(&self, id: EntityId) -> Option<EntityInfo> {
        self.backend.entity(id)
    }

    /// Every entity in the world, oldest first.
    pub fn entities(&self) -> Vec<EntityInfo> {
        self.backend.entities()
    }

    /// Puts an entity's feet at `feet`, in its world; its motion is kept. False for one that is
    /// gone.
    pub fn move_entity(&mut self, id: EntityId, feet: [f32; 3]) -> bool {
        self.backend.move_entity(id, feet)
    }

    /// Sets how fast an entity moves, in blocks a tick; gravity and drag go on from there.
    pub fn set_velocity(&mut self, id: EntityId, velocity: [f32; 3]) -> bool {
        self.backend.set_velocity(id, velocity)
    }

    /// Takes an entity out of the world, from every screen. False for one that is gone.
    pub fn remove_entity(&mut self, id: EntityId) -> bool {
        self.backend.remove_entity(id)
    }

    /// Kills an entity as `/kill` does: a mob dies, leaves its loot (the world's `doMobLoot`
    /// allowing) and is gone a second later; anything else goes at once. False for one that is
    /// gone or already dying.
    pub fn kill_entity(&mut self, id: EntityId) -> bool {
        self.backend.kill_entity(id)
    }

    /// Puts `effect` on a living mob for `ticks` at `amplifier` (0 is the first level), as
    /// `give_effect` puts one on a player; an undead mob takes no poison or regeneration. False
    /// for one that is gone, dying, or no mob.
    pub fn give_entity_effect(&mut self, id: EntityId, effect: Effect, ticks: u32, amplifier: u8) -> bool {
        self.backend.give_entity_effect(id, effect, ticks, amplifier)
    }

    /// The families vanilla puts entities of `kind` in (`minecraft:player` is `player`,
    /// `minecraft:zombie` `zombie undead monster mob`), what a selector's `family=` tests.
    pub fn families(&self, kind: &str) -> Vec<String> {
        self.backend.families(kind)
    }
}

/// The entities side of [`crate::Backend`].
pub trait EntityBackend {
    fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<EntityId>;
    fn entity(&self, id: EntityId) -> Option<EntityInfo>;
    fn entities(&self) -> Vec<EntityInfo>;
    fn move_entity(&mut self, id: EntityId, feet: [f32; 3]) -> bool;
    fn set_velocity(&mut self, id: EntityId, velocity: [f32; 3]) -> bool;
    fn remove_entity(&mut self, id: EntityId) -> bool;
    fn kill_entity(&mut self, id: EntityId) -> bool;
    fn give_entity_effect(&mut self, id: EntityId, effect: Effect, ticks: u32, amplifier: u8) -> bool;
    fn families(&self, _kind: &str) -> Vec<String> {
        Vec::new()
    }
}
