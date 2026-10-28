//! The warden, as BDS's was in the `warden` drill (research/captures/bds-warden-findings.md): it
//! sniffs for players about it, goes to look at what disturbs it (hearing.rs) and grows angry at
//! the player behind it (anger.rs); angry, it roars, then hunts that player as any hunter, booming
//! at one it cannot walk to (boom.rs). Summoned emerging it climbs out first; left undisturbed a
//! minute it digs down and is gone.
//! It casts darkness about it (darkness.rs).
// TODO: mobs are no suspects; a player's vibrations
// but its steps (blocks, doors, containers, shots) name no suspect; damage is not heard.

mod anger;
mod boom;
mod darkness;
mod hearing;

pub(crate) use hearing::hear;

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags2};
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Position, Tick, in_world};
use crate::{entities, mobs};
use super::{hunting, legs};

const KIND: &str = "minecraft:warden";
const EMERGING_EVENT: &str = "minecraft:spawn_emerging";
/// Its sounds come from this much of its height up.
const HEAD: f32 = 0.85;
/// Ticks it sniffs, roars, climbs out and digs down for: its pack's durations and a tick, as BDS's took.
const SNIFFS: u64 = 84;
const ROARS: u64 = 85;
const EMERGES: u64 = 141;
const DIGS: u64 = 111;
/// It sniffs again 5 to 10 seconds after, for players this near; one this near across and up is suspected.
const SNIFF_AGAIN: [u64; 2] = [100, 200];
const SNIFFS_FOR: f32 = 24.0;
const SUSPECTS: [f32; 2] = [6.0, 20.0];
/// Undisturbed this long, it digs down.
const IDLE: u64 = 1200;
/// It goes to look at a disturbance this fast and stops this near it (BDS's stopped 0.9 off the
/// player it went to).
const INVESTIGATES: f32 = 0.21;
const ARRIVES: f32 = 0.9;
/// Its `FlagsExtended` bits as BDS sent them: at rest, and one for each of the rest.
const REST: i64 = i64::MIN | 1 << 25;
const ROARING: i64 = 1 << 20;
const EMERGING: i64 = 1 << 40;
const SNIFFING: i64 = 1 << 41;
const DIGGING: i64 = 1 << 42;
const BOOMING: i64 = 1 << 43;

#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Idle,
    Sniffing(u64),
    Roaring(u64, Entity),
    Booming(u64, Entity),
    Emerging(u64),
    Digging(u64),
}

impl Stage {
    fn flags(self) -> i64 {
        REST | match self {
            Stage::Idle => 0,
            Stage::Sniffing(_) => SNIFFING,
            Stage::Roaring(..) => ROARING,
            Stage::Booming(..) => BOOMING,
            Stage::Emerging(_) => EMERGING,
            Stage::Digging(_) => DIGGING,
        }
    }
}

#[derive(Component, Default)]
pub(super) struct Warden {
    stage: Stage,
    anger: anger::Anger,
    next_sniff: u64,
    calm_since: u64,
    disturbance: Option<[f32; 3]>,
    touch_ready: u64,
    deaf_until: u64,
    boom_ready: u64,
    /// The flags, heartbeat and ambient sound its viewers were last told.
    told: Option<(i64, i32, &'static str)>,
}

impl Warden {
    fn listens(&self, now: u64) -> bool {
        now >= self.deaf_until && !matches!(self.stage, Stage::Emerging(_) | Stage::Digging(_))
    }
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind != KIND {
        return;
    }
    let now = world.resource::<Tick>().0;
    let stage = if event == Some(EMERGING_EVENT) { Stage::Emerging(now) } else { Stage::Idle };
    world.entity_mut(entity).insert(Warden { stage, calm_since: now, ..Warden::default() });
}

/// Whether it does something of its own this tick that keeps it from a hunter's hit.
pub(super) fn is_busy(world: &World, entity: Entity) -> bool {
    world.get::<Warden>(entity).is_some_and(|warden| !matches!(warden.stage, Stage::Idle))
}

/// A player hit it: it is angry at it at once (`angry_threshold` and `angry_boost`).
pub(super) fn provoked(world: &mut World, entity: Entity, player: Entity) {
    let now = world.resource::<Tick>().0;
    if let Some(mut warden) = world.get_mut::<Warden>(entity) {
        warden.anger.raise(player, anger::HIT);
        warden.calm_since = now;
    }
}

/// Wardens done digging down go, without a death or loot, as BDS's did.
pub(super) fn tick(world: &mut World) {
    darkness::tick(world);
    let now = world.resource::<Tick>().0;
    let gone: Vec<Entity> = world.query::<(Entity, &Warden)>().iter(world).filter(|(_, warden)| matches!(warden.stage, Stage::Digging(since) if now >= since + DIGS)).map(|(entity, _)| entity).collect();
    for warden in gone {
        mobs::remove(world, warden);
    }
}

/// Before its move: true where it stands or walks by its own rules, false where it hunts or
/// strolls as any mob.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let (Some(body), Some(stage)) = (world.get::<Body>(entity).copied(), world.get::<Warden>(entity).map(|warden| warden.stage)) else { return false };
    let now = world.resource::<Tick>().0;
    if now.is_multiple_of(anger::COOLS) {
        let suspects: Vec<Entity> = in_world(world).into_iter().filter(|player| hearing::is_suspect(world, *player, layer)).collect();
        world.get_mut::<Warden>(entity).expect("read above").anger.cool(|suspect| suspects.contains(&suspect));
    }
    if !matches!(stage, Stage::Emerging(_) | Stage::Digging(_)) {
        hearing::arrive(world, entity, layer, id, kind, &body);
        hearing::touched(world, entity, layer, id, kind, &body);
    }
    let own = decide(world, entity, layer, id, kind, &body, stage, now);
    tell(world, entity, layer, id, &body);
    own
}

/// What the warden does in `stage` this tick; see [`steer`].
#[allow(clippy::too_many_arguments)]
fn decide(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, stage: Stage, now: u64) -> bool {
    let set = |world: &mut World, stage: Stage| world.get_mut::<Warden>(entity).expect("a warden").stage = stage;
    match stage {
        Stage::Emerging(since) if now >= since + EMERGES => {
            set(world, Stage::Idle);
            world.get_mut::<Warden>(entity).expect("a warden").calm_since = now;
        }
        Stage::Emerging(_) | Stage::Digging(_) => {}
        Stage::Sniffing(since) if now >= since + SNIFFS => sniffed(world, entity, layer, id, kind, body, now),
        // A roar comes before a sniff.
        Stage::Sniffing(_) if world.get::<Warden>(entity).is_some_and(|warden| warden.anger.level() >= anger::ANGRY) => {
            set(world, Stage::Idle);
            return idle(world, entity, layer, id, kind, body, now);
        }
        Stage::Sniffing(_) => {}
        Stage::Roaring(since, suspect) if now >= since + ROARS => {
            set(world, Stage::Idle);
            let angry = world.get::<Warden>(entity).is_some_and(|warden| warden.anger.at(suspect) >= anger::ANGRY);
            if angry && hearing::is_suspect(world, suspect, layer) {
                hunting::hunt(world, entity, layer, id, kind, body, suspect);
            }
        }
        Stage::Roaring(..) => {}
        Stage::Booming(since, _) if now >= since + boom::LASTS => {
            set(world, Stage::Idle);
            world.get_mut::<Warden>(entity).expect("a warden").boom_ready = now + boom::AGAIN;
        }
        Stage::Booming(since, prey) => boom::tick(world, entity, layer, id, kind, body, since, prey),
        Stage::Idle => return idle(world, entity, layer, id, kind, body, now),
    }
    legs::rest(world, entity);
    true
}

/// An idle warden lets prey it is no longer angry at go, roars at a suspect it grew angry at,
/// booms at prey out of reach or leaves it to hunting.rs, goes to look at a disturbance, sniffs,
/// or digs down; true where it stands or walks by these.
fn idle(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, now: u64) -> bool {
    let warden = world.get::<Warden>(entity).expect("a warden");
    let (top, disturbance, boom_ready, next_sniff, calm_since) = (warden.anger.top(), warden.disturbance, warden.boom_ready, warden.next_sniff, warden.calm_since);
    let calmed = hunting::quarry(world, entity).is_some_and(|(prey, _)| warden.anger.at(prey) < anger::ANGRY);
    if calmed {
        hunting::give_up(world, entity, layer, id, kind, body);
    }
    let prey = hunting::quarry(world, entity).map(|(prey, _)| prey);
    let next = if let Some((suspect, _)) = top.filter(|(suspect, anger)| *anger >= anger::ANGRY && prey != Some(*suspect)) {
        Stage::Roaring(now, suspect)
    } else if let Some(prey) = prey {
        world.get_mut::<Warden>(entity).expect("a warden").calm_since = now;
        let feet = world.get::<Position>(prey).map(|position| position.feet);
        if !(now >= boom_ready && feet.is_some_and(|feet| boom::can_boom(world, layer, body, feet))) {
            return false;
        }
        Stage::Booming(now, prey)
    } else if let Some(at) = disturbance {
        if (at[0] - body.at[0]).hypot(at[2] - body.at[2]) > ARRIVES {
            legs::walk(world, entity, layer, body, [at[0], at[2]], INVESTIGATES, ARRIVES);
            return true;
        }
        world.get_mut::<Warden>(entity).expect("a warden").disturbance = None;
        return false;
    } else if now >= next_sniff && nearest(world, layer, body).is_some_and(|(_, far)| far <= SNIFFS_FOR) {
        Stage::Sniffing(now)
    } else if now >= calm_since + IDLE {
        Stage::Digging(now)
    } else {
        return false;
    };
    world.get_mut::<Warden>(entity).expect("a warden").stage = next;
    legs::rest(world, entity);
    true
}

/// Its sniff is over: the nearest player it smelled is suspected if near enough, else looked at.
fn sniffed(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, now: u64) {
    let wait = SNIFF_AGAIN[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll((SNIFF_AGAIN[1] - SNIFF_AGAIN[0] + 1) as u32));
    let mut warden = world.get_mut::<Warden>(entity).expect("a warden");
    (warden.stage, warden.next_sniff) = (Stage::Idle, now + wait);
    let Some((player, _)) = nearest(world, layer, body).filter(|(_, far)| *far <= SNIFFS_FOR) else { return };
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return };
    let suspected = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]) <= SUSPECTS[0] && (feet[1] - body.at[1]).abs() <= SUSPECTS[1];
    hearing::disturb(world, entity, layer, id, kind, body, suspected.then_some((player, anger::ANNOYS)), feet, 0);
}

/// The nearest player it may be angry at, and how far.
fn nearest(world: &World, layer: Entity, body: &Body) -> Option<(Entity, f32)> {
    let far = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    in_world(world)
        .into_iter()
        .filter(|player| hearing::is_suspect(world, *player, layer))
        .filter_map(|player| Some((player, far(world.get::<Position>(player)?.feet))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// Tells its viewers its flags, heartbeat and ambient sound where they changed.
fn tell(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body) {
    let Some(warden) = world.get::<Warden>(entity) else { return };
    let level = warden.anger.level();
    let now = (warden.stage.flags(), anger::heartbeat(level), anger::ambient(level));
    let before = warden.told;
    if before == Some(now) {
        return;
    }
    world.get_mut::<Warden>(entity).expect("read above").told = Some(now);
    let item = |key, r#type, legacy_type, value| MetadataDictionaryItem { key, r#type, legacy_type, value };
    let mut metadata = Vec::new();
    if before.is_none_or(|before| before.0 != now.0) {
        metadata.push(item(Key::FlagsExtended, Type::Long, 7, Value::FlagsExtended(MetadataFlags2(now.0))));
    }
    if before.is_some_and(|before| before.2 != now.2) {
        metadata.push(item(Key::AmbientSoundEventName, Type::String, 4, Value::Default(Plain::String(now.2.into()))));
    }
    if before.is_none_or(|before| before.1 != now.1) {
        metadata.push(item(Key::HeartbeatIntervalTicks, Type::Int, 2, Value::Default(Plain::Int(now.1))));
    }
    let packet = SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&packet)]);
}
