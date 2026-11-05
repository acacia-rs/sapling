//! A hit on a player: asked of the world's rules and plugins, dressed in its armour and effects,
//! landed, flashed and told (docs/research/player-damage.md), and the plugins' `HealthBackend`.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{DamageCause, HealthBackend, PlayerId};
use sapling_data::EYE_HEIGHT;

use super::{Vitals, death, effects, frost, is_dead, is_mortal, layer_of, omens, struck, tell, told_health, totem};
use crate::components::{Layer, NetId, Outgoing, Position, Tick};
use crate::creatures::Cause;
use crate::handlers::Ctx;
use crate::movement::{self, Motion};

const RESISTS: f32 = 0.2;
/// What a helmet lets through of a hit on the head (research/captures/bds-worn.txt).
const HELMETED: f32 = 0.75;

/// The flash of a hit of `cause` that landed; a fall's names the player's eyes.
pub(super) fn flash(world: &mut World, player: Entity, cause: Cause) {
    let (Some(id), Some(position)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).copied()) else { return };
    let [x, y, z] = position.feet;
    let fire_at_position = (cause == Cause::Fall).then_some(Vec3f { x, y: y + EYE_HEIGHT, z });
    tell(world, player, &EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::HurtAnimation, data: cause.id(), fire_at_position });
}

/// The flash of a hit of `cause` the player's guard took whole, as BDS shows a guardian's magic
/// after its bite (research/captures/bds-guardianhits.txt).
pub(crate) fn flash_guarded(world: &mut World, player: Entity, cause: Cause) {
    if is_mortal(world, player) && !is_dead(world, player) {
        flash(world, player, cause);
    }
}

/// Tells the health and the burning where they changed, and lets a player with no health die of `cause`.
pub(super) fn settle(world: &mut World, player: Entity, cause: Cause) {
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    let (left, burning, ticks) = (vitals.health.left(), vitals.health.is_burning(), vitals.health.burning());
    let cushion = vitals.health.cushion();
    let padded = std::mem::replace(&mut vitals.padded, cushion) != cushion;
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    let tick = world.resource::<Tick>().0;
    // bds-t2eitems.txt: absorption hit down to nothing ends, told before the cushion.
    if padded && cushion == 0.0 {
        effects::end(world, player, sapling_api::Effect::Absorption);
    }
    if padded {
        tell(world, player, &effects::cushion(id, cushion, tick));
    }
    told_health::tell_changed(world, player);
    let lit = burning && world.get::<Motion>(player).is_some_and(|motion| !motion.pose.burning);
    let burning = burning && !(lit && !crate::hand_events::may_burn(world, player, ticks));
    if lit && !burning && let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.burn(None, true);
    }
    if let Some(mut motion) = world.get_mut::<Motion>(player).filter(|motion| motion.pose.burning != burning) {
        motion.pose.burning = burning;
        let shown = movement::pose_packet(id, motion.pose, tick);
        // Viewers are told by `movement::announce`.
        world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&shown);
    }
    if left <= 0.0 {
        death::die(world, player, cause);
    }
}

/// Has the player's health know its armour, what its enchantments and effects ward off, and its
/// movement what its armour changes: a level of resistance takes a fifth off every hit (bds-more.txt).
pub(super) fn dress(world: &mut World, player: Entity) {
    let armour = world.get::<crate::inventory::Inventory>(player).map_or([0.0; 2], |inventory| inventory.armour());
    let (warding, leather) = (crate::enchants::warding(world, player), frost::leather(world, player));
    if let Some(mut simulated) = world.get_mut::<crate::simulation::Simulated>(player) {
        simulated.equip(&warding, leather[frost::FEET]);
    }
    let resisted = effects::level(world, player, sapling_api::Effect::Resistance).map_or(0.0, |level| RESISTS * (f32::from(level) + 1.0));
    let fireproof = effects::level(world, player, sapling_api::Effect::FireResistance).is_some();
    // A level of jump boost forgives a block of a fall, slow falling all of it (bds-fx.txt).
    let floats = effects::level(world, player, sapling_api::Effect::SlowFalling).is_some();
    let springs = if floats { f32::INFINITY } else { effects::level(world, player, sapling_api::Effect::JumpBoost).map_or(0.0, |level| f32::from(level) + 1.0) };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.wear(armour, warding);
        vitals.health.ward((1.0 - resisted).max(0.0), fireproof);
        vitals.health.spring(springs);
    }
}

/// [`land`]s a hit once the world's rules and plugins let it; false where it took nothing.
pub(crate) fn hurt(world: &mut World, player: Entity, amount: f32, cause: Cause) -> bool {
    let spared = layer_of(world, player).and_then(|layer| world.get::<Layer>(layer)).is_some_and(|layer| crate::rules::spares_player(&layer.world.level().rules, cause));
    !spared && ask(world, player, amount, cause).is_some_and(|amount| land(world, player, amount, cause))
}

/// What plugins make of a hit on a player, from the attacker a hit's [`death::Slayer`] names.
pub(super) fn ask(world: &mut World, player: Entity, amount: f32, cause: Cause) -> Option<f32> {
    let attacker = death::attacker(world, player);
    crate::damage::ask(world, player, amount, cause, attacker)
}

/// Takes `amount` from a player's health as a hit of `cause`, less what its armour stops of a
/// cause it stops and what its enchantments ward off; false where it took nothing. Plugins were
/// asked ([`hurt`]).
pub(super) fn land(world: &mut World, player: Entity, amount: f32, cause: Cause) -> bool {
    dress(world, player);
    let fiery = matches!(cause, Cause::Fire | Cause::FireTick | Cause::Lava | Cause::Magma);
    if fiery && world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.is_fireproof()) {
        return false;
    }
    let helmeted = cause.is_on_the_head() && world.get::<crate::inventory::Inventory>(player).is_some_and(|inventory| inventory.has_helmet());
    let whole = amount;
    let amount = if helmeted { amount * HELMETED } else { amount };
    let pierced = world.get::<struck::Pierced>(player).map_or(0.0, |pierced| pierced.0);
    let lands = |mut vitals: Mut<Vitals>| {
        let amount = if cause.is_stopped_by_armour() { vitals.health.shielded_but(amount, pierced) } else { amount };
        let amount = vitals.health.warded(amount, cause);
        vitals.health.hurt(amount)
    };
    if !is_mortal(world, player) || !world.get_mut::<Vitals>(player).is_some_and(lands) {
        return false;
    }
    if cause.is_stopped_by_armour() {
        crate::inventory::wear_armour(world, player, whole, cause.is_on_the_head());
    }
    totem::rescue(world, player, cause);
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > 0.0) {
        flash(world, player, cause);
        omens::hurt(world, player);
    }
    settle(world, player, cause);
    true
}

impl HealthBackend for Ctx<'_> {
    fn health(&self, player: PlayerId) -> Option<f32> {
        self.world.get::<Vitals>(self.entity(player)?).map(|vitals| vitals.health.left())
    }

    fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool {
        let Some(entity) = self.entity(player) else { return false };
        self.world.entity_mut(entity).insert(death::Told);
        let hurt = hurt(self.world, entity, amount, cause);
        self.world.entity_mut(entity).remove::<death::Told>();
        hurt
    }

    fn hurt_player_by(&mut self, player: PlayerId, amount: f32, cause: DamageCause, by: &sapling_api::Attacker) -> bool {
        let Some(entity) = self.entity(player) else { return false };
        let slayer = match by {
            sapling_api::Attacker::Player(by) => format!("{}{}", death::BY_PLAYER, by.name),
            sapling_api::Attacker::Mob { kind } => kind.clone(),
            _ => return self.hurt_player(player, amount, cause),
        };
        self.world.entity_mut(entity).insert(death::Slayer(slayer));
        let hurt = self.hurt_player(player, amount, cause);
        self.world.entity_mut(entity).remove::<death::Slayer>();
        hurt
    }

    fn kill_player(&mut self, player: PlayerId) -> bool {
        let Some(entity) = self.entity(player).filter(|entity| is_mortal(self.world, *entity)) else { return false };
        let whole = self.world.get::<Vitals>(entity).map_or(0.0, |vitals| vitals.health.left() + vitals.health.cushion());
        // Plugins see a kill as a hit of all it has left: one they lower lands as a hit.
        let Some(harm) = ask(self.world, entity, whole, Cause::Generic) else { return false };
        if let Some(mut vitals) = self.world.get_mut::<Vitals>(entity) {
            if harm < whole { vitals.health.lose(harm) } else { vitals.health.end() }
        }
        if harm > 0.0 && harm < whole {
            flash(self.world, entity, Cause::Generic);
        }
        settle(self.world, entity, Cause::Generic);
        harm > 0.0
    }
}
