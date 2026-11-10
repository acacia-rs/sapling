//! A spear holder's charge, BDS's `UseKineticWeaponGoal` (Linux vtable 0xeeb4b48, tick 0x53497c0) as
//! research/captures/bds-aa2spear.txt (on foot) and bds-j3ride.txt (on a zombie horse) showed it. Within
//! its `approach_distance` of its prey it raises the spear (`ACTION`, bit 4) and walks at the prey and
//! through it, then to a spot its `reposition` distance south of the prey ([`spot`]), and back. While the spear is up and
//! past the spear's `delay`, a prey in its reach band is hit for the mob's attack plus the whole part
//! of its speed at the prey in blocks a second times the spear's `damage_multiplier`, thrown 0.4 away
//! and up within the knockback time, and viewers see `KineticDamageDealt`: no swing, no sound. After
//! the damage time the spear comes down for a second or so while it walks to a spot its cooldown
//! distance off. A holder never hits as a melee mob.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Tick;
use crate::creatures::hunting::{feet_of, on_difficulty, prey, prey_of};
use crate::{entities, vitals};

/// A spear's `kinetic_weapon` (1.26.52 behaviour pack items): ticks from raising it to its first hit,
/// its `damage_multiplier`, and how many ticks past the delay it still hurts and still throws.
struct Spear {
    name: &'static str,
    delay: u64,
    multiplier: f32,
    hurts_for: u64,
    throws_for: u64,
}

const SPEARS: &[Spear] = &[
    Spear { name: "minecraft:wooden_spear", delay: 15, multiplier: 0.7, hurts_for: 300, throws_for: 200 },
    Spear { name: "minecraft:stone_spear", delay: 14, multiplier: 0.82, hurts_for: 275, throws_for: 180 },
    Spear { name: "minecraft:copper_spear", delay: 13, multiplier: 0.82, hurts_for: 250, throws_for: 165 },
    Spear { name: "minecraft:iron_spear", delay: 12, multiplier: 0.95, hurts_for: 225, throws_for: 135 },
    Spear { name: "minecraft:golden_spear", delay: 14, multiplier: 0.7, hurts_for: 275, throws_for: 170 },
    Spear { name: "minecraft:diamond_spear", delay: 10, multiplier: 1.075, hurts_for: 200, throws_for: 130 },
    Spear { name: "minecraft:netherite_spear", delay: 8, multiplier: 1.2, hurts_for: 175, throws_for: 110 },
];

/// The kinds whose packs give them `use_kinetic_weapon`.
const HOLDERS: &[&str] = &["minecraft:zombie", "minecraft:husk", "minecraft:zombie_villager_v2", "minecraft:zombie_pigman", "minecraft:piglin"];
/// Its `approach_distance`, `reposition_distance` and `cooldown_distance` ranges, on foot and mounted (the packs).
const ON_FOOT: (f32, [f32; 2], [f32; 2]) = (10.0, [6.0, 7.0], [9.0, 11.0]);
const MOUNTED: (f32, [f32; 2], [f32; 2]) = (12.0, [8.0, 9.0], [11.0, 13.0]);
/// The spears' reach 2 to 4.5 with a 0.25 margin, halved by a mob's `weapon_reach_multiplier`, centre
/// to centre across (fitted: BDS's first hits came 2.5 off).
const REACH: [f32; 2] = [1.0, 2.5];
/// Nearer than the middle of its reach it has passed and repositions (the goal's `(min + max) / 2`).
const PASSES: f32 = 1.625;
/// The least speed at its prey, blocks a second, for a hit and for a throw: the spears' 4.6 and 5.1
/// times a mob's `weapon_min_speed_multiplier` 0.2.
const HURTS_FROM: f32 = 0.92;
const THROWS_FROM: f32 = 1.02;
const THROW: f32 = 0.4;
/// Ticks between two of its hits on one prey (bds-af2spear.txt: 11, mounted and on foot).
const REHITS: u64 = 11;
/// The spread of the angle its reposition spot is picked at, degrees (the goal's +0x158, read by gdb
/// in bds-af2spear-gdb.txt).
const SPREAD: f32 = 90.0;
/// How near a reposition spot counts as reached, and the ticks it tries for one.
const REACHED: f32 = 1.0;
const GIVES_UP: u64 = 100;
/// Ticks the spear stays down after its damage time (bds-aa2spear.txt: 22 and 30).
const RESTS: [u64; 2] = [20, 30];

#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Approach,
    Charge,
    /// Its spot and the tick it gives up on it.
    Reposition([f32; 3], u64),
    /// The tick it may raise the spear again, and the spot it walks to meanwhile.
    Rest(u64, [f32; 3]),
}

#[derive(Component, Clone, Copy, Default)]
pub(in crate::creatures) struct Lance {
    stage: Stage,
    /// The tick it raised the spear in, while it is up.
    raised: Option<u64>,
    hit: Option<u64>,
    /// Where it stood after its last move, for its speed.
    was: Option<[f32; 3]>,
}

fn spear_of(world: &World, entity: Entity, kind: &str) -> Option<&'static Spear> {
    let name = crate::creatures::command_gear::main_hand(world, entity).filter(|_| HOLDERS.contains(&kind))?.name;
    SPEARS.iter().find(|spear| spear.name == name)
}

fn across(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// Moves a spear holder's charge on a tick and gives where it walks: its prey, or its reposition spot.
/// None for one with no spear or no prey, which lowers a raised spear.
pub(in crate::creatures) fn plan(world: &mut World, entity: Entity, kind: &str, at: [f32; 3], prey: Option<[f32; 3]>) -> Option<[f32; 3]> {
    let (Some(spear), Some(feet)) = (spear_of(world, entity, kind), prey) else {
        lower(world, entity, Stage::Approach);
        return None;
    };
    let now = world.resource::<Tick>().0;
    let (approach, reposition, cooldown) = if world.get::<crate::riding::Riding>(entity).is_some() { MOUNTED } else { ON_FOOT };
    if world.get::<Lance>(entity).is_none() {
        world.entity_mut(entity).insert(Lance::default());
    }
    // The goal's state 3 (0x534a440): as the spear comes down it picks a spot as `spot` does, its
    // `cooldown_distance` off; bds-af2spear-gdb.txt's three far picks were these.
    if world.get::<Lance>(entity)?.raised.is_some_and(|raised| now >= raised + spear.delay + spear.hurts_for) {
        let rest = RESTS[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll((RESTS[1] - RESTS[0]) as u32 + 1));
        let away = spot(world, at, feet, cooldown);
        lower(world, entity, Stage::Rest(now + rest, away));
    }
    let mut lance = *world.get::<Lance>(entity)?;
    let off = across(at, feet);
    lance.stage = match lance.stage {
        Stage::Rest(until, away) if now < until => Stage::Rest(until, away),
        Stage::Approach | Stage::Rest(..) if off < approach => {
            if lance.raised.is_none() {
                lance.raised = Some(now);
                crate::creatures::husbandry::show_flag(world, entity, Flags::ACTION, true);
            }
            Stage::Charge
        }
        Stage::Approach | Stage::Rest(..) => Stage::Approach,
        Stage::Charge if off < PASSES => Stage::Reposition(spot(world, at, feet, reposition), now + GIVES_UP),
        Stage::Reposition(spot, until) if now >= until || across(at, spot) < REACHED => Stage::Approach,
        stage => stage,
    };
    world.entity_mut(entity).insert(lance);
    Some(match lance.stage {
        Stage::Reposition(spot, _) | Stage::Rest(_, spot) => spot,
        _ => feet,
    })
}

/// Where a holder that has passed its prey at `feet` goes next: `UseKineticWeaponGoal` 0x5349f00 turns
/// the angle from it to its prey (atan2, radians, plus a spread) into radians again as if degrees, so
/// the spot is all but due south (+z) of the prey, its reposition distance off, whichever way it came
/// (bds-af2spear-gdb.txt); the cell's middle.
fn spot(world: &mut World, at: [f32; 3], feet: [f32; 3], [near, far]: [f32; 2]) -> [f32; 3] {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let spread = SPREAD.to_radians() * 0.5;
    let angle = (feet[2] - at[2]).atan2(feet[0] - at[0]) + spread * (dice.fraction() * 2.0 - 1.0);
    let beyond = near + dice.fraction() * (far - near);
    let turned = angle.to_radians();
    [(feet[0] - turned.sin() * beyond).floor() + 0.5, feet[1].floor(), (feet[2] + turned.cos() * beyond).floor() + 0.5]
}

/// The spot a mounted spear holder's mount walks at while it repositions or cools down.
pub(in crate::creatures) fn aim(world: &World, rider: Entity) -> Option<[f32; 3]> {
    match world.get::<Lance>(rider)?.stage {
        Stage::Reposition(spot, _) | Stage::Rest(_, spot) => Some(spot),
        _ => None,
    }
}

/// Lowers a raised spear, which its viewers are told, and puts the charge at `stage`.
fn lower(world: &mut World, entity: Entity, stage: Stage) {
    let Some(mut lance) = world.get_mut::<Lance>(entity) else { return };
    let raised = lance.raised.take().is_some();
    lance.stage = stage;
    if raised {
        crate::creatures::husbandry::show_flag(world, entity, Flags::ACTION, false);
    }
}

/// After a holder's move: None for a mob with no spear, which hits as any other; else Some, its
/// hit on its prey if it landed one.
pub(in crate::creatures) fn strike(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<Option<()>> {
    let spear = spear_of(world, entity, kind)?;
    let was = world.get_mut::<Lance>(entity).and_then(|mut lance| lance.was.replace(body.at));
    Some(was.and_then(|was| hit(world, entity, layer, id, kind, body, spear, was)))
}

/// `was`: where it stood a tick before; a rider moves with its mount.
fn hit(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, spear: &Spear, was: [f32; 3]) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let lance = *world.get::<Lance>(entity)?;
    let raised = lance.raised.filter(|raised| now >= raised + spear.delay)?;
    // Not once it has passed: BDS's struck no unthrown prey twice on a pass (bds-aa2spear.txt).
    if !matches!(lance.stage, Stage::Charge) || lance.hit.is_some_and(|hit| now < hit + REHITS) {
        return None;
    }
    let target = prey_of(world, entity)?;
    let (feet, size) = (feet_of(world, target)?, prey::size_of(world, target));
    let off = across(body.at, feet);
    if !(REACH[0]..=REACH[1]).contains(&off) || feet[1] >= body.at[1] + body.size[1] || body.at[1] >= feet[1] + size[1] {
        return None;
    }
    let away = [(feet[0] - body.at[0]) / off, (feet[2] - body.at[2]) / off];
    let motion = [body.at[0] - was[0], 0.0, body.at[2] - was[2]];
    let fled = world.get::<Body>(target).filter(|_| prey::is_mob(world, target)).map_or([0.0; 3], |prey| prey.motion);
    let speed = ((motion[0] - fled[0]) * away[0] + (motion[2] - fled[2]) * away[1]) * 20.0;
    if speed < HURTS_FROM {
        return None;
    }
    let hunts = crate::creatures::genes::hunts(world, entity, sapling_data::mob_traits(kind).hunts?);
    let harm = crate::creatures::afflictions::hits_for(world, entity, hunts.attack + (speed * spear.multiplier).floor());
    let throws = now < raised + spear.delay + spear.throws_for && speed >= THROWS_FROM;
    world.get_mut::<Lance>(entity)?.hit = Some(now);
    if prey::is_mob(world, target) {
        prey::hit(world, layer, entity, target, harm, if throws { away } else { [0.0; 2] });
    } else {
        let harm = on_difficulty(world, harm)?;
        vitals::speared(world, target, harm, kind, throws.then_some([away[0] * THROW, THROW, away[1] * THROW]));
    }
    let dealt = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::KineticDamageDealt, data: 1, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&dealt)]);
    Some(())
}
