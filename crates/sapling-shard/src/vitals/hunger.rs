//! A player's hunger: what mending costs it and what an empty one costs the player, as BDS was
//! seen to (docs/research/player-damage.md, "Mending" and "Starving").
// TODO: mining and being hurt tire nobody (not drilled).

use acacia_proto::packets::UpdateAttributes;
use acacia_proto::types::PlayerAttributesItem;
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use crate::components::Settings;

const FULL: f32 = 20.0;
const SATED: f32 = 5.0;
/// With this much hunger a wounded player mends.
const MENDS: f32 = 18.0;
/// Ticks between two mendings, and between two pangs of an empty hunger.
const EVERY: u32 = 80;
const MEND_TIRES: f32 = 6.0;
/// Exhaustion that takes one of the saturation, or of the hunger once that is gone.
const SPENT: f32 = 4.0;

/// Health an empty hunger leaves a player: 10 on easy and peaceful, 1 on normal, none on hard
/// (research/captures/bds-hurt2.txt).
pub(super) fn starved(world: &World) -> f32 {
    match world.resource::<Settings>().0.difficulty {
        Difficulty::Hard => 0.0,
        Difficulty::Normal => 1.0,
        Difficulty::Easy | Difficulty::Peaceful => 10.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Turn {
    Mend,
    Starve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Hunger {
    food: f32,
    saturation: f32,
    exhaustion: f32,
    /// Ticks since it began to mend or to starve, or since the last turn of that.
    waited: u32,
}

impl Default for Hunger {
    fn default() -> Self {
        Self { food: FULL, saturation: SATED, exhaustion: 0.0, waited: 0 }
    }
}

impl Hunger {
    /// A tick of a player, `wounded` or whole: what is due to it. A mending is paid for here, and
    /// 4 of exhaustion are spent a tick, in the tick they are there.
    pub(super) fn tick(&mut self, wounded: bool) -> Option<Turn> {
        let turn = if self.food >= MENDS && wounded {
            Some(Turn::Mend)
        } else if self.food <= 0.0 {
            Some(Turn::Starve)
        } else {
            None
        };
        self.waited = if turn.is_some() { self.waited + 1 } else { 0 };
        let due = turn.filter(|_| self.waited >= EVERY);
        if due.is_some() {
            self.waited = 0;
        }
        if due == Some(Turn::Mend) {
            self.exhaustion += MEND_TIRES;
        }
        if self.exhaustion >= SPENT {
            self.exhaustion -= SPENT;
            let from = if self.saturation > 0.0 { &mut self.saturation } else { &mut self.food };
            *from = (*from - 1.0).max(0.0);
        }
        due
    }

    /// Adds to the exhaustion: a jump, a run (vitals.rs).
    pub(super) fn tire(&mut self, by: f32) {
        self.exhaustion += by;
    }

    /// Puts the hunger back to `food`, a loss plugins refused (life_events.rs).
    pub(super) fn keep_food(&mut self, food: f32) {
        self.food = food;
    }

    pub(super) fn is_full(&self) -> bool {
        self.food >= FULL
    }

    /// A food eaten: the saturation it gives is kept up to the hunger.
    pub(super) fn feed(&mut self, hunger: f32, saturation: f32) {
        self.food = (self.food + hunger).min(FULL);
        self.saturation = (self.saturation + saturation).min(self.food);
    }

    /// Its hunger, saturation and exhaustion: what is told of it.
    /// Hunger, saturation and exhaustion, as storage keeps them.
    pub(super) fn kept(&self) -> [f32; 3] {
        [self.food, self.saturation, self.exhaustion]
    }

    pub(super) fn restored([food, saturation, exhaustion]: [f32; 3]) -> Self {
        Self { food, saturation, exhaustion, ..Self::default() }
    }

    pub(super) fn shown(&self) -> (f32, f32, f32) {
        (self.food, self.saturation, self.exhaustion)
    }

    /// What the player `id` and its viewers are told of it.
    pub(super) fn told(&self, id: u64, tick: u64) -> UpdateAttributes {
        let item = |name: &str, current: f32, default: f32| PlayerAttributesItem { min: 0.0, max: FULL, current, default_min: 0.0, default_max: FULL, default, name: format!("minecraft:player.{name}"), modifiers: Vec::new() };
        let attributes = vec![item("hunger", self.food, FULL), item("saturation", self.saturation, SATED), item("exhaustion", self.exhaustion, 0.0)];
        UpdateAttributes { runtime_entity_id: id, attributes, tick }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wounded_player_mends_each_four_seconds_until_its_hunger_is_under_18_as_on_bds() {
        // research/captures/bds-hunger.txt: hurt 14, left alone.
        let mut hunger = Hunger::default();
        let mut after = Vec::new();
        for tick in 1..=1000 {
            if hunger.tick(true) == Some(Turn::Mend) {
                after.push((tick, hunger.shown()));
            }
        }
        assert_eq!(after, [(80, (20.0, 4.0, 2.0)), (160, (20.0, 3.0, 4.0)), (240, (20.0, 1.0, 2.0)), (320, (20.0, 0.0, 4.0)), (400, (18.0, 0.0, 2.0)), (480, (17.0, 0.0, 4.0))]);
        assert_eq!(hunger.shown(), (16.0, 0.0, 0.0), "the 4 left over are spent a tick later");
        assert!((0..200).all(|_| Hunger::default().tick(false).is_none()), "a whole player is not mended");
    }

    #[test]
    fn an_empty_hunger_pangs_each_four_seconds() {
        let mut hunger = Hunger { food: 0.0, saturation: 0.0, exhaustion: 0.0, waited: 0 };
        let pangs: Vec<i32> = (1..=240).filter(|_| hunger.tick(true) == Some(Turn::Starve)).collect();
        assert_eq!(pangs, [80, 160, 240]);
        assert_eq!(hunger.told(7, 3).attributes.iter().map(|attribute| (attribute.name.as_str(), attribute.current, attribute.default)).collect::<Vec<_>>(), [
            ("minecraft:player.hunger", 0.0, 20.0),
            ("minecraft:player.saturation", 0.0, 5.0),
            ("minecraft:player.exhaustion", 0.0, 0.0),
        ]);
    }
}
