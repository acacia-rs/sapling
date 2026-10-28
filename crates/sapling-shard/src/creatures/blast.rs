//! Mobs that blow up, as BDS's creeper did (docs/research/mob-attacks.md, "Creepers"): lit near
//! its prey, it burns a fuse down and bursts, hurting and throwing who is near and breaking blocks
//! (docs/research/block-uses.md, "Blasts").
// TODO: a fuse does not go out for a prey out of sight; a blast throws no arrows, orbs or falling
// blocks; Blast Protection does not lessen a mob's throw.

mod rays;

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, ExplosionEvent};

use super::harm;
use super::health::{Cause, Health};
use crate::blocks;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings};
use crate::{drops, entities, late_sounds, mobs, vitals};

/// A fuse is this many ticks long, and hisses this many ticks after it is lit.
const FUSE: i32 = 30;
const HISSES: i32 = 2;
const HEAD: f32 = 0.85;
const GRIEFING: &str = "mobGriefing";
/// A blast's worst, at its middle, for each unit of its power.
const HARM: f32 = 7.0;
/// What a blast's harm throws a mob by, away from it and up (bds-blasts.txt).
const KNOCK: f32 = 0.4;
/// A mob is thrown from the blast towards this far up its height (a pig's 0.81 of 0.9).
const EYES: f32 = 0.9;
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

/// A lit fuse: the ticks left of it.
#[derive(Component)]
pub(super) struct Fuse(i32);

pub(super) fn is_lit(world: &World, entity: Entity) -> bool {
    world.get::<Fuse>(entity).is_some()
}

fn tell(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, left: i32) {
    let told = sapling_data::mob_fuse(id, entities::mob_flags(world, entity, kind, MetadataFlags1::IGNITED, true), left);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// Lights the mob's fuse, if it is not lit.
pub(super) fn light(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if !is_lit(world, entity) {
        world.entity_mut(entity).insert(Fuse(FUSE - 1));
        tell(world, entity, layer, id, kind, body, FUSE - 1);
    }
}

/// Puts the mob's fuse out, if it is lit: its prey is gone or far.
pub(super) fn put_out(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if world.entity_mut(entity).take::<Fuse>().is_some() {
        let unlit = sapling_data::mob_flags_set(id, entities::mob_flags(world, entity, kind, MetadataFlags1::IGNITED, false));
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&unlit)]);
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// How much of a body standing at `feet`, `size` wide and high, sees a blast at `at`.
fn seen(world: &World, layer: Entity, at: [f32; 3], feet: [f32; 3], [wide, high]: [f32; 2]) -> f32 {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return 1.0 };
    let half = wide / 2.0;
    rays::seen(level, at, [feet[0] - half, feet[1], feet[2] - half], [feet[0] + half, feet[1] + high, feet[2] + half])
}

/// What a blast of `power` at `at` does to a player whose eyes are at `eyes`, `seen` of it seeing
/// the blast: what it costs on this difficulty, nothing on peaceful, and the speed it throws at.
/// A ghast's fireball's (`fireball`) goes by how far the body's `feet` are, costs no 1 more and is
/// the same on every difficulty (research/captures/bds-ghast-findings.md).
fn reaches(world: &World, at: [f32; 3], eyes: [f32; 3], feet: [f32; 3], power: f32, seen: f32, fireball: bool) -> Option<(f32, [f32; 3])> {
    let away: [f32; 3] = std::array::from_fn(|axis| eyes[axis] - at[axis]);
    let far = distance(eyes, at);
    let measured = if fireball { distance(feet, at) } else { far };
    let impact = (1.0 - measured / (2.0 * power)) * seen;
    let harm = (impact * impact + impact) / 2.0 * HARM * 2.0 * power + if fireball { 0.0 } else { 1.0 };
    let harm = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0.0,
        _ if fireball => harm,
        Difficulty::Easy => harm.min(harm / 2.0 + 1.0),
        Difficulty::Normal => harm,
        Difficulty::Hard => harm * 1.5,
    };
    (measured < 2.0 * power && far > 1e-4).then(|| (harm, away.map(|c| c / far * impact)))
}

/// What a blast does to the blocks it reaches: nothing, or it breaks them, and they may leave
/// their items.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wreck {
    Nothing,
    Breaks,
    Leaves,
    /// Each block leaves what it would by a chance of one in the blast's power: every blast but
    /// TNT's with `tntExplosionDropDecay` off (BDS's `Explosion::explode`).
    Decays,
}

/// A blast of `power` at `at`: of `source`, if an entity made it, which is gone with it.
pub(crate) fn explode(world: &mut World, source: Option<Entity>, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    blow(world, source, layer, at, power, wreck, Rule::Plain);
}

/// As [`explode`], a ghast's fireball's blast, by its own rule ([`reaches`]).
pub(super) fn explode_fireball(world: &mut World, source: Entity, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    blow(world, Some(source), layer, at, power, wreck, Rule::Fireball);
}

/// As [`explode`] with no source, a blast that sets fire: a bed's or a respawn anchor's.
pub(crate) fn explode_burning(world: &mut World, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    blow(world, None, layer, at, power, wreck, Rule::Burning);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    Plain,
    Fireball,
    Burning,
}

/// Lights about a third of `cells`, each free over a solid block, as the Java game's
/// `Explosion.finalizeExplosion` with fire.
fn set_fires(world: &mut World, layer: Entity, cells: &[[i32; 3]]) {
    for &[x, y, z] in cells {
        if world.get_resource_or_insert_with(blocks::Dice::default).roll(3) != 0 {
            continue;
        }
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y - 1)) else { continue };
        if level.registry().get(level.block(x, y - 1, z)).is_some_and(|under| under.is_solid()) {
            blocks::light_fire(world, layer, [x, y, z]);
        }
    }
}

fn blow(world: &mut World, source: Option<Entity>, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck, rule: Rule) {
    crate::blocks::vibrate(world, layer, at, crate::blocks::GameEvent::Explode, false);
    let fireball = rule == Rule::Fireball;
    let (cells, free) = if wreck != Wreck::Nothing { rays::broken(world, layer, at, power) } else { Default::default() };
    let (power, cells) = if crate::plugins::listening::<ExplosionEvent>(world) {
        let kind = source.and_then(|source| world.get::<entities::Kind>(source)).map(|kind| kind.0.clone());
        let blast = crate::plugins::offer_in(world, layer, ExplosionEvent::new(at, kind, power, cells));
        if blast.is_cancelled() {
            if let Some(source) = source {
                mobs::remove(world, source);
            }
            return;
        }
        (blast.power, blast.blocks)
    } else {
        (power, cells)
    };
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    let flash = LevelEvent { event: LevelEventEvent::ParticleExplosion, position: Vec3f { x, y, z }, data: power as i32 };
    match source {
        Some(source) => entities::tell_viewers(world, source, layer, cell, vec![sapling_net::encode(&flash)]),
        None => blocks::to_viewers(world, layer, cell, &[sapling_net::encode(&flash)]),
    }
    let bang = LevelSoundEvent { sound_id: "explode".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    late_sounds::play_later(world, 1, layer, cell, sapling_net::encode(&bang));
    if let Some(source) = source {
        mobs::remove(world, source);
    }

    let players: Vec<(Entity, [f32; 3])> = crate::components::in_world(world).into_iter().filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).filter(|(player, _)| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)).collect();
    for (player, feet) in players {
        let eyes = [feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]];
        let seen = seen(world, layer, at, feet, PLAYER_SIZE);
        if let Some((harm, thrown)) = reaches(world, at, eyes, feet, power, seen, fireball) {
            vitals::blasted(world, player, harm, at, thrown);
        }
    }
    // Items lying in it are gone, as the stacks BDS's first TNT left were with its second.
    let reach = 2.0 * power;
    for (item, _) in drops::lying_in(world, layer, at.map(|c| c - reach), at.map(|c| c + reach)) {
        let far = world.get::<Body>(item).map_or(f32::MAX, |its| distance(its.at, at));
        if far < reach {
            mobs::remove(world, item);
        }
    }
    crate::vehicles::blasted(world, layer, at, reach);
    throw_primed(world, layer, at, reach);
    let mut mobs = world.query::<(Entity, &InLayer, &Body, &Health, &NetId)>();
    let near: Vec<(Entity, u64, Body)> = mobs.iter(world).filter(|(_, in_layer, ..)| in_layer.0 == layer).map(|(mob, _, body, _, id)| (mob, id.0, *body)).collect();
    // The source's removal lands later in the tick; it is gone to the blast already.
    for (mob, id, its) in near.into_iter().filter(|(mob, ..)| Some(*mob) != source) {
        if crate::dragon::takes_blast(world, mob, source.is_some()) {
            blast_mob(world, layer, mob, id, &its, at, reach);
        }
    }
    crate::dragon::blasted(world, layer, at, power);
    if wreck != Wreck::Nothing {
        let leaves = match wreck {
            Wreck::Leaves => 1.0,
            Wreck::Decays => 1.0 / power,
            Wreck::Nothing | Wreck::Breaks => 0.0,
        };
        blocks::blow_out(world, layer, &cells, leaves);
    }
    if rule == Rule::Burning {
        set_fires(world, layer, &[cells, free].concat());
    }
}

/// How hard a blast at `at` that reaches `reach` hits a mob at `feet`, `size` wide and high:
/// BDS measures to its feet and takes half its width off (`Explosion::explode`).
fn impact(world: &World, layer: Entity, at: [f32; 3], feet: [f32; 3], size: [f32; 2], reach: f32) -> Option<f32> {
    let far = (distance(feet, at) - size[0] / 2.0).max(0.0);
    (far <= reach).then(|| (1.0 - far / reach) * seen(world, layer, at, feet, size))
}

/// The way from `from` to `to`, `long` long.
fn towards(from: [f32; 3], to: [f32; 3], long: f32) -> [f32; 3] {
    let far = distance(from, to);
    if far < 1e-4 { [0.0; 3] } else { std::array::from_fn(|axis| (to[axis] - from[axis]) / far * long) }
}

/// A mob in a blast at `at`: hurt by at least 1 however hidden, thrown off as by a hit from the
/// blast and, unless no hit throws it, towards its eyes by the blast's impact (bds-blasts.txt).
fn blast_mob(world: &mut World, layer: Entity, mob: Entity, id: u64, its: &Body, at: [f32; 3], reach: f32) {
    let Some(impact) = impact(world, layer, at, its.at, its.size, reach) else { return };
    let harm = (impact * impact + impact) / 2.0 * HARM * reach + 1.0;
    let hurt = crate::damage::ask(world, mob, harm, Cause::EntityExplosion, None).and_then(|harm| {
        let mut health = world.get::<Health>(mob).copied()?;
        // Resistance that takes all of it still hurts and throws (the drill's pigs had it).
        let resisted = !health.is_guarded() && health.left() > 0.0;
        (health.hurt(harm) || resisted).then(|| {
            *world.get_mut::<Health>(mob).expect("read above") = health;
            entities::tell_viewers(world, mob, layer, its.at.map(|c| c.floor() as i32), vec![harm::strike(id, &health, Cause::EntityExplosion, None)]);
        })
    });
    let kind = world.get::<entities::Kind>(mob).map(|kind| kind.0.clone()).unwrap_or_default();
    if sapling_data::mob_traits(&kind).steadfast {
        return;
    }
    let knock = match hurt {
        Some(()) => {
            let [x, _, z] = towards([at[0], 0.0, at[2]], [its.at[0], 0.0, its.at[2]], KNOCK);
            [x, KNOCK, z]
        }
        None => [0.0; 3],
    };
    let push = towards(at, [its.at[0], its.at[1] + its.size[1] * EYES, its.at[2]], impact);
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.throw(std::array::from_fn(|axis| knock[axis] + push[axis]));
    }
}

/// Primed TNT in a blast at `at` that reaches `reach` is pushed away by its impact.
fn throw_primed(world: &mut World, layer: Entity, at: [f32; 3], reach: f32) {
    let primed: Vec<(Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<crate::tnt::Primed>>().iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(entity, _, body)| (entity, *body)).collect();
    for (entity, its) in primed {
        if let Some(impact) = impact(world, layer, at, its.at, its.size, reach) {
            let push = towards(at, its.at, impact);
            if let Some(mut body) = world.get_mut::<Body>(entity) {
                body.motion = std::array::from_fn(|axis| body.motion[axis] + push[axis]);
            }
        }
    }
}

/// A tick of a lit fuse; whether the mob burst, and is gone.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> bool {
    let Some(left) = world.get::<Fuse>(entity).map(|fuse| fuse.0 - 1) else { return false };
    let Some(power) = sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunts.explodes).filter(|_| left < 0) else {
        world.entity_mut(entity).insert(Fuse(left));
        tell(world, entity, layer, id, kind, body, left);
        if left == FUSE - 1 - HISSES {
            let position = Vec3f { x: body.at[0], y: body.at[1] + HEAD * body.size[1], z: body.at[2] };
            let hiss = LevelSoundEvent { sound_id: "fuse".into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
            entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&hiss)]);
        }
        return false;
    };
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    let power = if world.get::<super::struck::Charged>(entity).is_some() { power * 2.0 } else { power };
    explode(world, Some(entity), layer, body.at, power, if griefs { Wreck::Decays } else { Wreck::Nothing });
    true
}
