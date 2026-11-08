//! The entity side of the handlers' context.

use sapling_api::{DamageCause, Effect, EntityBackend, EntityId, EntityInfo, Item, MobSlot, MobSlotRefusal, PlayerId};

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
        let (summoned, mob) = match made_alone {
            _ if shot.is_some() => (shot?, false),
            _ if crate::vehicles::is_vehicle(kind) => (crate::vehicles::summon(self.world, worlds::current(self.world), kind, feet, yaw)?, false),
            true => (self.spawn_entity(kind, feet)?, false),
            false => {
                let layer = worlds::current(self.world);
                let feet = super::summoned::placed(self.world, layer, kind, feet);
                let summoned = super::birth::summoned(self.world, layer, kind, feet, yaw, event)?;
                super::gaze::pitch_new(self.world, summoned, pitch);
                (summoned, true)
            }
        };
        // BDS showed no `EntitySpawn` for a summoned TNT, shot, vehicle, orb or bolt (bds-r1ench.txt and others).
        if let Some(player) = by.and_then(|by| self.entity(by)).filter(|_| mob) {
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
            // Its fuse is heard the next tick (bds-b1sculk.txt: `prime_fuse`'s particle a tick after its AddEntity).
            crate::blocks::vibrate_next_tick(self.world, layer, feet.map(|c| c.floor() as i32), crate::blocks::GameEvent::PrimeFuse, None);
            return Some(EntityId(self.world.get::<NetId>(primed)?.0));
        }
        if kind == crate::lightning::KIND {
            let bolt = crate::lightning::call_down(self.world, layer, feet, true)?;
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
        let from = entities::find(self.world, id).and_then(|entity| Some((self.world.get::<crate::body::Body>(entity)?.at, self.world.get::<crate::components::InLayer>(entity)?.0)));
        let moved = entities::teleport(self.world, id, feet);
        if let (true, Some((from, layer))) = (moved, from) {
            crate::blocks::teleported(self.world, layer, from, feet, false);
        }
        moved
    }

    fn name_entity(&mut self, id: EntityId, name: &str) -> bool {
        entities::find(self.world, id).is_some_and(|mob| super::husbandry::give_name(self.world, mob, name.to_owned()))
    }

    fn fire_entity_event(&mut self, id: EntityId, event: &str) -> bool {
        let Some(kind) = entities::describe(self.world, id).map(|entity| entity.kind) else { return false };
        match entities::find(self.world, id).filter(|&mob| self.world.get::<super::Creature>(mob).is_some()) {
            Some(mob) => entities::told_after_answer(self.world, |world| super::evented::fire(world, mob, &kind, event)),
            None => sapling_data::has_event(&kind, event),
        }
    }

    fn fire_player_event(&mut self, player: PlayerId, event: &str) -> bool {
        let Some(player) = self.entity(player) else { return false };
        entities::told_after_answer(self.world, |world| super::evented::fire_on_player(world, player, event))
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
        let Some(mob) = entities::find(self.world, id) else { return false };
        let player = match by {
            Some(sapling_api::Attacker::Player(player)) => self.entity(player.id),
            _ => None,
        };
        super::sourced::wound(self.world, mob, amount, cause, by, player)
    }

    fn hurt_entity_from(&mut self, id: EntityId, amount: f32, cause: DamageCause, source: &sapling_api::Target) -> bool {
        let Some(mob) = entities::find(self.world, id) else { return false };
        let source = match source {
            sapling_api::Target::Player(player) => self.entity(player.id),
            sapling_api::Target::Entity(id) => entities::find(self.world, *id),
            _ => None,
        };
        match source {
            Some(source) => super::sourced::hurt(self.world, mob, amount, cause, source),
            None => super::sourced::wound(self.world, mob, amount, cause, None, None),
        }
    }

    fn families(&self, kind: &str) -> Vec<String> {
        sapling_data::mob_families(kind).into_iter().map(str::to_owned).collect()
    }

    fn entity_families(&self, id: EntityId, kind: &str) -> Vec<String> {
        let grouped = entities::find(self.world, id).and_then(|mob| super::villagers::families::of(self.world, mob));
        grouped.map_or_else(|| self.families(kind), |families| families.into_iter().map(str::to_owned).collect())
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
            MobSlot::Inventory(at) => match crate::vehicles::put_goods(self.world, mob, at, stack.clone()) {
                Some(put) => put,
                None => super::villagers::put_in_pantry(self.world, mob, at, stack).unwrap_or(Err(MobSlotRefusal::NoSlot)),
            },
            _ => Err(MobSlotRefusal::NoSlot),
        }
    }
}
