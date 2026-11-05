//! Amethyst blocks beside a sensor that goes active resonate: LevelEvent 1067 at each one's corner,
//! then `resonate_<frequency>` posted from it, which other sensors hear as that frequency (S8;
//! research/captures/bds-uureson.txt: a teleport's 14 passed on to a sensor 7 off the amethyst).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::{GameEvent, IDLE, PHASE, is_sensor, name_at, number, send};

const AMETHYST: &str = "minecraft:amethyst_block";
const RESONATE: LevelEventEvent = LevelEventEvent::Unknown(1067);

/// The amethyst blocks beside the sensor at `pos`, in BDS's face order.
pub(super) fn beside(world: &World, layer: Entity, pos: BlockPos) -> Vec<BlockPos> {
    Face::ALL.into_iter().map(|face| face.neighbour(pos)).filter(|at| name_at(world, layer, *at).is_some_and(|(name, ..)| name == AMETHYST)).collect()
}

/// The amethysts beside each idle sensor of `soon`, which goes active next tick, are shown
/// resonating now: BDS played 1067 a tick before the sensor's phase change (bds-uureson.txt).
pub(super) fn ring(world: &mut World, soon: &[(Entity, BlockPos)]) {
    for &(layer, sensor) in soon {
        let idle = name_at(world, layer, sensor).is_some_and(|(name, properties, _)| is_sensor(name) && number(properties, PHASE).unwrap_or(IDLE) == IDLE);
        for pos in if idle { beside(world, layer, sensor) } else { Vec::new() } {
            let [x, y, z] = pos.map(|c| c as f32);
            let event = sapling_net::encode(&LevelEvent { event: RESONATE, position: Vec3f { x, y, z }, data: 500 });
            send::to_viewers(world, layer, pos, &[event]);
        }
    }
}

/// Posts each of `amethysts`' resonance of `frequency`.
pub(super) fn post(world: &mut World, layer: Entity, amethysts: &[BlockPos], frequency: u8, by_player: bool) {
    for &pos in amethysts {
        super::vibrate_at(world, layer, pos, GameEvent::Resonate(frequency), by_player);
    }
}
