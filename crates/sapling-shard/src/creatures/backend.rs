//! The entity side of the handlers' context.

use sapling_api::{Effect, EntityBackend, EntityId, EntityInfo};

use crate::components::NetId;
use crate::handlers::Ctx;
use crate::{entities, orbs, tnt, worlds};

const ORB: &str = "minecraft:xp_orb";

impl EntityBackend for Ctx<'_> {
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
}
