//! A tamed mob of a kind that defends its owner (`behavior.owner_hurt_by_target` and
//! `owner_hurt_target`, a wolf) fights what hurt its owner and what its owner hit, with its
//! tamed bite, until that is dead or gone. As BDS's tamed wolf did (research/captures/bds-g14wolf.txt),
//! it takes up a mob's fight the tick after the mob hurts its owner (not when it only means to),
//! leaps at its foe 2 to 4 off on the ground (`behavior.leap_at_target`, 0.4 across and up), and
//! its bite throws a mob 0.4 as a hunter's does, heard `attack` from its middle a tick later.
//! Its bites (bds-x2wolf.txt, 13 bites of two wolves) came 20 ticks apart at the least, or 31 to 35
//! as the zombie threw it back, from 0.61 to 1.421 off feet to feet: within [`reaches`].

use bevy_ecs::prelude::*;

use super::{Raised, distance, shown};
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Tick};
use crate::creatures::{Health, hunting};

/// Mobs this near their owner take up its fight, and give it up beyond this.
const RALLIES: f32 = 16.0;
/// Its bite reaches its width times this, squared, plus its foe's width, feet to feet squared
/// (`melee_attack`'s `reach_multiplier`), once in this many ticks.
const REACH_TIMES: f32 = 2.0;
const EVERY: u64 = 20;
const PLAYER: [f32; 2] = [0.6, 1.8];
/// How far off a foe it leaps at, and the leap's speed across and up.
const LEAPS: [f32; 2] = [2.0, 4.0];
const LEAP: f32 = 0.4;
const SOUNDS_FROM: f32 = 0.5;

/// Whether a biter of `body` reaches a foe of `size` standing at `at`: 1.428 for a wolf at a zombie.
fn reaches(body: &Body, at: [f32; 3], size: [f32; 2]) -> bool {
    distance(body.at, at).powi(2) <= (body.size[0] * REACH_TIMES).powi(2) + size[0]
}

/// Where `foe` stands and how wide and tall it is, if it is a player or a mob still alive.
fn foe_at(world: &World, foe: Entity) -> Option<([f32; 3], [f32; 2], Entity)> {
    let layer = world.get::<InLayer>(foe)?.0;
    if let Some(position) = world.get::<Position>(foe) {
        return crate::vitals::is_mortal(world, foe).then_some((position.feet, PLAYER, layer));
    }
    let alive = world.get::<Health>(foe).is_some_and(|health| health.left() > 0.0) && world.get::<crate::creatures::Dying>(foe).is_none();
    world.get::<Body>(foe).filter(|_| alive).map(|body| (body.at, body.size, layer))
}

/// The owner's defenders near it take `foe` for theirs.
fn rally(world: &mut World, owner: Entity, foe: Entity) {
    let Some(unique_id) = world.get::<crate::tags::Marks>(owner).map(|marks| marks.unique_id) else { return };
    let Some(feet) = world.get::<Position>(owner).map(|position| position.feet) else { return };
    let now = world.resource::<Tick>().0;
    let mut defenders = world.query::<(Entity, &mut Raised, &crate::entities::Kind, &Body)>();
    for (entity, mut raised, kind, body) in defenders.iter_mut(world) {
        let defends = sapling_data::husbandry(&kind.0).is_some_and(|rules| rules.defends);
        if defends && entity != foe && raised.is_tamed() && raised.kept.owner == Some(unique_id) && !raised.kept.sitting && distance(body.at, feet) <= RALLIES {
            raised.foe = Some((foe, now));
        }
    }
}

/// Something hurt the player `owner`: its defenders fight `by`, unless `by` is one of them.
pub(crate) fn owner_hurt(world: &mut World, owner: Entity, by: Entity) {
    let own = world.get::<Raised>(by).is_some_and(|raised| raised.is_tamed() && raised.kept.owner == world.get::<crate::tags::Marks>(owner).map(|marks| marks.unique_id));
    if !own {
        rally(world, owner, by);
    }
}

/// The player `owner` hit `struck`: its defenders fight it too, unless it is one of its own.
pub(crate) fn owner_hit(world: &mut World, owner: Entity, struck: Entity) {
    owner_hurt(world, owner, struck);
}

/// Before the move of a mob with a foe: it walks at it and bites it within reach. False for one
/// with none, or whose foe is dead, gone or too far.
pub(super) fn chase(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some((foe, ready)), Some(body)) = (world.get::<Raised>(entity).and_then(|raised| raised.foe), world.get::<Body>(entity).copied()) else { return false };
    let Some((at, size, _)) = foe_at(world, foe).filter(|(at, _, in_layer)| *in_layer == layer && distance(body.at, *at) <= RALLIES) else {
        world.get_mut::<Raised>(entity).expect("read above").foe = None;
        return false;
    };
    let rules = sapling_data::husbandry(kind);
    // By a way round where the floor bars it: bds-af2wolf.txt's wolves walked round a wall's ends.
    crate::creatures::hunting::approach(world, entity, layer, &body, at, rules.map_or(0.3, |rules| rules.walks), 0.0);
    let (dx, dz) = (at[0] - body.at[0], at[2] - body.at[2]);
    let across = dx.hypot(dz).max(1e-4);
    // Not over a wall: a leap each tick into it kept the wolf from its way round (fitted, not decompiled).
    let open = world.get::<crate::components::Layer>(layer).is_some_and(|level| crate::creatures::paths::is_clear(&level.world, body.at, at));
    if open && body.on_ground && (LEAPS[0]..=LEAPS[1]).contains(&distance(body.at, at)) {
        if let Some(mut leaping) = world.get_mut::<Body>(entity) {
            leaping.fling([dx / across * LEAP, LEAP, dz / across * LEAP]);
        }
    }
    let now = world.resource::<Tick>().0;
    if reaches(&body, at, size) && now >= ready {
        world.get_mut::<Raised>(entity).expect("read above").foe = Some((foe, now + EVERY));
        let far = dx.hypot(dz).max(1e-4);
        bite(world, entity, layer, kind, foe, rules.and_then(|rules| rules.tame_attack).unwrap_or(2.0), [dx / far, dz / far]);
    }
    true
}

fn bite(world: &mut World, entity: Entity, layer: Entity, kind: &str, foe: Entity, harm: f32, away: [f32; 2]) {
    shown::event(world, entity, acacia_proto::packets::EntityEventEventId::ArmSwing, 0);
    if world.get::<Position>(foe).is_some() {
        crate::vitals::bitten(world, foe, harm, kind, [0.0; 3]);
        return;
    }
    hunting::hit_mob(world, layer, entity, foe, harm, away);
    let (Some(id), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<Body>(entity).copied()) else { return };
    let cell = body.at.map(|c| c.floor() as i32);
    crate::late_sounds::play_later(world, 1, layer, cell, hunting::sound("attack", id, kind, &body, SOUNDS_FROM));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wolf_bites_a_zombie_1_421_off_but_not_a_diagonal_1_4_each_way() {
        let wolf = Body::new([0.0, 64.0, 0.0], [0.0; 3], [0.6, 0.8]);
        let zombie = [0.6, 1.95];
        assert!(reaches(&wolf, [0.19, 64.0, 1.41], zombie), "bds-x2wolf.txt at 57382");
        assert!(!reaches(&wolf, [1.2, 64.0, 1.2], zombie), "the hunters' box rule bit here");
    }
}
