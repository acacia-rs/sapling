//! The entity side of the handlers' context.

use sapling_api::{DamageCause, Effect, EntityBackend, EntityId, EntityInfo, Item, MobSlot, MobSlotRefusal, PlayerId};

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

    fn summon_as(&mut self, by: Option<PlayerId>, kind: &str, feet: [f32; 3], yaw: f32, pitch: f32, event: Option<&str>) -> Option<EntityId> {
        let made_alone = [ORB, crate::dragon::CRYSTAL, tnt::KIND, crate::lightning::KIND].contains(&kind);
        // BDS summoned no zombie on peaceful (research/captures/bds-server.txt).
        let peaceful = self.world.resource::<crate::components::Settings>().0.difficulty == sapling_api::Difficulty::Peaceful;
        if peaceful && crate::spawning::removed_in_peaceful(kind) {
            return None;
        }
        let shot = crate::shots::summon(self.world, worlds::current(self.world), kind, feet).and_then(|shot| self.world.get::<NetId>(shot)).map(|id| EntityId(id.0));
        let summoned = match made_alone {
            _ if shot.is_some() => shot?,
            _ if crate::vehicles::is_vehicle(kind) => crate::vehicles::summon(self.world, worlds::current(self.world), kind, feet, yaw)?,
            true => self.spawn_entity(kind, feet)?,
            false => {
                let summoned = super::spawn_as(self.world, worlds::current(self.world), kind, feet, sapling_api::SpawnReason::Command, yaw, event)?;
                super::gaze::pitch_new(self.world, summoned, pitch);
                summoned
            }
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
        entities::teleport(self.world, id, feet)
    }

    fn name_entity(&mut self, id: EntityId, name: &str) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::husbandry::give_name(self.world, mob, name.to_owned()))
    }

    fn turn_entity(&mut self, id: EntityId, yaw: f32) -> bool {
        let Some(entity) = entities::find(self.world, id) else { return false };
        super::gaze::turn(self.world, entity, yaw);
        true
    }

    fn move_entity_into(&mut self, id: EntityId, dimension: i32, feet: [f32; 3]) -> bool {
        let layer = entities::find(self.world, id).and_then(|entity| self.world.get::<InLayer>(entity)).map(|in_layer| in_layer.0);
        let Some(to) = layer.and_then(|layer| crate::dimensions::of(self.world, layer, dimension)) else { return false };
        entities::teleport_into(self.world, id, to, feet)
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

    fn give_entity_effect_shown(&mut self, id: EntityId, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::give_shown(self.world, mob, effect, ticks, amplifier, particles))
    }

    fn remove_entity_effect(&mut self, id: EntityId, effect: Effect) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::take(self.world, mob, Some(effect)))
    }

    fn clear_entity_effects(&mut self, id: EntityId) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::afflictions::take(self.world, mob, None))
    }

    fn set_entity_armour(&mut self, id: EntityId, piece: u8, item: Option<&Item>) -> bool {
        let Some(mob) = entities::find(self.world, id) else { return false };
        let level = self.world.get::<InLayer>(mob).and_then(|layer| self.world.get::<crate::components::Layer>(layer.0));
        let stack = item.and_then(|item| crate::inventory::stored(item, level?.world.registry()));
        (item.is_none() || stack.is_some()) && super::worn::put_on(self.world, mob, piece.into(), stack)
    }

    fn hurt_entity(&mut self, id: EntityId, amount: f32, cause: DamageCause) -> bool {
        self.hurt_entity_by(id, amount, cause, None)
    }

    /// A player's harm kills as its hit does: the mob is its slain, for experience and a raid's
    /// heroes (research/captures/bds-raidhero.txt).
    fn hurt_entity_by(&mut self, id: EntityId, amount: f32, cause: DamageCause, by: Option<&sapling_api::Attacker>) -> bool {
        let Some(mob) = entities::find(self.world, id).filter(|mob| self.world.get::<super::Dying>(*mob).is_none()) else { return false };
        let (Some(layer), Some(health)) = (self.world.get::<InLayer>(mob).map(|layer| layer.0), self.world.get::<Health>(mob).copied()) else { return false };
        let player = match by {
            Some(sapling_api::Attacker::Player(player)) => self.entity(player.id),
            _ => None,
        };
        let before = health.left();
        // BDS's `/damage` of 8 cost a zombie in a diamond chestplate 5.824 (research/captures/bds-boosts.txt).
        let amount = if cause.is_stopped_by_armour() { health.shielded(amount) } else { amount };
        if let Some(sapling_api::Attacker::Mob { kind }) = by
            && super::villagers::infect(self.world, mob, layer, kind, amount)
        {
            return true;
        }
        super::harm::wound_by(self.world, mob, layer, amount, cause, player);
        self.world.get::<super::Dying>(mob).is_some() || self.world.get::<Health>(mob).is_some_and(|health| health.left() < before)
    }

    fn families(&self, kind: &str) -> Vec<String> {
        sapling_data::mob_families(kind).into_iter().map(str::to_owned).collect()
    }

    fn set_mob_item(&mut self, id: EntityId, slot: MobSlot, item: Option<&sapling_api::Item>) -> Result<(), MobSlotRefusal> {
        let mob = entities::find(self.world, id).ok_or(MobSlotRefusal::NoSlot)?;
        let stack = match item {
            Some(item) => {
                let layer = self.world.get::<crate::components::Layer>(worlds::current(self.world)).ok_or(MobSlotRefusal::NoSlot)?;
                Some(crate::inventory::stored(item, layer.world.registry()).ok_or(MobSlotRefusal::NoSlot)?)
            }
            None => None,
        };
        match slot {
            MobSlot::Armor(_) | MobSlot::MainHand | MobSlot::OffHand => super::command_gear::set(self.world, mob, slot, stack),
            MobSlot::Body => crate::riding::put_body(self.world, mob, stack).then_some(()).ok_or(MobSlotRefusal::NoSlot),
            _ if crate::riding::is_mount(self.world, mob) => crate::riding::set_mount_item(self.world, mob, slot, stack),
            MobSlot::Inventory(at) => super::villagers::put_in_pantry(self.world, mob, at, stack).unwrap_or(Err(MobSlotRefusal::NoSlot)),
            _ => Err(MobSlotRefusal::NoSlot),
        }
    }
}
