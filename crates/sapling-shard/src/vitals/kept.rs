//! A player's health, hunger, breath and burning between visits: kept with its items
//! (inventory/store.rs) as BDS's attributes and its `Air` and `Fire` shorts.
// TODO: one who left dead comes back whole. Effects: effects.rs.

use acacia_proto::packets::{SetEntityData, UpdateAttributes};

use super::{AIR, MAX, Vitals, hunger::Hunger};
use crate::creatures::Health;

/// What storage keeps of a player's vitals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Kept {
    /// Health, hunger, saturation and exhaustion.
    pub vitals: [f32; 4],
    pub air: i16,
    pub fire: i16,
}

impl Vitals {
    pub(crate) fn kept(&self) -> Kept {
        let [hunger, saturation, exhaustion] = self.hunger.kept();
        let fire = i16::try_from(self.health.burning()).unwrap_or(i16::MAX);
        Kept { vitals: [self.health.left(), hunger, saturation, exhaustion], air: self.air, fire }
    }

    pub(crate) fn is_unsaved(&self) -> bool {
        self.saved != self.kept()
    }

    pub(crate) fn mark_saved(&mut self) {
        self.saved = self.kept();
    }

    /// Takes up what storage kept of a living player; a record without breath or burning has
    /// full breath and is not on fire.
    pub(crate) fn restore(&mut self, [health, hunger, saturation, exhaustion]: [f32; 4], air: Option<i16>, fire: Option<i16>) {
        if health > 0.0 {
            let burning = fire.map_or(0, |fire| fire.max(0) as u16);
            (self.health, self.hunger) = (Health::new(MAX, Some(health), burning), Hunger::restored([hunger, saturation, exhaustion]));
            self.air = air.map_or(AIR, |air| air.min(AIR));
            self.mark_saved();
        }
    }

    /// What tells a joining player `id` the health and hunger it came back with, if they are not
    /// those every player starts with.
    pub(crate) fn greeting(&mut self, id: u64, tick: u64) -> Vec<UpdateAttributes> {
        if self.kept().vitals == Vitals::default().kept().vitals {
            return Vec::new();
        }
        self.told = self.health.left();
        vec![sapling_data::mob_health_update(id, self.told, MAX), self.hunger.told(id, tick)]
    }

    /// What tells a joining player `id` the breath it came back with, if it is short of breath.
    pub(crate) fn breath_greeting(&self, id: u64) -> Option<SetEntityData> {
        (self.air != AIR).then(|| sapling_data::mob_air(id, self.air))
    }
}
