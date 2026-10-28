//! A mace's smash: a hit with a mace as its holder falls 1.5 or more, as BDS's
//! (`sapling_data::enchant::mace` has the rules, research/captures/bds-gameplay-findings.md the
//! evidence). The struck mob flashes `MaceSmash`, dust rises where it stands, the hitter stops
//! falling (its motion told as nought, its fall forgotten) and Wind Burst launches it, and the mobs
//! around are shoved.
// TODO: players around are not shoved, nor are the hitter's pets spared; a smash on a player
// flashes as a plain hit; `mace.smash_air` for a target off the ground was not drilled; the
// sound goes a tick early. Breach only pierces a player's armour: mobs wear none here.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::enchant::{self, Enchant, mace};

use super::Health;
use crate::body::Body;
use crate::components::{InLayer, Position};
use crate::entities::Kind;

pub(super) const MACE: &str = "minecraft:mace";
/// The `HurtAnimation` cause BDS flashes a smash with: `MaceSmash`, which the API does not name.
pub(super) const MACE_SMASH: i32 = 34;
/// BDS's motion of a mob standing (a tick's gravity after drag), which the shove adds to.
const RESTING: f32 = 0.0784;

/// A smash: how far the hitter had fallen.
#[derive(Clone, Copy)]
pub(super) struct Smash {
    pub fallen: f32,
}

/// The smash `player`, holding `held`, makes with its hit now, if any.
pub(super) fn smash(world: &World, player: Entity, held: Option<&str>) -> Option<Smash> {
    let fallen = crate::vitals::fallen(world, player);
    (held == Some(MACE) && fallen >= mace::SMASHES_FROM).then_some(Smash { fallen })
}

/// What `smash` adds to the hit of the mace `weapon`.
pub(super) fn bonus(smash: Option<Smash>, weapon: Option<&Value>) -> f32 {
    smash.map_or(0.0, |smash| mace::smash_bonus(smash.fallen, enchant::level(weapon, Enchant::Density)))
}

/// What a smash's target's viewers see and hear of it: dust at the target's feet `at` and the
/// smash from the hitter's eyes (bds-mace.txt).
pub(super) fn shown(smash: Smash, hitter: [f32; 3], at: [f32; 3]) -> [Bytes; 2] {
    let [x, y, z] = at;
    let dust = LevelEvent { event: LevelEventEvent::AddParticleSmashAttackGroundDust, position: Vec3f { x, y, z }, data: 0 };
    let name = if smash.fallen > mace::HEAVY_FROM { "mace.heavy_smash_ground" } else { "mace.smash_ground" };
    [sapling_net::encode(&dust), sapling_net::encode(&heard(name, [hitter[0], hitter[1] + sapling_data::EYE_HEIGHT, hitter[2]]))]
}

fn heard(name: &str, [x, y, z]: [f32; 3]) -> LevelSoundEvent {
    LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None }
}

/// After a smash on `target` at `at` landed: the hitter stops and Wind Burst on `weapon`
/// launches it, and the mobs around are shoved.
pub(super) fn follow(world: &mut World, player: Entity, target: Entity, at: [f32; 3], weapon: Option<&Value>) {
    crate::vitals::pushed(world, player, [0.0; 3]);
    crate::vitals::break_fall(world, player);
    if let Some(up) = mace::wind_burst(enchant::level(weapon, Enchant::WindBurst)) {
        burst(world, player, up);
    }
    shove(world, player, target, at);
}

/// Wind Burst launches the hitter `up`, in a wind charge's burst at its feet.
fn burst(world: &mut World, player: Entity, up: f32) {
    let (Some(feet), Some(layer)) = (world.get::<Position>(player).map(|position| position.feet), world.get::<InLayer>(player).map(|layer| layer.0)) else { return };
    crate::vitals::pushed(world, player, [0.0, up, 0.0]);
    let [x, y, z] = feet;
    let flash = LevelEvent { event: LevelEventEvent::ParticleWindExplosion, position: Vec3f { x, y, z }, data: 3 };
    let packets = [sapling_net::encode(&flash), crate::dragon::blast_particles(feet, 3.0), sapling_net::encode(&heard("wind_charge.burst", feet))];
    crate::blocks::to_viewers(world, layer, feet.map(|c| c.floor() as i32), &packets);
}

/// Shoves each mob within reach of `at`, but the hitter and its target.
fn shove(world: &mut World, player: Entity, target: Entity, at: [f32; 3]) {
    let Some(layer) = world.get::<InLayer>(target).map(|layer| layer.0) else { return };
    let near = |body: &Body| (0..3).all(|axis| (body.at[axis] - at[axis]).abs() <= mace::SHOVE_REACH);
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body, &Kind), With<Health>>();
    let shoved: Vec<(Entity, Body, String)> = mobs.iter(world).filter(|(mob, in_layer, body, _)| in_layer.0 == layer && *mob != target && *mob != player && near(body)).map(|(mob, _, body, kind)| (mob, *body, kind.0.clone())).collect();
    for (mob, body, kind) in shoved {
        let (dx, dz) = (body.at[0] - at[0], body.at[2] - at[2]);
        let apart = dx.hypot(dz);
        let away = if apart > 1e-4 { [dx / apart, dz / apart] } else { [0.0; 2] };
        let push = mace::shove(away, apart);
        let kept = 1.0 - sapling_data::mob_traits(&kind).resists;
        let motion = [body.motion[0], if body.on_ground { -RESTING } else { body.motion[1] }, body.motion[2]];
        if kept > 0.0
            && let Some(mut thrown) = world.get_mut::<Body>(mob)
        {
            thrown.throw(std::array::from_fn(|axis| motion[axis] + push[axis] * kept));
        }
    }
}
