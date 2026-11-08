//! A fox's `behavior.avoid_mob_type` (its pack): a player who is not sneaking and whom it does not
//! trust, a wolf or a polar bear, whose box meets the fox's grown 3 each way, sends it to a spot up to
//! 16 across that is further from it, at 1.5 times its pace while within 7 of it and its pace beyond.
//! Foxes 3 and 3.6 off fled, 4 and 5.8 off did not; flights ran 2 to 10 blocks and ended at a cell's
//! middle, with `FlagsExtended` bit 22 on till then (research/captures/bds-ar2fox.txt, bds-ar2sneak.txt).
//! A bred fox trusts the players who fed its parents (`minecraft:trust`, `trusting_fox`).
// TODO: a wild cat's and an ocelot's flight from players (walk 0.8, sprint 1.33) is not built; trust
// is not saved with the fox.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2};
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::shown;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, Position, Tick, in_world};
use crate::creatures::{Creature, Dying, legs, paths};
use crate::entities::Kind;
use crate::movement::Motion;

const FOX: &str = "minecraft:fox";
const FEARED: [&str; 2] = ["wolf", "polarbear"];
/// How far each way past the fox's box a threat is seen.
const SEES: f32 = 3.0;
const SPRINTS_WITHIN: f32 = 7.0;
const SPRINT: f32 = 1.5;
/// Where it flees to is looked for this far across, this many times.
const AWAY: f32 = 16.0;
const TRIES: usize = 10;
const ARRIVED: f32 = 0.6;
const GIVES_UP: u64 = 200;
const PLAYER_BOX: [f32; 2] = [0.6, 1.8];
/// A fox's `FlagsExtended` at rest, and the bit on while it flees (acacia-proto names it `AVOIDING_BLOCK`).
const REST: i64 = 1 << 25;
const FLEEING: i64 = 1 << 22;

/// Where a fleeing fox runs to, from whom, and the tick it gives up in.
#[derive(Component, Clone, Copy)]
pub(super) struct Avoiding {
    from: Entity,
    spot: [f32; 2],
    until: u64,
}

/// The players a fox trusts, by `UniqueID`.
#[derive(Component, Default)]
pub(super) struct Trusts(Vec<i64>);

/// A fox born of `parents` trusts the players who fed them into love.
pub(super) fn trust_breeders(world: &mut World, baby: Entity, kind: &str, parents: [Option<i64>; 2]) {
    if kind == FOX {
        world.entity_mut(baby).insert(Trusts(parents.into_iter().flatten().collect()));
    }
}

/// Before the move of a fox: it flees what it fears near, or keeps fleeing. False when it flees nothing.
pub(super) fn flee(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    if kind != FOX {
        return false;
    }
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let now = world.resource::<Tick>().0;
    let going = world.get::<Avoiding>(entity).copied().filter(|going| now < going.until && flat(body.at, going.spot) >= ARRIVED);
    let going = match going {
        Some(going) => going,
        None => {
            let was = world.entity_mut(entity).take::<Avoiding>().is_some();
            let Some(going) = threat(world, entity, layer, &body).and_then(|(from, at)| spot_away(world, layer, &body, at).map(|spot| Avoiding { from, spot, until: now + GIVES_UP })) else {
                if was {
                    show(world, entity, false);
                }
                return false;
            };
            world.entity_mut(entity).insert(going);
            if !was {
                show(world, entity, true);
            }
            going
        }
    };
    let near = feet(world, going.from).is_some_and(|at| flat(body.at, [at[0], at[2]]) < SPRINTS_WITHIN);
    let walks = sapling_data::husbandry(kind).map_or(0.3, |rules| rules.walks);
    let slowed = crate::creatures::strolling::last_block_share(&body, going.spot);
    legs::walk(world, entity, layer, &body, going.spot, walks * if near { SPRINT } else { 1.0 } * slowed, 0.0);
    true
}

fn flat(at: [f32; 3], to: [f32; 2]) -> f32 {
    (to[0] - at[0]).hypot(to[1] - at[2])
}

fn feet(world: &World, entity: Entity) -> Option<[f32; 3]> {
    world.get::<Position>(entity).map(|position| position.feet).or_else(|| world.get::<Body>(entity).map(|body| body.at))
}

/// The nearest player or feared mob whose box meets the fox's grown by [`SEES`], and its feet.
fn threat(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> Option<(Entity, [f32; 3])> {
    let trusted = world.get::<Trusts>(entity).map(|trusts| trusts.0.clone()).unwrap_or_default();
    let meets = |at: [f32; 3], size: [f32; 2]| {
        let reach = (body.size[0] + size[0]) * 0.5 + SEES;
        (at[0] - body.at[0]).abs() < reach && (at[2] - body.at[2]).abs() < reach && at[1] < body.at[1] + body.size[1] + SEES && at[1] + size[1] > body.at[1] - SEES
    };
    let players = in_world(world).into_iter().filter(|player| {
        let unique = world.get::<crate::tags::Marks>(*player).map(|marks| marks.unique_id);
        world.get::<InLayer>(*player).map(|on| on.0) == Some(layer)
            && world.get::<Mode>(*player).is_some_and(|mode| mode.0 != GameMode::Spectator)
            && !world.get::<Motion>(*player).is_some_and(|motion| motion.pose.sneaking)
            && !unique.is_some_and(|unique| trusted.contains(&unique))
    });
    let players: Vec<(Entity, [f32; 3])> = players.filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).filter(|(_, at)| meets(*at, PLAYER_BOX)).collect();
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>();
    let feared = mobs
        .iter(world)
        .filter(|(_, on, kind, other)| on.0 == layer && meets(other.at, other.size) && sapling_data::mob_families(&kind.0).iter().any(|family| FEARED.contains(family)))
        .map(|(mob, _, _, other)| (mob, other.at));
    let far = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>();
    players.into_iter().chain(feared).min_by(|a, b| far(a.1).total_cmp(&far(b.1)))
}

/// A cell's middle up to [`AWAY`] across, standable at the fox's height, on the far side of the fox
/// from `threat` and out of its sight: BDS's foxes never ran past the player (bds-ar2sneak.txt).
fn spot_away(world: &mut World, layer: Entity, body: &Body, threat: [f32; 3]) -> Option<[f32; 2]> {
    let tries: Vec<[f32; 2]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..TRIES).map(|_| [body.at[0] + (dice.fraction() * 2.0 - 1.0) * AWAY, body.at[2] + (dice.fraction() * 2.0 - 1.0) * AWAY]).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    let (from, reach) = ([body.at[0] - threat[0], body.at[2] - threat[2]], (body.size[0] + PLAYER_BOX[0]) * 0.5 + SEES);
    let suits = |[x, z]: [f32; 2]| {
        let away = (x - body.at[0]) * from[0] + (z - body.at[2]) * from[1] > 0.0;
        let unseen = (x - threat[0]).abs() >= reach || (z - threat[2]).abs() >= reach;
        away && unseen && paths::is_standable(level, [x.floor() as i32, body.at[1].floor() as i32, z.floor() as i32])
    };
    tries.into_iter().map(|[x, z]| [x.floor() + 0.5, z.floor() + 0.5]).find(|spot| suits(*spot))
}

fn show(world: &mut World, entity: Entity, fleeing: bool) {
    let Some(id) = world.get::<crate::components::NetId>(entity).map(|id| id.0) else { return };
    let flags = REST | if fleeing { FLEEING } else { 0 };
    let item = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(flags)) };
    let packet = sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 });
    shown::tell(world, entity, vec![packet]);
}
