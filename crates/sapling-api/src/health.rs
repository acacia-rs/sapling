use crate::{PlayerId, ShardCtx};

/// What hurt a player: it picks the flash its client shows, the words of its death and whether
/// armour stops part of it. Numbered as the wire numbers them (BDS's `ActorDamageCause`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCause {
    /// `/damage` without a cause, `/kill`.
    Generic = -1,
    /// A cactus or a bush.
    Contact = 1,
    Attack = 2,
    Projectile = 3,
    /// A block its head is in.
    Suffocation = 4,
    Fall = 5,
    /// Touching fire.
    Fire = 6,
    /// Burning on.
    FireTick = 7,
    Lava = 8,
    Drowning = 9,
    BlockExplosion = 10,
    EntityExplosion = 11,
    Void = 12,
    /// Instant damage and poison.
    Magic = 14,
    Wither = 15,
    Starving = 16,
    Anvil = 17,
    Thorns = 18,
    FallingBlock = 19,
    FlyIntoWall = 21,
    Magma = 22,
    Lightning = 24,
    Freezing = 27,
    Stalactite = 28,
    Stalagmite = 29,
    /// A goat's ram.
    RamAttack = 30,
    SonicBoom = 31,
    Campfire = 32,
    SoulCampfire = 33,
}

impl DamageCause {
    /// Each cause under the name `/damage` knows it by.
    const NAMED: [(&'static str, DamageCause); 29] = [
        ("none", Self::Generic),
        ("contact", Self::Contact),
        ("entity_attack", Self::Attack),
        ("projectile", Self::Projectile),
        ("suffocation", Self::Suffocation),
        ("fall", Self::Fall),
        ("fire", Self::Fire),
        ("fire_tick", Self::FireTick),
        ("lava", Self::Lava),
        ("drowning", Self::Drowning),
        ("block_explosion", Self::BlockExplosion),
        ("entity_explosion", Self::EntityExplosion),
        ("void", Self::Void),
        ("magic", Self::Magic),
        ("wither", Self::Wither),
        ("starve", Self::Starving),
        ("anvil", Self::Anvil),
        ("thorns", Self::Thorns),
        ("falling_block", Self::FallingBlock),
        ("fly_into_wall", Self::FlyIntoWall),
        ("magma", Self::Magma),
        ("lightning", Self::Lightning),
        ("freezing", Self::Freezing),
        ("stalactite", Self::Stalactite),
        ("stalagmite", Self::Stalagmite),
        ("ram_attack", Self::RamAttack),
        ("sonic_boom", Self::SonicBoom),
        ("campfire", Self::Campfire),
        ("soul_campfire", Self::SoulCampfire),
    ];

    pub fn id(self) -> i32 {
        self as i32
    }

    /// The cause `/damage` calls `name`; `override` is `none`.
    pub fn named(name: &str) -> Option<Self> {
        let name = if name == "override" { "none" } else { name };
        Self::NAMED.iter().find(|(known, _)| *known == name).map(|(_, cause)| *cause)
    }

    /// Whether armour stops part of it, as BDS's did (docs/research/player-damage.md, "Armour").
    pub fn is_stopped_by_armour(self) -> bool {
        matches!(self, Self::Generic | Self::Contact | Self::Attack | Self::Projectile | Self::Fire | Self::Lava | Self::BlockExplosion | Self::EntityExplosion | Self::Anvil | Self::Thorns | Self::FallingBlock | Self::Magma | Self::Lightning | Self::Stalactite | Self::Campfire | Self::SoulCampfire)
    }

    /// Falls on the head: a helmet takes a quarter of it first.
    pub fn is_on_the_head(self) -> bool {
        matches!(self, Self::Anvil | Self::FallingBlock | Self::Stalactite)
    }
}

/// Players in survival and adventure have 20 health; the shard takes from it for falls, fire and
/// lava, and a player with none left is dead until its client asks to rise at the world's spawn.
impl ShardCtx<'_> {
    /// A player's health, 0 to 20; `None` for one who is not in the world.
    pub fn health(&self, player: PlayerId) -> Option<f32> {
        self.backend.health(player)
    }

    /// Takes `amount` from a player's health as a hit of `cause`. False where it took nothing:
    /// the player is dead, in creative or spectator, or was hit as hard less than half a second ago.
    pub fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool {
        self.backend.hurt_player(player, amount, cause)
    }

    /// Kills a player as `/kill` does. False for one already dead, or in creative or spectator.
    pub fn kill_player(&mut self, player: PlayerId) -> bool {
        self.backend.kill_player(player)
    }
}

/// The health side of [`crate::Backend`].
pub trait HealthBackend {
    fn health(&self, player: PlayerId) -> Option<f32>;
    fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool;
    fn kill_player(&mut self, player: PlayerId) -> bool;
}
