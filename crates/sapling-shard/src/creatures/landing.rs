//! What a thrown thing does where it lands, on a block or on the player or mob it met (flight.rs),
//! by the vanilla pack's `on_hit`: a potion breaks (splashes.rs), a lingering one leaving a cloud
//! (shots/clouds.rs); a snowball throws whom it hits and costs a blaze 3; an egg throws whom it hits
//! and may hatch chicks; a pearl takes its thrower there; a bottle o' enchanting lets out
//! experience; a wind charge bursts (charges.rs).
// TODO: a snowball's and an egg's particles are not shown.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
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
use crate::{entities, late_sounds, view, vitals};

const BLAZE: &str = "minecraft:blaze";
const CHICKEN: &str = "minecraft:chicken";
const ENDERMITE: &str = "minecraft:endermite";
/// A player's wind charge (its pack's `explode`): power 1.2 reaching twice that, thrown 1.22 times
/// as hard, 1 off whom it hit.
// TODO: the reach and the throw are the Java game's explosion rule over the pack's numbers.
const WIND: Gust = Gust { reach: 2.4, push: 1.22, harm: 1.0 };
/// The splash a bottle o' enchanting breaks with (the Java game's colour).
const BOTTLE_COLOUR: i32 = 0xFF38_5DC6_u32 as i32;

/// The thrown `shot` lands at `at`, flying from `from`, on the player or mob `struck` if it met one.
pub(super) fn land(world: &mut World, shot: Entity, layer: Entity, at: [f32; 3], from: [f32; 3], struck: Option<(Entity, bool)>) {
    let Some((archer, by, _, kind)) = flight::shooter(world, shot) else { return };
    let thrower = world.get::<Loosed>(shot).and_then(|loosed| loosed.player);
    match kind.lands {
        Lands::Hurts => {}
        Lands::Splash(data) => super::splashes::burst(world, layer, at, from, data, struck.map(|(struck, _)| struck)),
        Lands::Lingers(data) => {
            splash(world, layer, at, shots::potion_colour(data));
            crate::shots::clouds::spawn(world, layer, at, data);
        }
        Lands::Snowball => {
            if let Some((struck, is_player)) = struck {
                let hurts = world.get::<entities::Kind>(struck).is_some_and(|kind| kind.0 == BLAZE);
                knock(world, struck, is_player, layer, from, if hurts { shots::SNOWBALL_BLAZE } else { 0.0 }, thrower);
            }
        }
        Lands::Egg => {
            if let Some((struck, is_player)) = struck {
                knock(world, struck, is_player, layer, from, 0.0, thrower);
            }
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
                vitals::pearled(world, player, at, shots::PEARL_FALL);
                let [x, y, z] = at;
                view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::SoundEndermanTeleport, position: Vec3f { x, y, z }, data: 0 })]);
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
            crate::orbs::spawn(world, layer, at, points);
        }
        Lands::Wind => {
            let player = struck.filter(|(_, is_player)| *is_player).map(|(struck, _)| struck);
            charges::gust(world, layer, at, player, &archer, world.get::<Body>(by).map_or(from, |body| body.at), WIND);
            if let Some((mob, false)) = struck {
                harm::wound_by(world, mob, layer, WIND.harm, Cause::Projectile, thrower);
            }
            let [x, y, z] = at;
            let packets = [sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleWindExplosion, position: Vec3f { x, y, z }, data: 0 }), flight::sound("wind_charge.burst", at, "")];
            view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &packets);
        }
    }
}

/// A bottle breaking at `at`, its splash of `colour` and the glass heard.
fn splash(world: &mut World, layer: Entity, at: [f32; 3], colour: i32) {
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    view::to_viewers(world, layer, cell, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: colour })]);
    late_sounds::play_later(world, 1, layer, cell, flight::sound("glass", at, ""));
}

/// Something thrown that hurts little throws whom it hit as a hit from `from` does, costing it `harm`.
fn knock(world: &mut World, struck: Entity, is_player: bool, layer: Entity, from: [f32; 3], harm: f32, thrower: Option<Entity>) {
    if is_player {
        return vitals::knocked(world, struck, from);
    }
    if harm > 0.0 {
        harm::wound_by(world, struck, layer, harm, Cause::Projectile, thrower);
    }
    let Some(mut body) = world.get_mut::<Body>(struck) else { return };
    let (dx, dz) = (body.at[0] - from[0], body.at[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    body.throw([dx / far * shots::KNOCK, shots::KNOCK, dz / far * shots::KNOCK]);
}

/// A baby chicken hatches at `at`, shown as one.
fn hatch(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<()> {
    let chick = super::spawn(world, layer, CHICKEN, at, SpawnReason::Thrown).and_then(|id| entities::find(world, id))?;
    super::husbandry::be_born(world, chick, CHICKEN);
    Some(())
}
