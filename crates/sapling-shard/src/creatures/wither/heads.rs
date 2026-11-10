//! The wither's three heads, as BDS's aimed (`_performRangedAttack`, `setAlternativeTarget`): the
//! middle one shoots its prey in volleys of four at its fire rate, the fourth a blue one; the side ones
//! are sides.rs. The middle head (the yaw told as its head's) turns 10 degrees a
//! tick at most towards its prey across, and back to the body with none (research/captures/bds-ab1wither2.txt:
//! held cows, 483 of 566 told moves within 5 degrees of the prey's yaw, turning 20 a told move). After a
//! volley's blue skull it looks back to the body for [`LOOKS_BACK`] ticks, fitted (bds-ab1wither2.txt: about
//! 1.4 s each volley, with `WitherRandomAttackPosGoal` starting after it). Unarmoured, the middle head's
//! next shot after a blue skull waits on its swoop (swoop.rs); armoured, on [`RESTS`].

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::super::health::Health;
use super::super::{hunting, legs};
use super::{KIND, Wither, skulls};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Settings, Tick};

mod sides;

const FIRE_RATE: u64 = 20;
const BLUE_EVERY: u32 = 4;
/// After each volley's blue skull the middle head waits 41 to 140 ticks, rounded up to its fire
/// rate (bds-t2wither.txt, bds-witherdash.txt: 60 to 140 in steps of 20; bds-m3wither.txt at 5: 85 to 110).
const RESTS: [u64; 2] = [41, 100];
/// Its health is cut in this many steps, each halving the fire rate off easy.
const STEPS: f32 = 6.0;
pub(super) const SIGHT: f32 = 70.0;
/// The middle head is this far up the wither, the side ones this far up and this far aside.
const MIDDLE_UP: f32 = 3.0;
const SIDE_UP: f32 = 2.2;
const ASIDE: f32 = 1.3;
const MOB_EYES: f32 = 0.85;
const HEAD_TURN: f32 = 10.0;
const LOOKS_BACK: u64 = 28;

impl Heads {
    /// The middle head shoots again at `tick`, after a swoop.
    pub(super) fn resume(&mut self, tick: u64) {
        self.next[0] = tick;
    }
}

#[derive(Default)]
pub(super) struct Heads {
    shots: u32,
    next: [u64; 3],
    targets: [Option<Entity>; 3],
    /// Each side head's looks since it last shot.
    idle: [u32; 2],
    pub(super) rate: Rate,
    /// The middle head's yaw, in degrees, once it has looked.
    look: Option<f32>,
    /// The tick the middle head looks back to the body until.
    back: u64,
}

/// The middle head's ticks between shots (`firerate`) and the health it next halves under
/// (`lastHealthInterval`, its max health until hurt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rate {
    pub(super) every: u64,
    pub(super) under: Option<i32>,
}

impl Default for Rate {
    fn default() -> Self {
        Rate { every: FIRE_RATE, under: None }
    }
}

impl Rate {
    /// At half health its fire rate is 20 again (142f96660).
    pub(super) fn rearm(&mut self) {
        self.every = FIRE_RATE;
    }

    /// Off easy, health under the last step less a sixth of its max halves the fire rate and moves
    /// the step down, once a hurt (`WitherBoss` hurt 142f96660; bds-m3wither.txt: 20, 10 under 375, 5 under 300 of 450).
    fn step(&mut self, health: &Health, difficulty: Difficulty) {
        let under = self.under.unwrap_or(health.max() as i32);
        let next = under - (health.max() / STEPS) as i32;
        if difficulty != Difficulty::Easy && (health.left() as i32) < next {
            (self.every, self.under) = ((self.every / 2).max(1), Some(next));
        }
    }
}

/// A sound of the wither's at `at`.
pub(super) fn sound(name: &str, [x, y, z]: [f32; 3]) -> bytes::Bytes {
    let sound = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// Where head `head` (0 the middle) shoots from.
fn mouth(body: &Body, yaw: f32, head: usize) -> [f32; 3] {
    if head == 0 {
        return [body.at[0], body.at[1] + MIDDLE_UP, body.at[2]];
    }
    let angle = (yaw + head as f32 * 180.0 - 180.0).to_radians();
    [body.at[0] + angle.cos() * ASIDE, body.at[1] + SIDE_UP, body.at[2] + angle.sin() * ASIDE]
}

/// The middle head's yaw a tick on from `head`: [`HEAD_TURN`] at most towards `prey` across, or the body's `yaw`.
fn look(head: f32, yaw: f32, body: &Body, prey: Option<[f32; 3]>) -> f32 {
    let wanted = prey.map_or(yaw, |feet| (body.at[0] - feet[0]).atan2(feet[2] - body.at[2]).to_degrees());
    let turn = (wanted - head + 180.0).rem_euclid(360.0) - 180.0;
    head + turn.clamp(-HEAD_TURN, HEAD_TURN)
}

/// Where a skull at `target` is aimed: its position (a player's eyes, a mob's feet) plus half its eye
/// height, as Java's `getEyeHeight() * 0.5` over BDS's positions (bds-y3wither.txt: 38 skulls passed the
/// standing actor 2.41 over its feet on average, 2.43 by this rule; a mob's eyes taken at its height's 0.85).
fn aim(world: &World, target: Entity, feet: [f32; 3]) -> [f32; 3] {
    let up = match world.get::<Body>(target).filter(|_| world.get::<Position>(target).is_none()) {
        Some(body) => body.size[1] * MOB_EYES * 0.5,
        None => sapling_data::EYE_HEIGHT * 1.5,
    };
    [feet[0], feet[1] + up, feet[2]]
}

/// Its prey, as its pack's `nearest_attackable_target` takes it: the nearest it sees within 70 of
/// the players and the living mobs neither undead nor inanimate (bds-t2wither.txt: a held cow).
pub(super) fn prey(world: &mut World, wither: Entity, layer: Entity, body: &Body) -> Option<(Entity, [f32; 3])> {
    let player = hunting::nearest(world, layer, body, SIGHT);
    let far = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] - body.at[axis]).powi(2)).sum::<f32>();
    let mobs: Vec<(Entity, [f32; 3])> = world
        .query_filtered::<(Entity, &InLayer, &Body, &crate::entities::Kind), (With<super::super::Creature>, Without<super::super::Dying>)>()
        .iter(world)
        .filter(|(mob, on, at, kind)| *mob != wither && on.0 == layer && far(at.at) <= SIGHT * SIGHT && !sapling_data::is_undead(&kind.0) && !sapling_data::has_family(&kind.0, "inanimate"))
        .map(|(mob, _, at, _)| (mob, at.at))
        .collect();
    let mob = mobs.into_iter().filter(|(_, feet)| hunting::in_sight(world, layer, body, *feet)).min_by(|a, b| far(a.1).total_cmp(&far(b.1)));
    [player, mob].into_iter().flatten().min_by(|a, b| far(a.1).total_cmp(&far(b.1)))
}

/// Its fire rate halves for the health it lost since the last tick.
pub(super) fn pace(world: &mut World, wither: Entity) {
    let (Some(health), difficulty) = (world.get::<Health>(wither).copied(), world.resource::<Settings>().0.difficulty) else { return };
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.heads.rate.step(&health, difficulty);
    }
}

/// A tick of the heads, the middle one's on `prey`: each shoots when its time comes, and viewers are
/// told whom each looks at, -1 for none as BDS's.
pub(super) fn tick(world: &mut World, wither: Entity, layer: Entity, body: &Body, prey: Option<(Entity, [f32; 3])>) {
    let now = world.resource::<Tick>().0;
    let yaw = world.get::<legs::Legs>(wither).map_or(0.0, |legs| legs.yaw);
    let Some(mut heads) = world.get_mut::<Wither>(wither).map(|mut kept| std::mem::take(&mut kept.heads)) else { return };
    let before = heads.targets;
    let armoured = world.get::<Wither>(wither).is_some_and(|kept| kept.armoured);
    if armoured && heads.next[0] == u64::MAX {
        heads.next[0] = now;
    }
    heads.targets[0] = prey.map(|(player, _)| player);
    let look = look(heads.look.unwrap_or(yaw), yaw, body, prey.filter(|_| now >= heads.back).map(|(_, feet)| feet));
    heads.look = Some(look);
    if let Some(mut told) = world.get_mut::<crate::sync::Told>(wither) {
        told.face(yaw, look);
    }
    if let Some((target, feet)) = prey.filter(|_| now >= heads.next[0]) {
        heads.shots += 1;
        let blue = heads.shots % BLUE_EVERY == 0;
        let to = aim(world, target, feet);
        skulls::fire(world, layer, wither, mouth(body, yaw, 0), to, blue);
        let every = heads.rate.every;
        let rest = if blue { (RESTS[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll(RESTS[1] as u32))).div_ceil(every) * every } else { every };
        heads.next[0] = now + rest;
        if blue {
            heads.back = now + LOOKS_BACK;
        }
        if blue && !armoured {
            heads.next[0] = u64::MAX;
            if let Some(mut kept) = world.get_mut::<Wither>(wither) {
                kept.swoop.after_volley(now, every);
            }
        }
    }
    sides::tick(world, wither, layer, body, yaw, &mut heads, now);
    let changed: Vec<(Key, Plain)> = [Key::WitherTarget1, Key::WitherTarget2, Key::WitherTarget3]
        .into_iter()
        .zip(heads.targets.iter().zip(before))
        .filter(|(_, (now, before))| **now != *before)
        .map(|(key, (target, _))| (key, Plain::Long(target.and_then(|target| world.get::<NetId>(target)).map_or(-1, |id| id.0 as i64))))
        .collect();
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.heads = heads;
    }
    if !changed.is_empty() {
        super::tell(world, wither, layer, changed);
    }
}
