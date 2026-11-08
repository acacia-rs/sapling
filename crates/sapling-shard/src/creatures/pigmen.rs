//! A zombie pigman's anger, by its pack's `minecraft:angry` (`broadcast_anger`, `broadcast_range`
//! 20, `duration` 25): one that takes a player for its prey sets every pigman within 20 on that player.
//! BDS's two bystanders 9 off took the player as the one hurt did (research/captures/bds-h2dmgm.txt),
//! and all three took it a tick after the hurt, by `/damage` or a melee hit alike; the bystanders were
//! heard `mob.warning` and shown `ANGRY` in that tick, the hurt one a tick later (bds-o1pig.txt).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities::{self, Kind};

const KIND: &str = "minecraft:zombie_pigman";
const SPREADS: f32 = 20.0;
const LASTS: u64 = 500;

/// The player that provoked a pigman, and the tick it did: the pigman takes it at its next tick.
#[derive(Component)]
pub(super) struct Provoked(Entity, u64);

/// A pigman another's anger reached: it warns as it takes its prey, not a tick later.
#[derive(Component)]
struct Bystander;

/// The ticks its `ANGRY` shows from and to, and whether it is shown.
#[derive(Component)]
struct Angry {
    from: u64,
    until: u64,
    shown: bool,
}

/// A pigman provoked by `player` waits for its next tick ([`join`]); false for any other kind.
pub(super) fn waits(world: &mut World, mob: Entity, kind: &str, player: Entity) -> bool {
    if kind != KIND {
        return false;
    }
    let now = world.resource::<Tick>().0;
    if world.get::<Provoked>(mob).is_some_and(|provoked| provoked.1 < now) {
        return false;
    }
    world.entity_mut(mob).insert(Provoked(player, now));
    true
}

/// Whether a pigman taking its prey warns at once (a bystander) rather than a tick later.
pub(super) fn warns_at_once(world: &World, mob: Entity) -> bool {
    world.get::<Bystander>(mob).is_some()
}

/// Before its move: the pigman provoked in an earlier tick takes its prey, and its `ANGRY` shows or ends.
pub(super) fn join(world: &mut World, pigman: Entity, layer: Entity, body: &Body) {
    show(world, pigman, layer, body);
    let now = world.resource::<Tick>().0;
    let Some(&Provoked(player, since)) = world.get::<Provoked>(pigman) else { return };
    if now <= since {
        return;
    }
    let Some(id) = world.get::<NetId>(pigman).map(|id| id.0) else { return };
    if world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) {
        super::hunting::provoke(world, pigman, layer, id, KIND, body, player);
    }
    world.entity_mut(pigman).remove::<Provoked>();
}

fn anger(world: &mut World, pigman: Entity, from: u64) {
    world.entity_mut(pigman).insert(Angry { from, until: from + LASTS, shown: false });
}

/// Shows the pigman's `ANGRY` as its anger starts and ends.
fn show(world: &mut World, pigman: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(state) = world.get::<Angry>(pigman) else { return };
    let angry = (state.from..state.until).contains(&now);
    if angry == state.shown {
        return;
    }
    let Some(id) = world.get::<NetId>(pigman).map(|id| id.0) else { return };
    if angry {
        world.get_mut::<Angry>(pigman).expect("read above").shown = true;
    } else {
        world.entity_mut(pigman).remove::<Angry>();
    }
    let flags = entities::mob_flags(world, pigman, KIND, MetadataFlags1::ANGRY, angry);
    entities::tell_viewers(world, pigman, layer, body.at.map(|c| c.floor() as i32), vec![crate::sync::flags_packet(id, flags)]);
}

/// The mob at `at` took `player` for its prey: if a pigman, it is angry from the next tick, and so is
/// every pigman within 20 from now, taking that player at once.
pub(super) fn spread_anger(world: &mut World, pigman: Entity, layer: Entity, at: [f32; 3], player: Entity) {
    if world.get::<Kind>(pigman).is_none_or(|kind| kind.0 != KIND) {
        return;
    }
    if world.get::<Bystander>(pigman).is_some() {
        return;
    }
    let now = world.resource::<Tick>().0;
    anger(world, pigman, now + 1);
    let near: Vec<(Entity, u64, Body)> = world
        .query::<(Entity, &InLayer, &NetId, &Body, &Kind)>()
        .iter(world)
        .filter(|(other, in_layer, _, body, kind)| *other != pigman && in_layer.0 == layer && kind.0 == KIND && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() <= SPREADS * SPREADS)
        .map(|(other, _, id, body, _)| (other, id.0, *body))
        .collect();
    for (other, id, body) in near {
        if super::is_alive(world, other) {
            // Marked as provoked a tick ago, so it takes the prey now rather than waiting.
            world.entity_mut(other).insert((Provoked(player, now.saturating_sub(1)), Bystander));
            super::hunting::provoke(world, other, layer, id, KIND, &body, player);
            world.entity_mut(other).remove::<(Provoked, Bystander)>();
            if world.get::<Angry>(other).is_none() {
                anger(world, other, now);
            }
            show(world, other, layer, &body);
        }
    }
}
