//! A trial spawner's wave: the players it takes on (`_tryRegisterNewPlayers`), the mobs it spawns
//! for them (`_spawnMob`) and keeps count of by `UniqueID`.

use acacia_proto::nbt::Value;
use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, SpawnReason};

use super::super::eject::{event, sound};
use super::Trial;
use super::super::{Near, centre, dice, distance, in_sight};
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::entities;
use crate::tags::UniqueId;

/// Players are looked for once in this many ticks, at a tick that differs by place.
const SCAN: i64 = 20;
/// A mob further than this from its spawner's corner no longer counts as its.
const STRAYED: f32 = 47.0;
/// The next spawn waits at least this long after a player is taken on.
const AFTER_DETECTION: i64 = 40;

/// Whether this tick is one the spawner at `pos` looks for players in (BDS's hash of the place).
pub(super) fn scans(pos: BlockPos, now: i64) -> bool {
    let [x, y, z] = pos.map(i64::from);
    let h = (z.wrapping_mul(0x6eb_fff5)) ^ (x.wrapping_mul(0x2f_c20f)) ^ y;
    h.wrapping_mul(h).wrapping_mul(0x285_b825).wrapping_add(h.wrapping_mul(11)).wrapping_add(now).rem_euclid(SCAN) == 0
}

/// The survival players within `range` of the spawner's corner it may take on: any, once it has
/// one; one it sees from its middle before that.
fn eligible(world: &World, layer: Entity, pos: BlockPos, near: &[Near], range: f32, first: bool) -> Vec<i64> {
    let corner = pos.map(|c| c as f32);
    near.iter()
        .filter(|player| crate::vitals::is_mortal(world, player.entity) && distance(player.eyes(), corner) <= range)
        .filter(|player| !first || in_sight(world, layer, centre(pos), player.eyes()))
        .map(|player| player.unique_id)
        .collect()
}

/// Takes on the players newly in range, telling it; the next spawn waits a little. Returns
/// whether any was taken on.
pub(super) fn register(world: &mut World, layer: Entity, pos: BlockPos, near: &[Near], trial: &mut Trial, now: i64) -> bool {
    let added: Vec<i64> = eligible(world, layer, pos, near, trial.range, trial.registered.is_empty()).into_iter().filter(|id| !trial.registered.contains(id)).collect();
    if added.is_empty() {
        return false;
    }
    trial.registered.extend(added);
    // BDS's charged detection, 3615, which the wire's names give to another event.
    let detected = if trial.ominous { LevelEventEvent::from_raw(3615) } else { LevelEventEvent::ParticleTrialSpawnerDetection };
    event(world, layer, pos, detected, centre(pos), trial.registered.len() as i32);
    trial.next_spawn = trial.next_spawn.max(now + AFTER_DETECTION);
    true
}

/// The spawner's mobs that are still its: alive, here and near.
pub(super) fn alive(world: &mut World, layer: Entity, pos: BlockPos, mobs: &[i64]) -> Vec<i64> {
    let corner = pos.map(|c| c as f32);
    let mut found = world.query::<(Entity, &UniqueId, &InLayer, &Body)>();
    let near: Vec<(Entity, i64)> = found.iter(world).filter(|(_, id, in_layer, body)| mobs.contains(&id.0) && in_layer.0 == layer && distance(body.at, corner) <= STRAYED).map(|(entity, id, ..)| (entity, id.0)).collect();
    mobs.iter().copied().filter(|id| near.iter().any(|(entity, of)| of == id && crate::creatures::is_alive(world, *entity))).collect()
}

/// One mob of `kind` somewhere within `range` of the spawner that it sees, kept for good; its
/// `UniqueID`, or `None` where the spot drawn would not do.
pub(super) fn spawn(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, range: i32, ominous: bool) -> Option<i64> {
    let at = {
        let mut dice = dice(world);
        let x = pos[0] as f32 + 0.5 + range as f32 * (dice.fraction() - dice.fraction());
        let y = pos[1] + dice.roll(3) as i32 - 1;
        let z = pos[2] as f32 + 0.5 + range as f32 * (dice.fraction() - dice.fraction());
        [x, y as f32, z]
    };
    let cell = at.map(|c| c.floor() as i32);
    let off: i32 = (0..3).map(|axis| (cell[axis] - pos[axis]).abs()).sum();
    let level = &world.get::<Layer>(layer)?.world;
    let solid = level.registry().get(level.block(cell[0], cell[1], cell[2])).is_none_or(|state| state.is_solid());
    if solid || off > range || !in_sight(world, layer, centre(pos), at) {
        return None;
    }
    let id = crate::creatures::spawn(world, layer, kind, at, SpawnReason::Spawner)?;
    let unique_id = world.get::<UniqueId>(entities::find(world, id)?)?.0;
    sound(world, layer, pos, "trial_spawner.spawn_mob", at, -1);
    event(world, layer, pos, LevelEventEvent::ParticleTrialSpawnerSpawning, at, ominous.into());
    event(world, layer, pos, LevelEventEvent::ParticleTrialSpawnerSpawning, centre(pos), ominous.into());
    Some(unique_id)
}

/// `{uuid: id}` for each id, as BDS keeps `registered_players` and `current_mobs`.
pub(super) fn to_list(ids: &[i64]) -> Value {
    super::super::fields::compounds(ids.iter().map(|id| Value::Compound(vec![("uuid".into(), Value::Long(*id))])).collect())
}

pub(super) fn from_list(items: &[Value]) -> Vec<i64> {
    items.iter().filter_map(|item| if let Some(Value::Long(id)) = item.get("uuid") { Some(*id) } else { None }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spawner_scans_once_in_twenty_ticks() {
        let ticks: Vec<i64> = (0..60).filter(|now| scans([3, 64, -7], *now)).collect();
        assert_eq!(ticks.len(), 3);
        assert_eq!(ticks[1] - ticks[0], 20);
        assert_eq!(from_list(&[Value::Compound(vec![("uuid".into(), Value::Long(-5))])]), [-5]);
    }
}
