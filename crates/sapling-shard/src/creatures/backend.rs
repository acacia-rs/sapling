//! The entity side of the handlers' context.

use sapling_api::{DamageCause, Effect, EntityBackend, EntityId, EntityInfo, PlayerId};

use super::health::Health;
use crate::components::{InLayer, NetId};
use crate::handlers::Ctx;
use crate::{entities, orbs, tnt, worlds};

const ORB: &str = "minecraft:xp_orb";

impl EntityBackend for Ctx<'_> {
    fn summon(&mut self, by: PlayerId, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
        let summoned = self.spawn_entity(kind, feet)?;
        if let Some(player) = self.entity(by) {
            super::summoned::show(self.world, player, kind);
        }
        Some(summoned)
    }

    fn summon_as(&mut self, by: Option<PlayerId>, kind: &str, feet: [f32; 3], yaw: f32, event: Option<&str>) -> Option<EntityId> {
        let made_alone = [ORB, crate::dragon::CRYSTAL, tnt::KIND, crate::lightning::KIND].contains(&kind);
        let summoned = match made_alone {
            _ if crate::vehicles::is_vehicle(kind) => crate::vehicles::summon(self.world, worlds::current(self.world), kind, feet, yaw)?,
            true => self.spawn_entity(kind, feet)?,
            false => super::spawn_as(self.world, worlds::current(self.world), kind, feet, sapling_api::SpawnReason::Command, yaw, event)?,
        };
        if let Some(player) = by.and_then(|by| self.entity(by)) {
            super::summoned::show(self.world, player, kind);
        }
        Some(summoned)
    }

    /// An orb is an orb, worth 1 and thrown as orbs are, as BDS's `/summon xp_orb` makes one.
    fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
        let layer = worlds::current(self.world);
        if kind == ORB {
            let orb = orbs::throw(self.world, layer, feet, 1)?;
            return Some(EntityId(self.world.get::<NetId>(orb)?.0));
        }
        if kind == crate::dragon::CRYSTAL {
            let crystal = crate::dragon::summon_crystal(self.world, layer, feet)?;
            return Some(EntityId(self.world.get::<NetId>(crystal)?.0));
        }
        if kind == tnt::KIND {
            let primed = tnt::prime(self.world, layer, feet, tnt::FUSE)?;
            return Some(EntityId(self.world.get::<NetId>(primed)?.0));
        }
        if kind == crate::lightning::KIND {
            let bolt = crate::lightning::strike(self.world, layer, feet, true)?;
            return Some(EntityId(self.world.get::<NetId>(bolt)?.0));
        }
        if crate::vehicles::is_vehicle(kind) {
            return crate::vehicles::summon(self.world, layer, kind, feet, 0.0);
        }
        super::spawn(self.world, layer, kind, feet, sapling_api::SpawnReason::Command)
    }

    fn entity(&self, id: EntityId) -> Option<EntityInfo> {
        entities::describe(self.world, id)
    }

    fn entities(&self) -> Vec<EntityInfo> {
        entities::listed(self.world, worlds::current(self.world))
    }

    fn move_entity(&mut self, id: EntityId, feet: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.at = feet)
    }

    fn set_velocity(&mut self, id: EntityId, velocity: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.motion = velocity)
    }

    fn remove_entity(&mut self, id: EntityId) -> bool {
        entities::remove(self.world, id)
    }

    fn kill_entity(&mut self, id: EntityId) -> bool {
        super::kill(self.world, id)
    }

    fn give_entity_effect(&mut self, id: EntityId, effect: Effect, ticks: u32, amplifier: u8) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::give(self.world, mob, effect, ticks, amplifier))
    }

    fn remove_entity_effect(&mut self, id: EntityId, effect: Effect) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::take(self.world, mob, Some(effect)))
    }

    fn clear_entity_effects(&mut self, id: EntityId) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::take(self.world, mob, None))
    }

    fn hurt_entity(&mut self, id: EntityId, amount: f32, cause: DamageCause) -> bool {
        let Some(mob) = entities::find(self.world, id).filter(|mob| self.world.get::<super::Dying>(*mob).is_none()) else { return false };
        let (Some(layer), Some(before)) = (self.world.get::<InLayer>(mob).map(|layer| layer.0), self.world.get::<Health>(mob).map(Health::left)) else { return false };
        super::harm::wound(self.world, mob, layer, amount, cause);
        self.world.get::<super::Dying>(mob).is_some() || self.world.get::<Health>(mob).is_some_and(|health| health.left() < before)
    }

    fn families(&self, kind: &str) -> Vec<String> {
        sapling_data::mob_families(kind).into_iter().map(str::to_owned).collect()
    }
}
