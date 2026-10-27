//! The wither, as BDS's (docs/research/wither-lightning-fireworks.md, "Wither"): built of soul
//! sand and skulls (blocks/summoning.rs) it comes with 1 health and charges 200 ticks, healing
//! and untouchable, then blows up with power 7. It flies at its prey, its heads shoot skulls
//! (heads.rs, skulls.rs), it breaks the blocks about it a second after it is hurt (wreck.rs), and
//! at half health it is armoured, blows up again and calls three wither skeletons. Its boss bar
//! shows every second to players within 55; with none within 50 it heals 1 a second.
// TODO: the armoured dash, the 200-tick death and its blast, a stored wither's shield, and the
// fire rate halving with each sixth of its health lost are not done.

mod heads;
mod skulls;
mod wreck;

use acacia_proto::packets::{BossEventColor, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};

use super::health::{Cause, Health};
use super::{Birth, blast, hunting};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick, in_world};
use crate::entities;

pub(super) const KIND: &str = "minecraft:wither";
const SKELETON: &str = "minecraft:wither_skeleton";
/// Ticks it charges for; it heals max/200 (rounded up) a tick of them.
const CHARGE: u32 = 200;
const BLAST: f32 = 7.0;
const GRIEFING: &str = "mobGriefing";
/// Its pack's health: the health BDS announced it with was already normal's share of it.
const PACK_HEALTH: f32 = 600.0;
/// Its health's share of its pack's on easy and normal.
const EASY: f32 = 0.5;
const NORMAL: f32 = 0.75;
/// Ticks it stands stunned after its half-health blast, which is this far up it, and how many
/// skeletons it calls.
const STUNNED: u32 = 80;
const HEAD_UP: f32 = 3.0;
const SKELETONS: usize = 3;
/// The bar shows to players this near, and it heals 1 every second with no player this near.
const BAR_REACH: f32 = 55.0;
const LONELY: f32 = 50.0;
const SECOND: u64 = 20;
const TITLE: &str = "entity.wither.name";
/// It flies at its prey at this speed across, keeping off it this far across and this far over it.
const SPEED: f32 = 0.3;
const KEEPS_OFF: f32 = 3.0;
const KEEPS_OVER: f32 = 5.0;
const SIGHT: f32 = 70.0;

/// A wither's own state.
#[derive(Component, Default)]
pub(super) struct Wither {
    /// Ticks of its charge left.
    charging: u32,
    stunned: u32,
    armoured: bool,
    /// Ticks to its block breaking after it was hurt.
    wrecks: Option<u32>,
    heads: heads::Heads,
}

/// Its own blasts and skulls do not hurt it while this is on.
#[derive(Component)]
struct Blasting;

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Wither::default());
    }
}

/// Puts a wither a world folder kept (its compound `compound`) back as it was: its max health,
/// its charge and its armour (research/captures/bds-keepmobs.txt).
pub(super) fn adopt(world: &mut World, wither: Entity, compound: &[u8]) {
    let Some(record) = sapling_world::wither_record(compound).filter(|_| world.get::<Wither>(wither).is_some()) else { return };
    if let Some(health) = world.get::<Health>(wither).copied() {
        let max = record.max_health as f32;
        world.entity_mut(wither).insert(Health::new(max, Some(health.left().min(max)), health.burning()));
    }
    let charging = u32::try_from(record.charging).unwrap_or(0);
    world.entity_mut(wither).insert(Wither { charging, armoured: record.armoured, ..Wither::default() });
}

/// Writes each kept wither's fight into what the world folder keeps of it.
pub(super) fn keep(world: &mut World) {
    let withers: Vec<(Entity, sapling_world::WitherRecord)> = world
        .query_filtered::<(Entity, &Wither, &Health), With<entities::Stored>>()
        .iter(world)
        .map(|(wither, kept, health)| (wither, sapling_world::WitherRecord { charging: kept.charging as i32, armoured: kept.armoured, max_health: health.max() as i32 }))
        .collect();
    for (wither, record) in withers {
        entities::restore(world, wither, |compound, _| sapling_world::withered_actor(compound, record).ok());
    }
}

/// A wither built of blocks with its feet at `feet` (it charges as [`begin`] has it).
pub(crate) fn build(world: &mut World, layer: Entity, feet: [f32; 3]) -> Option<Entity> {
    let id = super::born(world, layer, KIND, feet, Birth { reason: SpawnReason::Built, yaw: 0.0, gene: None, sound: "spawn" })?;
    entities::find(world, id)
}

/// A new wither, built or summoned: it comes with 1 health of a max its difficulty scales, and
/// charges (BDS's summoned one was kept charging at full health, research/captures/bds-keepmobs.txt).
pub(super) fn begin(world: &mut World, wither: Entity, layer: Entity) {
    let share = match world.resource::<Settings>().0.difficulty {
        Difficulty::Easy => EASY,
        Difficulty::Normal => NORMAL,
        _ => 1.0,
    };
    if world.get::<Wither>(wither).is_none() {
        return;
    }
    let max = PACK_HEALTH * share;
    world.entity_mut(wither).insert((Health::new(max, Some(1.0), 0), Wither { charging: CHARGE, ..Wither::default() }));
    tell(world, wither, layer, vec![(Key::WitherInvulnerableTicks, Plain::Int(CHARGE as i32)), (Key::WitherAerialAttack, Plain::Byte(1))]);
}

/// Whether a hit by `cause` leaves the mob whole: a charging wither takes nothing but the void,
/// none takes a fall or drowning, an armoured one no arrow, and its own blasts spare it.
pub(crate) fn spares(world: &World, mob: Entity, cause: Cause) -> bool {
    let Some(wither) = world.get::<Wither>(mob) else { return false };
    let own = world.get::<Blasting>(mob).is_some() && matches!(cause, Cause::EntityExplosion | Cause::BlockExplosion);
    (wither.charging > 0 && cause != Cause::Void) || matches!(cause, Cause::Fall | Cause::Drowning) || (wither.armoured && cause == Cause::Projectile) || own
}

/// Tells the wither's viewers its metadata `items`.
fn tell(world: &mut World, wither: Entity, layer: Entity, items: Vec<(Key, Plain)>) {
    let (Some(id), Some(at)) = (world.get::<NetId>(wither).map(|id| id.0), world.get::<Body>(wither).map(|body| body.at)) else { return };
    let legacy = |plain: &Plain| match plain {
        Plain::Byte(_) => (Type::Byte, 0),
        Plain::Int(_) => (Type::Int, 2),
        _ => (Type::Long, 7),
    };
    let metadata = items.into_iter().map(|(key, plain)| {
        let (r#type, legacy_type) = legacy(&plain);
        MetadataDictionaryItem { key, r#type, legacy_type, value: Value::Default(plain) }
    });
    let told = SetEntityData { runtime_entity_id: id, metadata: metadata.collect(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, wither, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// A blast of the wither's own at `at`, breaking blocks where mobs may grief.
fn blow_up(world: &mut World, wither: Entity, layer: Entity, at: [f32; 3]) {
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    world.entity_mut(wither).insert(Blasting);
    blast::explode(world, None, layer, at, BLAST, if griefs { blast::Wreck::Decays } else { blast::Wreck::Nothing });
    world.entity_mut(wither).remove::<Blasting>();
}

/// A tick of its charge: it heals, and blows up at the end.
fn charge(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let left = world.get::<Wither>(wither).map_or(0, |kept| kept.charging) - 1;
    if let Some(mut health) = world.get_mut::<Health>(wither) {
        let by = (health.max() / CHARGE as f32).ceil();
        health.mend(by);
    }
    if left == 0 {
        blow_up(world, wither, layer, body.at);
        entities::after_announcing(world, layer, body.at.map(|c| c.floor() as i32), vec![heads::sound("spawn", body.at)]);
    }
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.charging = left;
    }
    tell(world, wither, layer, vec![(Key::WitherInvulnerableTicks, Plain::Int(left as i32))]);
}

/// At half health it is armoured, blows up from its head, stands stunned, and off easy calls skeletons.
fn armour(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let half = world.get::<Health>(wither).is_some_and(|health| health.left() <= health.max() / 2.0);
    if !half || world.get::<Wither>(wither).is_none_or(|kept| kept.armoured) {
        return;
    }
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        (kept.armoured, kept.stunned) = (true, STUNNED);
    }
    tell(world, wither, layer, vec![(Key::WitherAerialAttack, Plain::Byte(0))]);
    blow_up(world, wither, layer, [body.at[0], body.at[1] + HEAD_UP, body.at[2]]);
    if matches!(world.resource::<Settings>().0.difficulty, Difficulty::Normal | Difficulty::Hard) {
        for _ in 0..SKELETONS {
            super::spawn(world, layer, SKELETON, body.at, SpawnReason::Summoned);
        }
    }
}

/// Flies at its prey's head height and over, closing in across till it is near.
fn fly(world: &mut World, wither: Entity, body: &Body, prey: Option<[f32; 3]>) {
    let Some(mut moving) = world.get_mut::<Body>(wither) else { return };
    let [mut x, mut y, mut z] = [moving.motion[0], moving.motion[1] * 0.6, moving.motion[2]];
    if let Some(prey) = prey {
        if body.at[1] < prey[1] + KEEPS_OVER {
            y = y.max(0.0);
            y += SPEED - y * 0.6;
        }
        let across = [prey[0] - body.at[0], prey[2] - body.at[2]];
        let long = across[0].hypot(across[1]);
        if long > KEEPS_OFF {
            x += across[0] / long * SPEED - x * 0.6;
            z += across[1] / long * SPEED - z * 0.6;
        }
    }
    moving.motion = [x, y, z];
}

/// Before its move: charging, stunned or hunting. Whether it is a wither.
pub(super) fn steer(world: &mut World, wither: Entity, layer: Entity) -> bool {
    let (Some(kept), Some(body)) = (world.get::<Wither>(wither), world.get::<Body>(wither).copied()) else { return false };
    let (charging, stunned) = (kept.charging, kept.stunned);
    if charging > 0 {
        charge(world, wither, layer, &body);
        return true;
    }
    armour(world, wither, layer, &body);
    wreck::tick(world, wither, layer, &body);
    heal_alone(world, wither, layer, &body);
    if stunned > 0 {
        if let Some(mut kept) = world.get_mut::<Wither>(wither) {
            kept.stunned -= 1;
        }
        return true;
    }
    let prey = hunting::nearest(world, layer, &body, SIGHT).map(|(_, feet)| feet);
    fly(world, wither, &body, prey);
    heads::tick(world, wither, layer, &body);
    true
}

/// A second's healing with no player near.
fn heal_alone(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    if world.resource::<Tick>().0 % SECOND != 0 {
        return;
    }
    let near = |feet: [f32; 3]| (0..3).all(|axis| (feet[axis] - body.at[axis]).abs() <= LONELY);
    let lonely = !in_world(world).into_iter().any(|player| world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) && world.get::<Position>(player).is_some_and(|at| near(at.feet)));
    if let Some(mut health) = world.get_mut::<Health>(wither).filter(|_| lonely) {
        health.mend(1.0);
    }
}

/// The wither was hurt: its blocks are broken a second on.
pub(crate) fn hurt(world: &mut World, mob: Entity) {
    if let Some(mut kept) = world.get_mut::<Wither>(mob).filter(|kept| kept.wrecks.is_none()) {
        kept.wrecks = Some(wreck::AFTER);
    }
}

/// Skulls fly, and each wither's bar shows every second.
pub(super) fn tick(world: &mut World) {
    skulls::tick(world);
    if world.resource::<Tick>().0 % SECOND != 0 {
        return;
    }
    let bars: Vec<(Entity, i64, f32, [f32; 3])> = world.query_filtered::<(&InLayer, &NetId, &Body, &Health), With<Wither>>().iter(world).map(|(layer, id, body, health)| (layer.0, id.0 as i64, health.left() / health.max(), body.at)).collect();
    for (layer, id, progress, at) in bars {
        crate::dragon::show_boss_bar(world, layer, id, TITLE, BossEventColor::RebeccaPurple, progress, at, BAR_REACH);
    }
}
