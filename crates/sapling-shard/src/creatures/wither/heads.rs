//! The wither's three heads, as BDS's aimed (`_performRangedAttack`, `setAlternativeTarget`): the
//! middle one shoots its prey in volleys of four at its fire rate, the fourth a blue one; off easy each side
//! one takes a player at random within 30 across and 8 up or down that it sees, shoots it and
//! looks again 40 to 59 ticks on, or after 15 looks with none shoots a blue skull at random near.
// TODO: the middle head holds still while it moves; the side heads' wait
// between looks with no one in reach is the Java game's.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::super::health::Health;
use super::super::{hunting, legs};
use super::{KIND, Wither, skulls};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Settings, Tick, in_world};

const FIRE_RATE: u64 = 20;
const BLUE_EVERY: u32 = 4;
/// After each volley's blue skull the middle head waits 41 to 140 ticks, rounded up to its fire
/// rate (bds-t2wither.txt, bds-witherdash.txt: 60 to 140 in steps of 20; bds-m3wither.txt at 5: 85 to 110).
const RESTS: [u64; 2] = [41, 100];
/// Its health is cut in this many steps, each halving the fire rate off easy.
const STEPS: f32 = 6.0;
const SIGHT: f32 = 70.0;
const REACH: [f32; 2] = [30.0, 8.0];
const TRIES: usize = 10;
const AGAIN: [u64; 2] = [40, 20];
const LOOKS_AGAIN: [u64; 2] = [10, 10];
const IDLE: u32 = 15;
/// A blue skull shot at random goes this far off across, and up or down.
const STRAY: [f32; 2] = [10.0, 5.0];
/// The middle head is this far up the wither, the side ones this far up and this far aside.
const MIDDLE_UP: f32 = 3.0;
const SIDE_UP: f32 = 2.2;
const ASIDE: f32 = 1.3;
/// A skull is aimed this far under the eyes of whom it is shot at.
const UNDER_EYES: f32 = 0.5;

#[derive(Default)]
pub(super) struct Heads {
    shots: u32,
    next: [u64; 3],
    targets: [Option<Entity>; 3],
    idle: u32,
    pub(super) rate: Rate,
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

fn aim(feet: [f32; 3]) -> [f32; 3] {
    [feet[0], feet[1] + sapling_data::EYE_HEIGHT - UNDER_EYES, feet[2]]
}

/// A player of `layer` at random within a side head's reach that it sees.
fn pick(world: &mut World, layer: Entity, body: &Body) -> Option<(Entity, [f32; 3])> {
    let within = |feet: [f32; 3]| (feet[0] - body.at[0]).abs() <= REACH[0] && (feet[2] - body.at[2]).abs() <= REACH[0] && (feet[1] - body.at[1]).abs() <= REACH[1];
    let near: Vec<(Entity, [f32; 3])> = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer)).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).filter(|(_, feet)| within(*feet)).collect();
    if near.is_empty() {
        return None;
    }
    (0..TRIES).find_map(|_| {
        let picked = near[world.get_resource_or_insert_with(Dice::default).roll(near.len() as u32) as usize];
        hunting::in_sight(world, layer, body, picked.1).then_some(picked)
    })
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

/// A tick of the heads: each shoots when its time comes, and viewers are told whom each looks at.
pub(super) fn tick(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let fights = matches!(world.resource::<Settings>().0.difficulty, Difficulty::Normal | Difficulty::Hard);
    let yaw = world.get::<legs::Legs>(wither).map_or(0.0, |legs| legs.yaw);
    let Some(mut heads) = world.get_mut::<Wither>(wither).map(|mut kept| std::mem::take(&mut kept.heads)) else { return };
    let before = heads.targets;
    let prey = prey(world, wither, layer, body);
    heads.targets[0] = prey.map(|(player, _)| player);
    if let Some((_, feet)) = prey.filter(|_| now >= heads.next[0]) {
        heads.shots += 1;
        let blue = heads.shots % BLUE_EVERY == 0;
        skulls::fire(world, layer, wither, mouth(body, yaw, 0), aim(feet), blue);
        let every = heads.rate.every;
        let rest = if blue { (RESTS[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll(RESTS[1] as u32))).div_ceil(every) * every } else { every };
        heads.next[0] = now + rest;
    }
    for head in 1..3 {
        if !fights || now < heads.next[head] {
            continue;
        }
        match pick(world, layer, body) {
            Some((player, feet)) => {
                skulls::fire(world, layer, wither, mouth(body, yaw, head), aim(feet), false);
                heads.targets[head] = Some(player);
                heads.idle = 0;
                heads.next[head] = now + AGAIN[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll(AGAIN[1] as u32));
            }
            None => {
                heads.targets[head] = None;
                heads.idle += 1;
                if head == 1 && heads.idle > IDLE {
                    heads.idle = 0;
                    let to = {
                        let mut dice = world.get_resource_or_insert_with(Dice::default);
                        let mut off = |reach: f32| (dice.fraction() * 2.0 - 1.0) * reach;
                        [body.at[0] + off(STRAY[0]), body.at[1] + off(STRAY[1]), body.at[2] + off(STRAY[0])]
                    };
                    skulls::fire(world, layer, wither, mouth(body, yaw, head), to, true);
                }
                heads.next[head] = now + LOOKS_AGAIN[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll(LOOKS_AGAIN[1] as u32));
            }
        }
    }
    let changed: Vec<(Key, Plain)> = [Key::WitherTarget1, Key::WitherTarget2, Key::WitherTarget3]
        .into_iter()
        .zip(heads.targets.iter().zip(before))
        .filter(|(_, (now, before))| **now != *before)
        .map(|(key, (target, _))| (key, Plain::Long(target.and_then(|target| world.get::<NetId>(target)).map_or(0, |id| id.0 as i64))))
        .collect();
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.heads = heads;
    }
    if !changed.is_empty() {
        super::tell(world, wither, layer, changed);
    }
}
