//! A player's health and hunger between visits: kept with its items (inventory/store.rs).
// TODO: breath and fire are not kept; one who left dead comes back whole. Effects: effects.rs.

use acacia_proto::packets::UpdateAttributes;

use super::{MAX, Vitals, hunger::Hunger};
use crate::creatures::Health;

impl Vitals {
    /// What storage keeps: health, hunger, saturation and exhaustion.
    pub(crate) fn kept(&self) -> [f32; 4] {
        let [hunger, saturation, exhaustion] = self.hunger.kept();
        [self.health.left(), hunger, saturation, exhaustion]
    }

    pub(crate) fn is_unsaved(&self) -> bool {
        self.saved != self.kept()
    }

    pub(crate) fn mark_saved(&mut self) {
        self.saved = self.kept();
    }

    /// Takes up what storage kept of a living player.
    pub(crate) fn restore(&mut self, [health, hunger, saturation, exhaustion]: [f32; 4]) {
        if health > 0.0 {
            (self.health, self.hunger) = (Health::new(MAX, Some(health), 0), Hunger::restored([hunger, saturation, exhaustion]));
            self.mark_saved();
        }
    }

    /// What tells a joining player `id` the health and hunger it came back with, if they are not
    /// those every player starts with.
    pub(crate) fn greeting(&mut self, id: u64, tick: u64) -> Vec<UpdateAttributes> {
        if self.kept() == Vitals::default().kept() {
            return Vec::new();
        }
        self.told = self.health.left();
        vec![sapling_data::mob_health_update(id, self.told, MAX), self.hunger.told(id, tick)]
    }
}
