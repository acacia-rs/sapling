//! A mob's health, and what lava and fire do to it, as BDS was seen to
//! (docs/research/mob-damage.md).
// TODO: nothing else hurts (falls, drowning, a block it is in, attacks); water puts a fire out
// without BDS's fizz; a stored mob's health and fire are not written back to its world folder; an
// armour stand has no health here, though BDS burns it.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use sapling_world::World as Level;

use crate::body::{Body, Liquid};
use crate::components::Layer;
use crate::entities;

/// Ticks after a hit in which only a harder one lands, and only by what it adds.
const GUARD: u8 = 10;
/// Ticks a mob burns on after it last touched fire or lava, hurt each `BURN_EVERY`.
const BURNS: u16 = 160;
const BURN_EVERY: u16 = 20;
const FIRE_HARM: f32 = 1.0;
const LAVA_HARM: f32 = 4.0;
const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];

/// What hurt, as `EntityEvent` names it (BDS's `ActorDamageCause`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cause {
    /// Touching fire or lava.
    Fire = 6,
    /// Burning on.
    Burning = 7,
    Lava = 8,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Health {
    left: f32,
    max: f32,
    /// The last hit that landed, while `guarded` ticks of its guard are left.
    last: f32,
    guarded: u8,
    burning: u16,
    /// What viewers know: its health, and whether it is on fire.
    told: f32,
    lit: bool,
}

impl Health {
    pub(super) fn new(max: f32, left: Option<f32>) -> Self {
        let left = left.map_or(max, |left| left.clamp(0.0, max));
        Self { left, max, last: 0.0, guarded: 0, burning: 0, told: left, lit: false }
    }

    pub(super) fn left(&self) -> f32 {
        self.left
    }

    pub(super) fn is_burning(&self) -> bool {
        self.burning > 0
    }

    pub(super) fn end(&mut self) {
        self.left = 0.0;
    }

    /// Whether a hit of `amount` landed.
    fn hurt(&mut self, amount: f32) -> bool {
        let taken = if self.guarded > 0 { amount - self.last } else { amount };
        if taken <= 0.0 {
            return false;
        }
        (self.last, self.guarded) = (amount, GUARD);
        self.left = (self.left - taken).max(0.0);
        true
    }

    /// A tick of fire, `touching` fire or lava, or in water: what hurt it, if anything.
    fn burn(&mut self, touching: bool, wet: bool) -> Option<Cause> {
        self.guarded = self.guarded.saturating_sub(1);
        if touching {
            self.burning = BURNS;
            return self.hurt(FIRE_HARM).then_some(Cause::Fire);
        }
        if wet || self.burning == 0 {
            self.burning = 0;
            return None;
        }
        self.burning -= 1;
        (self.burning > 0 && self.burning % BURN_EVERY == 0 && self.hurt(FIRE_HARM)).then_some(Cause::Burning)
    }
}

/// Whether the body's box reaches into a cell of lava or fire, however full.
fn touches_fire(level: &Level, body: &Body) -> bool {
    const EDGE: f32 = 1e-4;
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=(high[axis] - EDGE).floor() as i32;
    let registry = level.registry();
    let fiery = |x, y, z| level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) && [level.block(x, y, z), level.liquid(x, y, z)].into_iter().any(|id| registry.get(id).is_some_and(|state| FIERY.contains(&state.name)));
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| fiery(x, y, z))))
}

fn hurt_event(id: u64, cause: Cause) -> Bytes {
    sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::HurtAnimation, data: cause as i32, fire_at_position: None })
}

/// A tick of the mob `id`'s health, its body `start` before the tick's move and `now` after;
/// whether it died of it. Fire hurts before viewers are told its health, lava after, so that
/// lava's harm is told a tick late, as on BDS. A `dying` mob is only told of.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, start: &Body, now: &Body, dying: bool) -> bool {
    let Some(mut health) = world.get::<Health>(entity).copied() else { return false };
    let burns = !dying && !sapling_data::is_fire_immune(kind);
    let touching = burns && world.get::<Layer>(layer).is_some_and(|layer| touches_fire(&layer.world, start));
    let mut packets = Vec::new();
    let fire = if burns { health.burn(touching, now.liquid() == Some(Liquid::Water)) } else { None };
    packets.extend(fire.filter(|_| health.left > 0.0).map(|cause| hurt_event(id, cause)));
    if health.left != health.told {
        health.told = health.left;
        packets.push(sapling_net::encode(&sapling_data::mob_health_update(id, health.left, health.max)));
    }
    if burns && health.is_burning() != health.lit {
        health.lit = health.is_burning();
        packets.push(sapling_net::encode(&sapling_data::mob_fire_flags(id, kind, health.lit)));
    }
    if burns && health.left > 0.0 && now.liquid() == Some(Liquid::Lava) && health.hurt(LAVA_HARM) && health.left > 0.0 {
        packets.push(hurt_event(id, Cause::Lava));
    }
    if let Some(mut kept) = world.get_mut::<Health>(entity) {
        *kept = health;
    }
    entities::tell_viewers(world, entity, layer, now.at.map(|c| c.floor() as i32), packets);
    !dying && health.left <= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pig_in_lava_is_hurt_as_bds_hurt_one() {
        // research/captures/bds-burn.txt: touching from its 1st tick, in the lava from its 3rd.
        let mut pig = Health::new(10.0, None);
        let mut seen = Vec::new();
        for tick in 1..=23 {
            let fire = pig.burn(true, false);
            let after_fire = pig.left;
            let lava = tick >= 3 && pig.left > 0.0 && pig.hurt(LAVA_HARM);
            if fire.is_some() || lava {
                seen.push((tick, after_fire, pig.left));
            }
        }
        assert_eq!(seen, [(1, 9.0, 9.0), (3, 9.0, 6.0), (13, 5.0, 2.0), (23, 1.0, 0.0)]);
    }

    #[test]
    fn out_of_lava_a_mob_burns_on_for_eight_seconds_hurt_each_second() {
        let mut golem = Health::new(100.0, None);
        assert_eq!(golem.burn(true, false), Some(Cause::Fire));
        let hits: Vec<u16> = (1..=200).filter(|_| golem.burn(false, false) == Some(Cause::Burning)).collect();
        assert_eq!(hits, [20, 40, 60, 80, 100, 120, 140]);
        assert!(!golem.is_burning() && golem.left() == 92.0);
        golem.burn(true, false);
        assert!(golem.is_burning() && golem.burn(false, true).is_none() && !golem.is_burning(), "water puts it out");
    }
}
