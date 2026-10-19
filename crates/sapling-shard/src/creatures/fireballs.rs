//! A ghast's fireballs, as BDS's flew (research/captures/bds-ghast-findings.md): straight at 1.6
//! a tick; one that meets a player hurts it 6 and sets it alight, stands 6 ticks and blows up
//! there with power 1; one that meets a block blows up where it meets it.
// TODO: a blast lights no fire; a fireball is not pushed off its ghast, hits no mob, and cannot be
// struck back.

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;

use super::blast::{self, Wreck};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const KIND: &str = "minecraft:fireball";
const SIZE: [f32; 2] = [1.0, 1.0];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It leaves this far up its ghast and this far towards its prey, at this speed, aimed this far
/// up its prey.
const HELD: f32 = 0.5;
const AHEAD: f32 = 1.0;
const SPEED: f32 = 1.6;
const AIMED: f32 = 0.9;
/// What it costs whom it meets, the ticks it sets them alight, the ticks it stands after, and
/// its blast.
const HARM: f32 = 6.0;
const BURNS: u16 = 100;
const STANDS: u64 = 6;
const POWER: f32 = 1.0;
/// BDS's flew on; here one that met nothing goes after this many ticks, so that misses do not pile up.
const FLIES: u64 = 200;
const GRIEFING: &str = "mobGriefing";

#[derive(Component)]
struct Fireball {
    kind: String,
    born: u64,
    /// The tick it met a player, and whom.
    met: Option<(u64, Entity)>,
}

/// A ghast of `kind` with the body `ghast` shoots a fireball at a player whose feet are `prey`.
pub(super) fn fire(world: &mut World, layer: Entity, ghast: Entity, kind: &str, body: &Body, prey: [f32; 3]) {
    let mouth = [body.at[0], body.at[1] + HELD, body.at[2]];
    let aim: [f32; 3] = std::array::from_fn(|axis| prey[axis] + if axis == 1 { AIMED } else { 0.0 } - mouth[axis]);
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let across = aim[0].hypot(aim[2]).max(1e-4);
    let from = [mouth[0] + aim[0] / across * AHEAD, mouth[1], mouth[2] + aim[2] / across * AHEAD];
    let motion = aim.map(|c| c / long * SPEED);
    let owner = world.get::<NetId>(ghast).map_or(-1, |id| id.0 as i64);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(fireball) = entities::spawn(world, layer, KIND, Body::new(from, motion, SIZE), from, announce) else { return };
    let born = world.resource::<Tick>().0;
    world.entity_mut(fireball).insert(Fireball { kind: kind.to_owned(), born, met: None });
}

/// The player of `layer` whose box the fireball's, its bottom middle at `at`, reaches into.
fn met(world: &World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let across = (PLAYER[0] + SIZE[0]) / 2.0;
    in_world(world).into_iter().find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

fn burst(world: &mut World, fireball: Entity, layer: Entity, at: [f32; 3]) {
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    blast::explode_fireball(world, fireball, layer, at, POWER, if griefs { Wreck::Breaks } else { Wreck::Nothing });
}

/// A tick of one fireball.
fn fly(world: &mut World, fireball: Entity, layer: Entity, id: u64) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let mut body = *world.get::<Body>(fireball)?;
    let (kind, born, met_at) = world.get::<Fireball>(fireball).map(|ball| (ball.kind.clone(), ball.born, ball.met))?;
    // It hurts as it stops standing, and blows up the tick after.
    if let Some((since, player)) = met_at {
        if now == since + STANDS && vitals::shot(world, player, HARM, &kind, body.at, false) {
            vitals::set_alight(world, player, BURNS);
        }
        if now > since + STANDS {
            burst(world, fireball, layer, body.at);
        }
        return Some(());
    }
    if now >= born + FLIES {
        mobs::remove(world, fireball);
        return None;
    }
    // Looked at a quarter of a step at a time: a whole one is as long as a player is met across.
    let along = |part: f32| -> [f32; 3] { std::array::from_fn(|axis| body.at[axis] + body.motion[axis] * part) };
    let struck = [0.25, 0.5, 0.75, 1.0].into_iter().find_map(|part| met(world, layer, along(part)).map(|player| (player, along(part))));
    let to = struck.map_or(along(1.0), |(_, at)| at);
    if !Body::new(to, [0.0; 3], SIZE).is_free(&world.get::<Layer>(layer)?.world) {
        burst(world, fireball, layer, body.at);
        return None;
    }
    body.at = to;
    *world.get_mut::<Body>(fireball)? = body;
    let packets = world.get_mut::<Told>(fireball)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, fireball, layer, body.at.map(|c| c.floor() as i32), packets);
    if let Some((player, _)) = struck {
        world.get_mut::<Fireball>(fireball)?.met = Some((now, player));
    }
    Some(())
}

/// Moves every fireball a tick.
pub(super) fn tick(world: &mut World) {
    let fireballs: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Fireball>>().iter(world).map(|(ball, layer, id)| (ball, layer.0, id.0)).collect();
    for (fireball, layer, id) in fireballs {
        fly(world, fireball, layer, id);
    }
}
