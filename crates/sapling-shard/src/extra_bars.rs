//! Boss bars past the player's own: the client keeps one bar per entity, so each hangs on an
//! unseen entity of its own that only its player is told of and that follows it about.

use std::collections::BTreeMap;

use acacia_proto::packets::{MoveEntity, RemoveEntity};
use acacia_proto::types::{Rotation, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BossBar;

use crate::components::{InLayer, Outgoing, Players, Position};
use crate::hud::{boss_event, boss_events};

/// How far the player may get from its bars' entities before they are moved after it.
const SLACK: f32 = 8.0;
/// MoveEntity's teleport bit.
const TELEPORT: u8 = 0x02;

struct Shown {
    id: u64,
    bar: BossBar,
    at: [f32; 3],
}

#[derive(Component, Default)]
pub(crate) struct ExtraBars(BTreeMap<BarKey, Shown>);

/// Whose a bar is. A game bar that is no boss's own (a raid's) takes a slot here rather than the
/// player's own id, which `ctx.set_boss_bar` uses; the dragon's and the wither's hang on themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BarKey {
    Plugin(u32),
    Raid,
}

/// A new unseen entity at `at`, told to `player` alone.
fn add_holder(world: &mut World, player: Entity, at: [f32; 3]) -> Option<u64> {
    let air = crate::labels::air(world, world.get::<InLayer>(player)?.0)?;
    let id = world.resource::<Players>().next_net_id;
    world.resource_mut::<Players>().next_net_id += 1;
    world.get_mut::<Outgoing>(player)?.0.extend(crate::labels::floating_announcement(id, at, air, ""));
    Some(id)
}

pub(crate) fn set(world: &mut World, player: Entity, key: BarKey, bar: Option<&BossBar>) {
    let Some(feet) = world.get::<Position>(player).map(|p| p.feet) else { return };
    if world.get::<ExtraBars>(player).is_none() {
        world.entity_mut(player).insert(ExtraBars::default());
    }
    let old = world.get_mut::<ExtraBars>(player).expect("just inserted").0.remove(&key);
    let (id, at) = match (&old, bar) {
        (Some(old), _) => (old.id, old.at),
        (None, Some(_)) => match add_holder(world, player, feet) {
            Some(id) => (id, feet),
            None => return,
        },
        (None, None) => return,
    };
    let mut out = world.get_mut::<Outgoing>(player).expect("players have an outbox");
    for (r#type, shown) in boss_events(old.as_ref().map(|old| &old.bar), bar) {
        out.push(&boss_event(id, r#type, &shown));
    }
    if bar.is_none() {
        out.push(&RemoveEntity { entity_id_self: id as i64 });
    }
    if let Some(bar) = bar {
        world.get_mut::<ExtraBars>(player).expect("just inserted").0.insert(key, Shown { id, bar: bar.clone(), at });
    }
}

/// Moves each player's bar entities to it once it got [`SLACK`] away from them.
pub(crate) fn follow(world: &mut World) {
    let mut players = world.query::<(&Position, &mut ExtraBars, &mut Outgoing)>();
    for (position, mut bars, mut out) in players.iter_mut(world) {
        let feet = position.feet;
        for shown in bars.0.values_mut().filter(|shown| shown.at.iter().zip(feet).any(|(was, now)| (was - now).abs() > SLACK)) {
            shown.at = feet;
            let [x, y, z] = feet;
            out.push(&MoveEntity { runtime_entity_id: shown.id, flags: TELEPORT, position: Vec3f { x, y, z }, rotation: Rotation { yaw: 0.0, pitch: 0.0, head_yaw: 0.0 } });
        }
    }
}
