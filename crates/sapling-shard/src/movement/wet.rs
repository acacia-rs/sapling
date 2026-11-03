//! A player's `splash` going into water and its `ambient.underwater` sounds as its eyes go under
//! and come out, heard by it and its viewers as BDS played them (research/captures/bds-wetflags.txt,
//! docs/research/splash-sound.md "Players").

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use sapling_data::EYE_HEIGHT;
use sapling_world::World as Level;

use crate::body::{Body, InLiquid, Liquid, in_liquid, liquid_in, surface};

const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];
/// How much of the step across counts, and how loud speed makes it (as a mob's: splash.rs).
const ACROSS_WEIGHT: f32 = 0.2;
const LOUDNESS: f32 = 0.2;

/// What the last input found: the first only notes it, as a mob's first check (splash.rs).
#[derive(Default, Clone, Copy)]
pub(super) struct Wet {
    checked: bool,
    body: bool,
    eyes: bool,
}

fn sound(name: &str, id: u64, [x, y, z]: [f32; 3], extra_data: i32) -> LevelSoundEvent {
    LevelSoundEvent {
        sound_id: name.into(),
        position: Vec3f { x, y: y + PLAYER_SIZE[1] / 2.0, z },
        extra_data,
        entity_type: "minecraft:player".into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: id as i64,
        fire_at_position: None,
    }
}

/// The sounds of the player `id` that stepped by `delta` to `feet`, from the middle of its box.
pub(super) fn sounds(wet: &mut Wet, level: &Level, id: u64, feet: [f32; 3], delta: [f32; 3]) -> Vec<LevelSoundEvent> {
    let body = in_liquid(level, &Body::new(feet, [0.0; 3], PLAYER_SIZE), InLiquid::Sinks, Liquid::Water);
    let eye = feet[1] + EYE_HEIGHT;
    let cell = [feet[0].floor() as i32, eye.floor() as i32, feet[2].floor() as i32];
    let eyes = liquid_in(level, cell, Liquid::Water).is_some_and(|found| eye < surface(cell[1], found));
    let was = std::mem::replace(wet, Wet { checked: true, body, eyes });
    if !was.checked {
        return Vec::new();
    }
    let mut sounds = Vec::new();
    if body && !was.body {
        let [dx, dy, dz] = delta;
        let volume = ((ACROSS_WEIGHT * dx * dx + dy * dy + ACROSS_WEIGHT * dz * dz).sqrt() * LOUDNESS).min(1.0);
        sounds.push(sound("splash", id, feet, (volume * 16_777_215.0) as i32));
    }
    if eyes != was.eyes {
        sounds.push(sound(if eyes { "ambient.underwater.enter" } else { "ambient.underwater.exit" }, id, feet, -1));
    }
    sounds
}
