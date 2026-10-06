//! A player's experience: its level and how far into the next one, as BDS's
//! `Player::addExperience` keeps them (docs/research/experience.md, section 4). The client is told
//! both as attributes; they are kept with the player's items (inventory/store.rs).

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::components::{NetId, Outgoing, Position, Tick};
use crate::inventory::Inventory;

/// The level-up sound plays on every fifth level, at most this often.
const SOUND_EVERY: u64 = 100;
const SOUND: &str = "levelup";

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Experience {
    pub level: i32,
    /// 0 to 1.
    pub progress: f32,
    /// When the level-up sound last played.
    sounded: Option<u64>,
}

/// Experience from `level` to the next (BDS `Player::getXpNeededForNextLevel`, as Java's).
fn needed(level: i32) -> f32 {
    (match level {
        0..15 => 2 * level + 7,
        15..30 => 5 * level - 38,
        _ => 9 * level - 158,
    }) as f32
}

impl Experience {
    pub fn new(level: i32, progress: f32) -> Self {
        Self { level: level.max(0), progress: progress.clamp(0.0, 1.0), sounded: None }
    }

    /// Adds `points`; true when it reached a fifth level.
    fn add(&mut self, points: i32) -> bool {
        let mut fifth = false;
        self.progress += points as f32 / needed(self.level);
        while self.progress >= 1.0 {
            let before = needed(self.level);
            self.level += 1;
            self.progress = (self.progress - 1.0) * before / needed(self.level);
            fifth |= self.level % 5 == 0;
        }
        fifth
    }
}

/// What the client is told of a player's experience: the level too when `with_level`.
pub(crate) fn attributes(id: &NetId, experience: &Experience, with_level: bool) -> acacia_proto::packets::UpdateAttributes {
    sapling_data::experience_attributes(id.0, Some(experience.level).filter(|_| with_level), experience.progress)
}

/// Gives a player `points` of experience, which it is shown; a fifth level sounds, once in
/// [`SOUND_EVERY`] ticks.
pub(crate) fn give(world: &mut World, player: Entity, points: i32) {
    let tick = world.resource::<Tick>().0;
    let mut query = world.query::<(&NetId, &Position, &mut Inventory, &mut Outgoing)>();
    let Ok((id, position, mut inventory, mut out)) = query.get_mut(world, player) else { return };
    let experience = &mut inventory.experience;
    let level = experience.level;
    let sounds = experience.add(points) && experience.sounded.is_none_or(|last| tick >= last + SOUND_EVERY);
    out.push(&attributes(id, experience, experience.level != level));
    if sounds {
        experience.sounded = Some(tick);
        // A volume: louder up to level 30 (BDS `Player::addLevels`).
        let volume = (experience.level as f32 / 30.0).min(1.0) * 0.75 * 0xFF_FFFF as f32;
        let [x, y, z] = position.feet;
        let at = Vec3f { x, y: y + 0.9, z };
        let sound = LevelSoundEvent { sound_id: SOUND.into(), position: at, extra_data: volume as i32, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
        out.push(&sound);
    }
    inventory.unsaved = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_take_what_the_game_says() {
        assert_eq!([needed(0), needed(14), needed(15), needed(29), needed(30)], [7.0, 35.0, 37.0, 107.0, 112.0]);
        let mut experience = Experience::default();
        assert!(!experience.add(10));
        assert_eq!((experience.level, (experience.progress * 9.0).round()), (1, 3.0), "7 to level 1, 3 of the next 9");
        let mut experience = Experience::default();
        assert!(experience.add(7 + 9 + 11 + 13 + 15 + 5), "level 5");
        assert_eq!((experience.level, (experience.progress * 17.0).round()), (5, 5.0), "and 5 of the next 17");
    }
}
