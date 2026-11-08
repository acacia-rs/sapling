//! Mobs that blow up, as BDS's creeper did (docs/research/mob-attacks.md, "Creepers"): lit near
//! its prey, it burns a fuse down and bursts, hurting and throwing who is near and breaking blocks
//! (docs/research/block-uses.md, "Blasts").
// TODO: Blast Protection spares no mob's throw.

mod items;
mod reached;
pub(super) mod rays;

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, ExplosionEvent};

use super::health::{Cause, Health};
use reached::{blast_mob, harm_by, throw_falling, throw_primed};
pub(super) use reached::{impact, towards};
use crate::blocks;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings};
use crate::{entities, late_sounds, mobs, vitals};

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
pub(super) const EYES: f32 = 0.9;
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

/// A lit fuse: the ticks left of it, and whether it was forced (it does not go out).
#[derive(Component)]
pub(super) struct Fuse(i32, bool);

pub(super) fn is_lit(world: &World, entity: Entity) -> bool { world.get::<Fuse>(entity).is_some() }

fn tell(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, left: i32) {
    let told = sapling_data::mob_fuse(id, entities::mob_flags(world, entity, kind, MetadataFlags1::IGNITED, true), left);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// Lights the mob's fuse, if it is not lit.
pub(super) fn light(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    light_as(world, entity, layer, id, kind, body, false);
}

/// Lights the fuse of a creeper summoned with `minecraft:start_exploding_forced`: BDS's hissed 2
/// ticks after and burst 30 after, with no prey (research/captures/bds-fights.txt).
pub(super) fn light_forced(world: &mut World, entity: Entity, layer: Entity, kind: &str) {
    if let (Some(id), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<Body>(entity).copied()) {
        light_as(world, entity, layer, id, kind, &body, true);
    }
}

fn light_as(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, forced: bool) {
    if !is_lit(world, entity) {
        world.entity_mut(entity).insert(Fuse(FUSE - 1, forced));
        tell(world, entity, layer, id, kind, body, FUSE - 1);
    }
}

/// Puts the mob's fuse out, if it is lit and not forced: its prey is gone or far.
pub(super) fn put_out(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if world.get::<Fuse>(entity).is_some_and(|fuse| !fuse.1) && world.entity_mut(entity).take::<Fuse>().is_some() {
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
/// A ghast's fireball's goes by how far the body's `feet` are, costs no 1 more and is the same on
/// every difficulty (research/captures/bds-ghast-findings.md); a bed's or anchor's goes by the feet
/// too and is the same on every difficulty (bds-n13more.txt: 6.872 on all three, Resistance IV).
fn reaches(world: &World, at: [f32; 3], eyes: [f32; 3], feet: [f32; 3], power: f32, seen: f32, rule: Rule) -> Option<(f32, [f32; 3])> {
    let away: [f32; 3] = std::array::from_fn(|axis| eyes[axis] - at[axis]);
    let far = distance(eyes, at);
    let measured = if rule == Rule::Plain { far } else { distance(feet, at) };
    let impact = (1.0 - measured / (2.0 * power)) * seen;
    let harm = (impact * impact + impact) / 2.0 * HARM * 2.0 * power + if rule == Rule::Fireball { 0.0 } else { 1.0 };
    let harm = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0.0,
        _ if rule != Rule::Plain => harm,
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
    blow(world, source, layer, at, power, wreck, Rule::Plain, |_| {});
}

/// As [`explode`], primed TNT's or a TNT minecart's: measured from the feet and the same on every
/// difficulty (bds-r1ench.txt: TNT 1.81 from the feet cost 7.769 of 60 on easy under Resistance IV,
/// impact 0.774, and threw towards the eyes by that).
pub(crate) fn explode_primed(world: &mut World, source: Entity, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    blow(world, Some(source), layer, at, power, wreck, Rule::Primed, |_| {});
}

/// As [`explode`], a ghast's fireball's blast, by its own rule ([`reaches`]).
pub(super) fn explode_fireball(world: &mut World, source: Entity, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    blow(world, Some(source), layer, at, power, wreck, Rule::Fireball, |_| {});
}

/// As [`explode`] with no source, a blast that sets fire: a bed's or a respawn anchor's, which `clears`
/// takes away first, its cells listed with the blast's (bds-p2last.txt, bds-v2nether.txt).
pub(crate) fn explode_burning(world: &mut World, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck, clears: impl FnOnce(&mut World)) {
    blow(world, None, layer, at, power, wreck, Rule::Burning, clears);
}

/// The particles BDS sends after a blast's flash (`LevelEventGeneric` 2026, zigzagged as acacia-proto does not,
/// sleep.rs): its middle, power and every cell it blew, the tags in name order as BDS's (bds-fights.txt).
pub(crate) fn blast_particles([x, y, z]: [f32; 3], power: f32, cells: &[[i32; 3]]) -> bytes::Bytes {
    use acacia_proto::nbt::{Nbt, Value};
    let float = |name: String, value: f32| Nbt { name, value: Value::Float(value) };
    let mut nbt = vec![float("originX".into(), x), float("originY".into(), y), float("originZ".into(), z), float("radius".into(), power)];
    nbt.push(Nbt { name: "size".into(), value: Value::Int(cells.len() as i32) });
    for (i, cell) in cells.iter().enumerate() {
        nbt.extend(["x", "y", "z"].into_iter().zip(cell).map(|(axis, c)| float(format!("pos{i}{axis}"), *c as f32)));
    }
    nbt.sort_by(|a, b| a.name.cmp(&b.name));
    sapling_net::encode(&acacia_proto::packets::LevelEventGeneric { event_id: 2026 << 1, nbt })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    Plain,
    Primed,
    Fireball,
    Burning,
}

/// Lights about a third of `cells`, each free over a solid block, as the Java game's
/// `Explosion.finalizeExplosion` with fire; each left unlit is listed as air written again, as BDS's anchor and
/// bed blasts listed them, a broken cell twice (bds-p2last.txt, bds-v2nether.txt).
fn set_fires(world: &mut World, layer: Entity, cells: &[[i32; 3]]) {
    for &[x, y, z] in cells {
        let rolled = world.get_resource_or_insert_with(blocks::Dice::default).roll(3) == 0;
        let level = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y - 1));
        if rolled && level.is_some_and(|level| level.registry().get(level.block(x, y - 1, z)).is_some_and(|under| under.is_solid())) {
            blocks::light_fire(world, layer, [x, y, z]);
        } else {
            blocks::log_again(world, layer, [x, y, z]);
        }
    }
}

/// A blast, and first what `clears` takes away; what both write is shown together, sub-chunk by sub-chunk,
/// and then the blast's drops, as BDS showed a tick's writes (bds-p2last.txt, bds-blasts.txt).
#[allow(clippy::too_many_arguments)]
fn blow(world: &mut World, source: Option<Entity>, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck, rule: Rule, clears: impl FnOnce(&mut World)) {
    let left = blocks::logged(world, |world| {
        clears(world);
        blast(world, source, layer, at, power, wreck, rule)
    });
    blocks::spill_blown(world, layer, left);
}

fn blast(world: &mut World, source: Option<Entity>, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck, rule: Rule) -> blocks::Blown {
    // Heard the next tick with what it hurt, the nearer winning (bds-b1sculk.txt: its items' particle a tick after the flash).
    crate::blocks::vibrate_next_tick(world, layer, at.map(|c| c.floor() as i32), crate::blocks::GameEvent::Explode, None);
    let (cells, free) = if wreck != Wreck::Nothing { rays::broken(world, source, layer, at, power) } else { Default::default() };
    let (power, cells) = if crate::plugins::listening::<ExplosionEvent>(world) {
        let kind = source.and_then(|source| world.get::<entities::Kind>(source)).map(|kind| kind.0.clone());
        let blast = crate::plugins::offer_in(world, layer, ExplosionEvent::new(at, kind, power, cells));
        if blast.is_cancelled() {
            if let Some(source) = source {
                mobs::remove(world, source);
            }
            return blocks::Blown::default();
        }
        (blast.power, blast.blocks)
    } else {
        (power, cells)
    };
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    let flash = LevelEvent { event: LevelEventEvent::ParticleExplosion, position: Vec3f { x, y, z }, data: power as i32 };
    // A burning blast lists the free cells it reached too (bds-v2nether.txt); others only what they broke (bds-fights.txt).
    let listed = if rule == Rule::Burning { [&cells[..], &free[..]].concat() } else { cells.clone() };
    let shown = vec![sapling_net::encode(&flash), blast_particles(at, power, &listed)];
    match source {
        Some(source) => entities::tell_viewers(world, source, layer, cell, shown),
        None => blocks::to_viewers(world, layer, cell, &shown),
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
        if let Some((harm, thrown)) = reaches(world, at, eyes, feet, power, seen, rule) {
            // BDS's player hurt by a bed's blast was told cause 10, block_explosion (bds-n13more.txt).
            let cause = if rule == Rule::Burning { Cause::BlockExplosion } else { Cause::EntityExplosion };
            vitals::blasted(world, player, harm, at, thrown, cause);
        }
    }
    // Orbs lying in it are gone, as an orb 2.8 off a creeper's blast was the tick after (bds-a3creeper2.txt).
    let reach = 2.0 * power;
    items::blast_items(world, layer, at, reach);
    let lying: Vec<Entity> = world.query_filtered::<(Entity, &InLayer), With<crate::orbs::Orb>>().iter(world).filter(|(_, in_layer)| in_layer.0 == layer).map(|(orb, _)| orb).collect();
    for gone in lying {
        if world.get::<Body>(gone).is_some_and(|its| distance(its.at, at) < reach) {
            mobs::remove(world, gone);
        }
    }
    crate::vehicles::blasted(world, layer, |world, feet, size| impact(world, layer, at, feet, size, reach).map(|impact| harm_by(impact, reach)));
    throw_primed(world, layer, at, reach);
    throw_falling(world, layer, at, reach);
    let mut mobs = world.query::<(Entity, &InLayer, &Body, &Health, &NetId)>();
    let near: Vec<(Entity, u64, Body)> = mobs.iter(world).filter(|(_, in_layer, ..)| in_layer.0 == layer).map(|(mob, _, body, _, id)| (mob, id.0, *body)).collect();
    // The source's removal lands later in the tick; it is gone to the blast already.
    for (mob, id, its) in near.into_iter().filter(|(mob, ..)| Some(*mob) != source) {
        if crate::dragon::takes_blast(world, mob, source.is_some()) {
            blast_mob(world, layer, mob, id, &its, at, reach);
        }
    }
    crate::dragon::blasted(world, layer, at, power);
    let leaves = match wreck {
        Wreck::Leaves => 1.0,
        Wreck::Decays => 1.0 / power,
        Wreck::Nothing | Wreck::Breaks => 0.0,
    };
    let left = if wreck != Wreck::Nothing { blocks::break_blown(world, layer, &cells, leaves) } else { Default::default() };
    // A ghast's fireball burns where it may grief (bds-d3ghast.txt, bds-d3ghast2.txt).
    if rule == Rule::Burning || (rule == Rule::Fireball && wreck != Wreck::Nothing) {
        set_fires(world, layer, &[&cells[..], &free[..]].concat());
    }
    left
}

/// A tick of a lit fuse; whether the mob burst, and is gone.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> bool {
    let Some((left, forced)) = world.get::<Fuse>(entity).map(|fuse| (fuse.0 - 1, fuse.1)) else { return false };
    let Some(power) = sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunts.explodes).filter(|_| left < 0) else {
        world.entity_mut(entity).insert(Fuse(left, forced));
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
