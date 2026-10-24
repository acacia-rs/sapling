//! Sounds and particles by name for plugins: PlaySound, StopSound and SpawnParticleEffect as
//! BDS's `/playsound`, `/stopsound` and `/particle` sent them (research/captures/bds-messaging.txt).

use acacia_proto::packets::{PlaySound, SpawnParticleEffect, StopSound};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{PlayerId, Sound, SoundBackend};

use crate::components::{InLayer, Layer};
use crate::handlers::Ctx;
use crate::{view, worlds};

/// PlaySound carries eighths of a block, cut toward zero: BDS sent y -18.375 as -147.
const EIGHTHS: f32 = 8.0;
/// A particle tied to no entity.
const NO_ENTITY: i64 = -1;

fn play(sound: &Sound) -> PlaySound {
    let [x, y, z] = sound.position.map(|axis| (axis * EIGHTHS) as i32);
    let coordinates = BlockCoordinates { x, y, z };
    PlaySound { name: sound.name.clone(), coordinates, volume: sound.volume, pitch: sound.pitch, loop_count: 0, bypass_listener_range_check: false, handle: None, playback_position_seconds: None }
}

fn particle(dimension: i32, name: &str, [x, y, z]: [f32; 3]) -> Bytes {
    let packet = SpawnParticleEffect { dimension: dimension as u8, entity_id: NO_ENTITY, position: Vec3f { x, y, z }, particle_name: name.to_owned(), molang_variables: None };
    sapling_net::encode(&packet)
}

fn block(position: [f32; 3]) -> [i32; 3] {
    position.map(|axis| axis.floor() as i32)
}

impl Ctx<'_> {
    fn send(&mut self, player: PlayerId, packet: Bytes) {
        if let Some(mut out) = self.out(player) {
            out.0.push(packet);
        }
    }

    /// The world a player is in, or the one in scope.
    fn layer_for(&self, player: Option<PlayerId>) -> Option<Entity> {
        match player {
            Some(player) => self.world.get::<InLayer>(self.entity(player)?).map(|in_layer| in_layer.0),
            None => Some(worlds::current(self.world)),
        }
    }
}

impl SoundBackend for Ctx<'_> {
    fn play_sound(&mut self, to: Option<PlayerId>, sound: &Sound) {
        let packet = sapling_net::encode(&play(sound));
        match to {
            Some(player) => self.send(player, packet),
            None => view::to_viewers(self.world, worlds::current(self.world), block(sound.position), &[packet]),
        }
    }

    fn stop_sound(&mut self, player: PlayerId, name: Option<&str>) {
        let packet = StopSound { name: name.unwrap_or_default().to_owned(), stop_all: name.is_none(), stop_music_legacy: false };
        self.send(player, sapling_net::encode(&packet));
    }

    fn spawn_particle(&mut self, to: Option<PlayerId>, name: &str, position: [f32; 3]) {
        let Some(layer) = self.layer_for(to) else { return };
        let Some(dimension) = self.world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return };
        let packet = particle(dimension, name, position);
        match to {
            Some(player) => self.send(player, packet),
            None => view::to_viewers(self.world, layer, block(position), &[packet]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_is_placed_in_eighths_cut_toward_zero() {
        let sound = Sound::new("random.orb", [0.5, -18.38, 2.4997]).volume(0.5);
        let packet = play(&sound);
        assert_eq!([packet.coordinates.x, packet.coordinates.y, packet.coordinates.z], [4, -147, 19], "as BDS sent them");
        assert_eq!((packet.volume, packet.pitch, packet.loop_count), (0.5, 1.0, 0));
    }
}
