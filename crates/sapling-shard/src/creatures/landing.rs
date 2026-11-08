//! What a thrown thing does where it lands, on a block or on the player or mob it met (flight.rs),
//! by the vanilla pack's `on_hit`: a potion breaks (splashes.rs), a lingering one leaving a cloud
//! (shots/clouds.rs); a snowball throws whom it hits and costs a blaze 3; an egg throws whom it hits
//! and may hatch chicks; a pearl takes its thrower there; a bottle o' enchanting lets out
//! experience; a wind charge bursts (charges.rs).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::SpawnReason;
use sapling_data::shots;

use super::arrows::Lands;
use super::charges::{self, Gust};
use super::flight::{self, Loosed};
use super::harm;
use super::health::Cause;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::InLayer;
use crate::{entities, view, vitals};

const BLAZE: &str = "minecraft:blaze";
const CHICKEN: &str = "minecraft:chicken";
const ENDERMITE: &str = "minecraft:endermite";
const SNOW_PUFF: i32 = 15;
const SNOW_PUFFS: usize = 6;
/// An item's crumbs (`iconcrack`), its network id above: six of a broken egg, five of a splash
/// potion (research/captures/bds-f14cthrow.txt).
const ITEM_CRACK: i32 = 14;
const EGG_CRACKS: usize = 6;
pub(super) const POTION_CRACKS: usize = 5;
/// A pearl's thrower is heard leaving and arriving this far over the feet (bds-f14cthrow.txt).
const EYES: f32 = 1.62;
/// A player's wind charge (its pack's `explode`): power 1.2 reaching twice that, thrown 1.22 times
/// as hard, 1 off whom it hit; mobs are pushed too.
// TODO: the players' reach and throw are the Java game's explosion rule over the pack's numbers.
const WIND: Gust = Gust { reach: 2.4, push: 1.22, harm: 1.0, mobs: true };
/// The splash a bottle o' enchanting breaks with (the Java game's colour).
const BOTTLE_COLOUR: i32 = 0xFF38_5DC6_u32 as i32;

/// The thrown `shot` lands at `at`, flying from `from`, on the player or mob `struck` if it met one.
pub(super) fn land(world: &mut World, shot: Entity, layer: Entity, at: [f32; 3], from: [f32; 3], struck: Option<(Entity, bool)>) {
    let Some((archer, by, _, kind)) = flight::shooter(world, shot) else { return };
    let thrower = world.get::<Loosed>(shot).and_then(|loosed| loosed.player);
    let heard = match kind.lands {
        Lands::Splash(_) | Lands::Lingers(_) | Lands::Experience => Some("glass"),
        Lands::Snowball | Lands::Egg | Lands::Pearl => Some("undefined"),
        Lands::Hurts | Lands::Wind => None,
    };
    if let Some(heard) = heard {
        world.entity_mut(shot).insert(HeardGoing(flight::sound(heard, at, "")));
    }
    match kind.lands {
        Lands::Hurts => {}
        Lands::Splash(data) => super::splashes::burst(world, layer, at, from, data, struck.map(|(struck, _)| struck)),
        Lands::Lingers(data) => {
            splash(world, layer, at, shots::potion_colour(data));
            world.entity_mut(shot).insert(Leaves(Box::new(move |world| {
                crate::shots::clouds::spawn(world, layer, at, data);
            })));
        }
        Lands::Snowball => {
            if let Some((struck, is_player)) = struck {
                let hurts = world.get::<entities::Kind>(struck).is_some_and(|kind| kind.0 == BLAZE);
                knock(world, struck, is_player, layer, from, if hurts { shots::SNOWBALL_BLAZE } else { 0.0 }, thrower, by);
            }
            burst_of_snow(world, layer, at);
        }
        Lands::Egg => {
            if let Some((struck, is_player)) = struck {
                knock(world, struck, is_player, layer, from, 0.0, thrower, by);
            }
            view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &cracks("minecraft:egg", EGG_CRACKS, at));
            let chicks = {
                let mut dice = world.get_resource_or_insert_with(Dice::default);
                shots::chicks(&mut || dice.fraction())
            };
            for _ in 0..chicks {
                hatch(world, layer, at);
            }
        }
        Lands::Pearl => {
            if let Some(player) = thrower.filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && !vitals::is_dead(world, *player)) {
                let left = world.get::<crate::components::Position>(player).map_or(at, |position| position.feet);
                vitals::pearled(world, player, at, shots::PEARL_FALL);
                let event = |event, [x, y, z]: [f32; 3]| sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data: 0 });
                let up = |[x, y, z]: [f32; 3]| [x, y + EYES, z];
                let shown = [event(LevelEventEvent::ParticleEndermanTeleport, at), event(LevelEventEvent::SoundPortal, up(left)), event(LevelEventEvent::SoundPortal, up(at))];
                view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &shown);
            }
            let lucky = world.get_resource_or_insert_with(Dice::default).fraction() < shots::ENDERMITE_CHANCE;
            if lucky {
                super::spawn(world, layer, ENDERMITE, at, SpawnReason::Thrown);
            }
        }
        Lands::Experience => {
            splash(world, layer, at, BOTTLE_COLOUR);
            let points = {
                let mut dice = world.get_resource_or_insert_with(Dice::default);
                shots::bottle_experience(&mut || dice.fraction())
            };
            world.entity_mut(shot).insert(Leaves(Box::new(move |world| crate::orbs::spawn(world, layer, at, points))));
        }
        Lands::Wind => {
            let player = struck.filter(|(_, is_player)| *is_player).map(|(struck, _)| struck);
            let at = if struck.is_some() { at } else { drawn_back(at, from) };
            charges::gust(world, layer, at, player, &archer, world.get::<Body>(by).map_or(from, |body| body.at), WIND);
            if let Some((mob, false)) = struck {
                harm::wound_by(world, mob, layer, WIND.harm, Cause::Projectile, thrower);
            }
            let [x, y, z] = at;
            let packets = [sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleWindExplosion, position: Vec3f { x, y, z }, data: 1 }), flight::sound("wind_charge.burst", at, "")];
            view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &packets);
        }
    }
}

/// What a landed thing leaves as it goes the tick after: a lingering potion's cloud, a bottle's orbs,
/// announced just before its RemoveEntity (research/captures/bds-f14cthrow.txt).
#[derive(Component)]
pub(super) struct Leaves(Box<dyn FnOnce(&mut World) + Send + Sync>);

/// Runs what `shot` leaves behind, if anything.
pub(super) fn leave(world: &mut World, shot: Entity) {
    if let Some(Leaves(left)) = world.entity_mut(shot).take::<Leaves>() {
        left(world);
    }
}

/// What a landed thing is heard as, where it landed, told just after its RemoveEntity the tick after:
/// `glass` for a bottle, `undefined` for the rest (bds-f14cthrow.txt).
#[derive(Component)]
struct HeardGoing(Bytes);

/// Takes the sound `shot` is heard as it goes, if it landed.
pub(super) fn heard_going(world: &mut World, shot: Entity) -> Option<Bytes> {
    world.entity_mut(shot).take::<HeardGoing>().map(|heard| heard.0)
}

/// What the viewers of a cell hear once this tick's RemoveEntity of the shots gone there is sent.
#[derive(Resource, Default)]
pub(crate) struct AfterGone(Vec<(Entity, [i32; 3], Vec<Bytes>)>);

/// Queues `sounds` for the viewers of `cell` in `layer`, after this tick's RemoveEntity packets.
pub(super) fn after_gone(world: &mut World, layer: Entity, cell: [i32; 3], sounds: Vec<Bytes>) {
    world.get_resource_or_insert_with(AfterGone::default).0.push((layer, cell, sounds));
}

/// Tells what [`after_gone`] queued; runs after `mobs::update`.
pub(crate) fn tell_after_gone(world: &mut World) {
    let Some(mut queued) = world.get_resource_mut::<AfterGone>() else { return };
    for (layer, cell, sounds) in std::mem::take(&mut queued.0) {
        view::to_viewers(world, layer, cell, &sounds);
    }
}

/// `count` crumbs of the item `name` at `at`.
pub(super) fn cracks(name: &str, count: usize, [x, y, z]: [f32; 3]) -> Vec<Bytes> {
    let data = sapling_data::item_network_id(name).map_or(0, |id| i32::from(id) << 16) | ITEM_CRACK;
    vec![sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleProjectileHit, position: Vec3f { x, y, z }, data }); count]
}

/// Whether a thing that lands so goes the tick after, whenever it lands: every thrown thing but a wind
/// charge, its RemoveEntity coming with the glass or `undefined` heard then (splashes.rs; bds-f14cthrow.txt,
/// bds-s7shots.txt, bds-p2shoot.txt, bds-small.txt).
pub(super) fn goes_late(lands: Lands) -> bool {
    !matches!(lands, Lands::Hurts | Lands::Wind)
}

/// Where a charge that met a block at `at`, flying from `from`, bursts: [`charges::BACK`] back.
fn drawn_back(at: [f32; 3], from: [f32; 3]) -> [f32; 3] {
    let way: [f32; 3] = std::array::from_fn(|axis| at[axis] - from[axis]);
    let long = way.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    std::array::from_fn(|axis| at[axis] - way[axis] / long * charges::BACK)
}

/// A bottle breaking at `at`, its splash of `colour` (its glass is [`HeardGoing`]).
fn splash(world: &mut World, layer: Entity, at: [f32; 3], colour: i32) {
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    view::to_viewers(world, layer, cell, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: colour })]);
}

/// A snowball's puffs where it lands (research/captures/bds-p2shoot.txt).
fn burst_of_snow(world: &mut World, layer: Entity, at: [f32; 3]) {
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    let puff = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleProjectileHit, position: Vec3f { x, y, z }, data: SNOW_PUFF });
    view::to_viewers(world, layer, cell, &vec![puff; SNOW_PUFFS]);
}

/// Something thrown that hurts little throws whom it hit as a hit from `from` does, costing it `harm`,
/// and turns a hunter it hit on `by`, who threw it.
#[allow(clippy::too_many_arguments)]
fn knock(world: &mut World, struck: Entity, is_player: bool, layer: Entity, from: [f32; 3], harm: f32, thrower: Option<Entity>, by: Entity) {
    if is_player {
        return vitals::knocked(world, struck, from);
    }
    if harm > 0.0 {
        harm::wound_by(world, struck, layer, harm, Cause::Projectile, thrower);
    } else {
        harm::flinch(world, struck, layer, Cause::Projectile);
    }
    super::hunting::hurt_by(world, struck, by);
    let Some(at) = world.get::<Body>(struck).map(|body| body.at) else { return };
    let (dx, dz) = (at[0] - from[0], at[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    super::hits::throw_at_once(world, struck, layer, [dx / far * shots::KNOCK, shots::KNOCK, dz / far * shots::KNOCK]);
}

/// A baby chicken hatches at `at`, shown as one.
fn hatch(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<()> {
    let chick = super::spawn(world, layer, CHICKEN, at, SpawnReason::Thrown).and_then(|id| entities::find(world, id))?;
    super::husbandry::be_born(world, chick, CHICKEN);
    Some(())
}
