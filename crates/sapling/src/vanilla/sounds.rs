//! `/playsound`, `/stopsound` and `/particle`, as BDS answered them and sent their packets
//! (research/captures/bds-messaging.txt). Names are not checked: BDS played `no.such.sound`.

use sapling_api::command::{Coords, Selector};
use sapling_api::{Command, Output, Sender, ShardCtx, Sound};

use super::{no_target, players};

/// A player's eyes are this far over its feet; a sound with no place is played there.
const EYES: f32 = 1.62;
/// How far a sound of volume 1 or less carries; a louder one carries this much per unit.
const RANGE: f32 = 16.0;
/// How far from its hearer a sound out of range is moved to, played at `minimumVolume`.
const NEAR: f32 = 2.0;

/// Where and how loud `listener` hears a sound at `at`: `None` where it is out of range and no
/// minimum volume brings it near (the Java game's rule, which BDS's answers fit).
fn heard(listener: [f32; 3], at: [f32; 3], volume: f32, minimum: f32) -> Option<([f32; 3], f32)> {
    let range = if volume > 1.0 { volume * RANGE } else { RANGE };
    let off: [f32; 3] = std::array::from_fn(|axis| at[axis] - listener[axis]);
    let distance = off.iter().map(|d| d * d).sum::<f32>().sqrt();
    match distance <= range {
        true => Some((at, volume)),
        false if minimum > 0.0 => Some((std::array::from_fn(|axis| listener[axis] + off[axis] / distance * NEAR), minimum)),
        false => None,
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "playsound", description = "Plays a sound", permission = "operator")]
pub(crate) struct PlaySound {
    sound: String,
    player: Option<Selector>,
    position: Option<Coords>,
    volume: Option<f32>,
    pitch: Option<f32>,
    minimum_volume: Option<f32>,
}

impl Command for PlaySound {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, self.player.as_ref());
        if found.is_empty() {
            return no_target(out);
        }
        let at = self.position.map(|position| position.from(ctx.origin(sender)));
        let (volume, minimum) = (self.volume.unwrap_or(1.0), self.minimum_volume.unwrap_or(0.0));
        let mut played = 0;
        for player in &found {
            let Some(feet) = ctx.position(player.id).map(|position| position.feet) else { continue };
            let eyes = [feet[0], feet[1] + EYES, feet[2]];
            let Some((position, volume)) = heard(eyes, at.unwrap_or(eyes), volume, minimum) else { continue };
            ctx.play_sound(player.id, &Sound::new(self.sound.clone(), position).volume(volume).pitch(self.pitch.unwrap_or(1.0)));
            played += 1;
        }
        // TODO: which answer BDS gives when some targets hear it and some do not was not drilled.
        match (played, found.as_slice()) {
            (0, _) => out.translate("commands.playsound.failed", Vec::new(), false),
            (_, [one]) => out.translate("commands.playsound.success.single", vec![self.sound, one.name.clone()], true),
            _ => out.translate("commands.playsound.success.multiple", vec![self.sound, found.len().to_string()], true),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "stopsound", description = "Stops a sound", permission = "operator")]
pub(crate) struct StopSound {
    player: Selector,
    sound: Option<String>,
}

impl Command for StopSound {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, Some(&self.player));
        if found.is_empty() {
            return no_target(out);
        }
        found.iter().for_each(|player| ctx.stop_sound(player.id, self.sound.as_deref()));
        match self.sound {
            Some(sound) => out.translate("commands.stopsound.success", vec![sound], true),
            None => out.translate("commands.stopsound.success.all", Vec::new(), true),
        }
    }
}

/// `/particle <effect> [position]`: shown to whoever sees the place.
#[derive(Command)]
#[command(crate = "crate", name = "particle", description = "Creates a particle emitter", permission = "operator")]
pub(crate) struct Particle {
    effect: String,
    position: Option<Coords>,
}

impl Command for Particle {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        ctx.spawn_particle_in_world(&self.effect, self.position.map_or(here, |position| position.from(here)));
        out.translate("commands.spawnParticleEmitter.success", vec![self.effect], true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_out_of_range_is_brought_near_or_not_heard() {
        let eyes = [0.5, -18.38, 0.5];
        assert_eq!(heard(eyes, [16.5, -18.38, 0.5], 1.0, 0.0), Some(([16.5, -18.38, 0.5], 1.0)));
        assert_eq!(heard(eyes, [100.5, -20.0, 0.5], 1.0, 0.0), None, "BDS: commands.playsound.failed");
        assert_eq!(heard(eyes, [30.5, -18.38, 0.5], 2.0, 0.0).map(|(_, volume)| volume), Some(2.0), "volume 2 carries 32");
        let (near, volume) = heard(eyes, [100.5, -20.0, 0.5], 1.0, 0.5).unwrap();
        assert_eq!((near.map(|axis| (axis * 8.0) as i32), volume), ([19, -147, 4], 0.5), "where BDS put it");
    }
}
