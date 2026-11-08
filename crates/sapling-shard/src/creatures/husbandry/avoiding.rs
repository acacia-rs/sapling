//! `behavior.avoid_mob_type` (the packs): what a mob fears near sends it to a spot further from it,
//! at 1.5 times its pace while within 7 of it and its pace beyond, with `FlagsExtended` bit 22 on
//! till it gets there.
//! - A fox fears a player who is not sneaking and whom it does not trust, a wolf or a polar bear,
//!   whose box meets its own grown 3 each way, and looks for a spot up to 16 across. Foxes 3 and 3.6
//!   off fled, 4 and 5.8 off did not; flights ran 2 to 10 blocks and ended at a cell's middle
//!   (research/captures/bds-ar2fox.txt, bds-ar2sneak.txt). A bred fox trusts the players who fed its
//!   parents (`minecraft:trust`, `trusting_fox`).
//! - A worried panda fears any player or mob but a panda within 16 (its pack's `max_dist`), and looks
//!   up to 20 across (`max_flee`): held worried pandas 4 to 16 from a held cow all showed bit 22 the
//!   tick they came (research/captures/bds-g14panda.txt); a free one sprinted at 1.5 times its pace
//!   within about 7 and walked beyond (bds-pandas-findings.md).
// TODO: a wild cat's and an ocelot's flight from players (walk 0.8, sprint 1.33) is not built; trust
// is not saved with the fox; whether a worried panda fears creative players was not drilled.

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
const PANDA: &str = "minecraft:panda";
const FEARED: [&str; 2] = ["wolf", "polarbear"];
const SPRINTS_WITHIN: f32 = 7.0;
const SPRINT: f32 = 1.5;
/// Where it flees to is looked for this many times.
const TRIES: usize = 10;
const ARRIVED: f32 = 0.6;
const GIVES_UP: u64 = 200;
const PLAYER_BOX: [f32; 2] = [0.6, 1.8];
/// `FlagsExtended` at rest, and the bit on while it flees (acacia-proto names it `AVOIDING_BLOCK`).
const REST: i64 = 1 << 25;
const FLEEING: i64 = 1 << 22;

/// How a threat is seen: its box meeting the mob's grown so much each way, or its feet within so far.
#[derive(Clone, Copy)]
enum Sees {
    Grown(f32),
    Within(f32),
}

/// Whom a kind fears, how it sees them and how far across it looks for a spot away.
struct Avoids {
    fears: fn(&str) -> bool,
    trusts: bool,
    sees: Sees,
    away: f32,
}

fn fox_fears(kind: &str) -> bool {
    sapling_data::mob_families(kind).iter().any(|family| FEARED.contains(family))
}

fn not_panda(kind: &str) -> bool {
    kind != PANDA
}

const FOX_AVOIDS: Avoids = Avoids { fears: fox_fears, trusts: true, sees: Sees::Grown(3.0), away: 16.0 };
const WORRIED: Avoids = Avoids { fears: not_panda, trusts: false, sees: Sees::Within(16.0), away: 20.0 };

fn avoids(world: &World, entity: Entity, kind: &str) -> Option<&'static Avoids> {
    match kind {
        FOX => Some(&FOX_AVOIDS),
        PANDA if crate::creatures::genes::is_worried(world, entity) => Some(&WORRIED),
        _ => None,
    }
}

/// Where a fleeing mob runs to, from whom, and the tick it gives up in.
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

/// Before the move of a mob that avoids others: it flees what it fears near, or keeps fleeing.
/// False when it flees nothing.
pub(super) fn flee(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(avoids) = avoids(world, entity, kind) else { return false };
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let now = world.resource::<Tick>().0;
    let going = world.get::<Avoiding>(entity).copied().filter(|going| now < going.until && flat(body.at, going.spot) >= ARRIVED);
    let going = match going {
        Some(going) => going,
        None => {
            let was = world.entity_mut(entity).take::<Avoiding>().is_some();
            let found = threat(world, entity, layer, &body, avoids).and_then(|(from, at)| spot_away(world, layer, &body, at, avoids).map(|spot| Avoiding { from, spot, until: now + GIVES_UP }));
            let Some(going) = found else {
                if was {
                    shown::flags_extended(world, entity, REST);
                }
                return false;
            };
            world.entity_mut(entity).insert(going);
            if !was {
                shown::flags_extended(world, entity, REST | FLEEING);
            }
            going
        }
    };
    let near = feet(world, going.from).is_some_and(|at| flat(body.at, [at[0], at[2]]) < SPRINTS_WITHIN);
    let walks = sapling_data::husbandry(kind).map_or(0.3, |rules| rules.walks) * crate::creatures::genes::pace(world, entity);
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

/// Whether a threat at `at`, `size` big, is seen by `body` as `sees` has it.
fn seen(body: &Body, sees: Sees, at: [f32; 3], size: [f32; 2]) -> bool {
    match sees {
        Sees::Grown(grown) => {
            let reach = (body.size[0] + size[0]) * 0.5 + grown;
            (at[0] - body.at[0]).abs() < reach && (at[2] - body.at[2]).abs() < reach && at[1] < body.at[1] + body.size[1] + grown && at[1] + size[1] > body.at[1] - grown
        }
        Sees::Within(far) => (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>() <= far * far,
    }
}

/// The nearest player or feared mob the mob sees, and its feet.
fn threat(world: &mut World, entity: Entity, layer: Entity, body: &Body, avoids: &Avoids) -> Option<(Entity, [f32; 3])> {
    let trusted = world.get::<Trusts>(entity).map(|trusts| trusts.0.clone()).unwrap_or_default();
    let players = in_world(world).into_iter().filter(|player| {
        let unique = world.get::<crate::tags::Marks>(*player).map(|marks| marks.unique_id);
        world.get::<InLayer>(*player).map(|on| on.0) == Some(layer)
            && world.get::<Mode>(*player).is_some_and(|mode| mode.0 != GameMode::Spectator)
            && !(avoids.trusts && world.get::<Motion>(*player).is_some_and(|motion| motion.pose.sneaking))
            && !unique.is_some_and(|unique| trusted.contains(&unique))
    });
    let players: Vec<(Entity, [f32; 3])> = players.filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).filter(|(_, at)| seen(body, avoids.sees, *at, PLAYER_BOX)).collect();
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>();
    let feared = mobs
        .iter(world)
        .filter(|(mob, on, kind, other)| *mob != entity && on.0 == layer && seen(body, avoids.sees, other.at, other.size) && (avoids.fears)(&kind.0))
        .map(|(mob, _, _, other)| (mob, other.at));
    let far = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>();
    players.into_iter().chain(feared).min_by(|a, b| far(a.1).total_cmp(&far(b.1)))
}

/// A cell's middle up to the kind's `away` across, standable at the mob's height, on the far side of
/// it from `threat` and out of its sight: BDS's foxes never ran past the player (bds-ar2sneak.txt).
fn spot_away(world: &mut World, layer: Entity, body: &Body, threat: [f32; 3], avoids: &Avoids) -> Option<[f32; 2]> {
    let tries: Vec<[f32; 2]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..TRIES).map(|_| [body.at[0] + (dice.fraction() * 2.0 - 1.0) * avoids.away, body.at[2] + (dice.fraction() * 2.0 - 1.0) * avoids.away]).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    let from = [body.at[0] - threat[0], body.at[2] - threat[2]];
    let reach = match avoids.sees {
        Sees::Grown(grown) => (body.size[0] + PLAYER_BOX[0]) * 0.5 + grown,
        Sees::Within(far) => far,
    };
    let suits = |[x, z]: [f32; 2]| {
        let away = (x - body.at[0]) * from[0] + (z - body.at[2]) * from[1] > 0.0;
        let unseen = (x - threat[0]).abs() >= reach || (z - threat[2]).abs() >= reach;
        away && unseen && paths::is_standable(level, [x.floor() as i32, body.at[1].floor() as i32, z.floor() as i32])
    };
    tries.into_iter().map(|[x, z]| [x.floor() + 0.5, z.floor() + 0.5]).find(|spot| suits(*spot))
}
