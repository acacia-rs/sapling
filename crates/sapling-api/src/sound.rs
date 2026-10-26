//! Sounds and particles by name, as `/playsound`, `/stopsound` and `/particle` send them: to one
//! player, or to whoever sees a place in the world in scope.

use crate::{PlayerId, ShardCtx};

/// A sound of the client's sound definitions (`random.orb`, `mob.pig.say`) at a point.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    pub name: String,
    pub position: [f32; 3],
    /// 1 is the sound as recorded; above 1 it carries further, not louder.
    pub volume: f32,
    /// 1 is the sound as recorded.
    pub pitch: f32,
}

impl Sound {
    pub fn new(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self { name: name.into(), position, volume: 1.0, pitch: 1.0 }
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch;
        self
    }
}

impl ShardCtx<'_> {
    /// Plays `sound` to one player, wherever they are.
    pub fn play_sound(&mut self, player: PlayerId, sound: &Sound) {
        self.backend.play_sound(Some(player), sound);
    }

    /// Plays `sound` to everyone who sees its place in the world in scope.
    pub fn play_sound_in_world(&mut self, sound: &Sound) {
        self.backend.play_sound(None, sound);
    }

    /// Stops the sound `name` a player hears, or every sound where `name` is `None`.
    pub fn stop_sound(&mut self, player: PlayerId, name: Option<&str>) {
        self.backend.stop_sound(player, name);
    }

    /// Shows one player the particle effect `name` (`minecraft:heart_particle`) at `position`.
    pub fn spawn_particle(&mut self, player: PlayerId, name: &str, position: [f32; 3]) {
        self.backend.spawn_particle(Some(player), name, position);
    }

    /// Shows the particle effect `name` to everyone who sees `position` in the world in scope.
    pub fn spawn_particle_in_world(&mut self, name: &str, position: [f32; 3]) {
        self.backend.spawn_particle(None, name, position);
    }
}

/// The sound and particle side of [`crate::Backend`]; `None` for a player is the world in scope.
pub trait SoundBackend {
    fn play_sound(&mut self, to: Option<PlayerId>, sound: &Sound);
    fn stop_sound(&mut self, player: PlayerId, name: Option<&str>);
    fn spawn_particle(&mut self, to: Option<PlayerId>, name: &str, position: [f32; 3]);
}

#[cfg(test)]
impl SoundBackend for crate::testing::Fake {
    fn play_sound(&mut self, to: Option<PlayerId>, sound: &Sound) {
        self.log.push(format!("sound {to:?} {} {:?} {} {}", sound.name, sound.position, sound.volume, sound.pitch));
    }
    fn stop_sound(&mut self, player: PlayerId, name: Option<&str>) {
        self.log.push(format!("stop sound {} {name:?}", player.0));
    }
    fn spawn_particle(&mut self, to: Option<PlayerId>, name: &str, position: [f32; 3]) {
        self.log.push(format!("particle {to:?} {name} {position:?}"));
    }
}
