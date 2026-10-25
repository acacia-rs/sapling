//! What mobs shoot, and how. Arrows, as BDS's skeleton shot them (docs/research/mob-attacks.md,
//! "Skeletons"): loosed at its prey with its archer's effect if it has one. A blaze's small
//! fireballs ("The blaze"), which fly straight, burn whom they hit and are gone where they land. A
//! llama's spit ("Hit back by day"), which falls and throws nobody, and a witch's potions ("The
//! witch"), which break where they land (`splashes`). Their flight: flight.rs.
// TODO: a mob's arrow is not picked up and goes after a minute; its aim and its spread are the
// Java game's, fitted to no more than the speeds BDS showed.

use bevy_ecs::prelude::*;

use super::flight::{self, Launch};
pub(super) use super::flight::sound;
use crate::blocks::Dice;
use crate::body::Body;
use crate::late_sounds;

/// What a shot does where it lands, beyond meeting a block or a body.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Lands {
    /// It hurts whom it meets: an arrow, a trident, a fireball, spit.
    Hurts,
    /// A thrown potion of this data value breaks (`splashes`); a lingering one leaves a cloud.
    Splash(i16),
    Lingers(i16),
    Snowball,
    Egg,
    Pearl,
    Experience,
    Wind,
}

/// A kind of thing shot or thrown.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shot {
    pub kind: &'static str,
    /// A mob looses it this far before itself and this high up, at this speed.
    pub ahead: f32,
    pub held: f32,
    pub speed: f32,
    /// A mob aims it this high up its prey, raised by this much of the way there.
    pub aimed: f32,
    pub loft: f32,
    /// A tick keeps this much of its speed (this much in water) and adds this much fall.
    pub drag: f32,
    pub wet_drag: f32,
    pub weight: f32,
    /// What it costs, or its speed twice over, rounded up.
    pub harm: Option<f32>,
    /// It stays in the block it meets, and it sets whom it hits on fire for this many ticks.
    pub sticks: bool,
    pub burns: u16,
    /// It throws the player it hits as a hit does.
    pub throws: bool,
    pub sound: &'static str,
    /// Heard where it hits a body, and where it sticks.
    pub heard: [&'static str; 2],
    /// Each part of a mob's aim is off by up to this.
    pub spread: f32,
    /// Its rise is this much a block of the way to its prey, less this, where it is not aimed.
    pub arc: Option<[f32; 2]>,
    pub lands: Lands,
    /// Ticks it flies before it is gone, if it does not stick.
    pub lasts: Option<u64>,
    pub size: [f32; 2],
}

/// The pack's projectile component: a tick keeps 0.99 of the speed, 0.6 in water.
pub(crate) const ARROW: Shot = Shot {
    kind: "minecraft:arrow",
    ahead: 1.0,
    held: 1.61,
    speed: 1.6,
    aimed: 0.6,
    loft: 0.2,
    drag: 0.99,
    wet_drag: 0.6,
    weight: 0.05,
    harm: None,
    sticks: true,
    burns: 0,
    throws: true,
    sound: "bow",
    heard: ["bow.hit", "bow.hit"],
    spread: SPREAD,
    arc: None,
    lands: Lands::Hurts,
    lasts: None,
    size: [0.25, 0.25],
};
/// research/captures/bds-blaze.txt: from 1.4 up the blaze, 1.15 to 1.42 a tick and no slower
/// after; bds-kinds2.txt: 5 off the player it hit, which burnt five seconds.
const FIREBALL: Shot = Shot { kind: "minecraft:small_fireball", ahead: 0.0, held: 1.4, speed: 1.3, aimed: 0.9, loft: 0.0, drag: 1.0, wet_drag: 1.0, weight: 0.0, harm: Some(5.0), sticks: false, burns: 100, sound: "shoot", lasts: Some(FLIES), ..ARROW };
/// research/captures/bds-provoke2.txt: from 1.58 up the llama at 1.5 a tick, falling 0.06 a tick
/// more; 1 off the player it hit, which it did not throw. Aimed as the Java game's.
const SPIT: Shot = Shot { kind: "minecraft:llama_spit", ahead: 0.0, held: 1.58, speed: 1.5, weight: 0.06, harm: Some(1.0), sticks: false, throws: false, sound: "shoot", lasts: Some(FLIES), ..ARROW };
/// research/captures/bds-witch.txt: from 1.415 up the witch and 1.15 before it at 0.75 a tick and
/// no spread, its rise fitted to three throws 4.3, 6.2 and 8.2 off.
pub(crate) const POTION: Shot = Shot { kind: "minecraft:splash_potion", ahead: 1.15, held: 1.415, speed: 0.75, sticks: false, sound: "throw", spread: 0.0, arc: Some([0.0748, 0.2607]), lands: Lands::Splash(0), ..ARROW };

/// What a kind shoots, by the name of it in `mob-traits.tsv` (`shot=`); arrows where none is named.
pub(super) fn shot_named(name: Option<&str>) -> Shot {
    [FIREBALL, SPIT, POTION].into_iter().find(|shot| shot.kind.strip_prefix("minecraft:") == name).unwrap_or(ARROW)
}

impl Shot {
    pub(super) fn is_potion(&self) -> bool {
        matches!(self.lands, Lands::Splash(_))
    }

    /// The same, a thrown potion of the data value `data`.
    pub(super) fn of_potion(self, data: i16) -> Self {
        Self { lands: Lands::Splash(data), ..self }
    }
}

/// Each part of a shot's speed is off by up to this.
const SPREAD: f32 = 0.075;
/// One a mob shoots that does not stick is gone after this many ticks in the air.
const FLIES: u64 = 100;

/// A mob of `kind` with the body `archer` looses a `shot` at the player whose feet are `prey`.
pub(super) fn loose(world: &mut World, layer: Entity, by: Entity, kind: &str, archer: &Body, prey: [f32; 3], gives: Option<(&'static str, u32)>, shot: Shot) {
    let (dx, dz) = (prey[0] - archer.at[0], prey[2] - archer.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    let from = [archer.at[0] + dx / far * shot.ahead, archer.at[1] + shot.held, archer.at[2] + dz / far * shot.ahead];
    let aim = match shot.arc {
        Some([per_block, less]) => [dx / far, per_block * far - less, dz / far],
        None => [prey[0] - from[0], prey[1] + shot.aimed - from[1] + far * shot.loft, prey[2] - from[2]],
    };
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let motion = aim.map(|c| c / long * shot.speed + (dice.fraction() + dice.fraction() - 1.0) * shot.spread);
    if flight::launch(world, Launch { layer, by, archer: kind.to_owned(), from, motion, shot, gives }, None).is_none() {
        return;
    }
    let middle = [archer.at[0], archer.at[1] + archer.size[1] / 2.0, archer.at[2]];
    late_sounds::play_later(world, 0, layer, archer.at.map(|c| c.floor() as i32), flight::sound(shot.sound, middle, kind));
}
