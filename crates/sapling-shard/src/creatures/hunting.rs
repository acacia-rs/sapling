//! Mobs that hunt players, as BDS's were seen to (docs/research/mob-attacks.md): one picks
//! the nearest player it could kill, walks straight at it and hits it when it is within reach.
// TODO: a spider keeps the prey it took in the dark when day comes; prey once seen is kept behind a
// wall; a player and a hunter do not push each other aside; a spider hunts by day too and none holds what it hits with; peaceful worlds keep their
// monsters, idle; viewers far off are told every move, where BDS told one of every other; a piglin
// hunts a player in gold too, and a polar bear does not rear before its hit.

mod strike;

pub(super) use strike::strike;

use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::{blast, legs, paths};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick, in_world};
use crate::{entities, late_sounds, vitals};

/// Ticks between two looks for a player.
const LOOKS: u64 = 10;
/// A way round is looked for again after this many ticks, and a cell of it is reached this near
/// its middle.
const REPLANS: u64 = 10;
const ARRIVED: f32 = 0.35;
/// An iron golem's hit throws this fast away and up (research/captures/bds-golem.txt).
const FLINGS: [f32; 2] = [0.52, 0.4288];
/// An archer walks at its prey for this many ticks after taking it (BDS's went 1.7 blocks, from
/// 10 off and from 4), then stands while it is this near.
const NEARS: u64 = 19;
const SHOOTS_FROM: f32 = 10.0;
/// One that blows up lights its fuse this near its prey, and puts it out this far from it.
const IGNITES: f32 = 2.5;
const GOES_OUT: f32 = 6.0;
/// A hunter's box reaches this much further across for its hit, and it hits once in this many ticks.
const REACH: f32 = 0.8;
const COOLDOWN: u64 = 20;
/// The speed its hit throws a player at, away from it and up.
const THROW: f32 = 0.4;
/// Nearer than this to its prey it stands: their boxes touch.
const TOUCHING: f32 = 0.6;
const PLAYER: [f32; 2] = [0.6, 1.8];
/// How far apart a line of sight is looked at.
const GLANCE: f32 = 0.25;
/// Its sounds come from this much of its height up: its growl, and its hit.
const HEAD: f32 = 0.85;
const MIDDLE: f32 = 0.5;
/// Kinds heard `angry` a tick after their growl as they take prey (research/captures/bds-beasts.txt).
const ANGERED: &[&str] = &["minecraft:hoglin", "minecraft:zoglin"];

#[derive(Component, Default)]
pub(super) struct Hunter {
    prey: Option<Entity>,
    /// The tick it took its prey in.
    since: u64,
    pub(super) ready: u64,
    /// The shots left of a volley, and the tick the next goes in.
    pub(super) volley: Option<(u32, u64)>,
    /// The cells of its way round what stands before its prey, and the tick it found them in.
    way: Vec<[i32; 3]>,
    planned: u64,
}

/// Makes the mob a hunter if its kind is one.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).hunts.is_some() {
        world.entity_mut(entity).insert(Hunter::default());
    }
}

pub(super) fn sound(name: &str, id: u64, kind: &str, body: &Body, up: f32) -> bytes::Bytes {
    let position = Vec3f { x: body.at[0], y: body.at[1] + up * body.size[1], z: body.at[2] };
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// The feet of the player if a hunter in `layer` at `from` that sees `sight` far may hunt it.
fn seen(world: &World, player: Entity, layer: Entity, from: [f32; 3], sight: f32) -> Option<[f32; 3]> {
    let feet = world.get::<Position>(player)?.feet;
    (world.get::<InLayer>(player)?.0 == layer && vitals::is_mortal(world, player) && distance(from, feet) <= sight).then_some(feet)
}

/// Whether no full block, glass too, is between a hunter's eyes and the player's
/// (research/captures/bds-around.txt).
pub(super) fn in_sight(world: &World, layer: Entity, body: &Body, feet: [f32; 3]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    sees(level, [body.at[0], body.at[1] + HEAD * body.size[1], body.at[2]], [feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]])
}

/// Whether no full block, glass too, is on the line from `from` to `to`.
pub(crate) fn sees(level: &sapling_world::World, from: [f32; 3], to: [f32; 3]) -> bool {
    let glances = (distance(from, to) / GLANCE).ceil().max(1.0);
    let hides = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube());
    (0..=glances as usize).all(|glance| !hides(std::array::from_fn(|axis| (from[axis] + (to[axis] - from[axis]) * glance as f32 / glances).floor() as i32)))
}

/// The nearest player within `sight` of a mob in `layer` that it sees and may hurt, and its feet.
pub(super) fn nearest(world: &World, layer: Entity, body: &Body, sight: f32) -> Option<(Entity, [f32; 3])> {
    let near = |(_, feet): &(Entity, [f32; 3])| distance(body.at, *feet);
    in_world(world).into_iter().filter_map(|player| Some((player, seen(world, player, layer, body.at, sight)?))).filter(|(_, feet)| in_sight(world, layer, body, *feet)).min_by(|a, b| near(a).total_cmp(&near(b)))
}

/// The hunter `id` has the entity `target` as its prey, or none for 0.
fn targets(id: u64, target: i64) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::TargetEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(target)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Where the hunter's prey stands, if it has any.
pub(super) fn prey_at(world: &World, entity: Entity) -> Option<[f32; 3]> {
    Some(world.get::<Position>(world.get::<Hunter>(entity)?.prey?)?.feet)
}

/// A hunter that died lets its prey go, which its viewers are told (bds-bite.txt).
pub(super) fn let_go(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body) {
    if world.get_mut::<Hunter>(entity).is_some_and(|mut hunter| hunter.prey.take().is_some()) {
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![targets(id, 0)]);
    }
}

/// Keeps or picks the hunter's prey and tells its viewers of a change; where the prey is.
fn pick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<[f32; 3]> {
    let now = world.resource::<Tick>().0;
    let had = world.get::<Hunter>(entity)?.prey;
    let sight = sapling_data::mob_traits(kind).hunts?.sees;
    let hunts = world.resource::<Settings>().0.difficulty != Difficulty::Peaceful;
    let since = world.get::<Hunter>(entity)?.since;
    let calm = sapling_data::mob_traits(kind).hunts?.calms.is_some_and(|calms| now >= since + u64::from(calms));
    let mut found = had.filter(|_| hunts && !calm).and_then(|prey| Some((prey, seen(world, prey, layer, body.at, sight)?)));
    // A spider under the sun by day took none on BDS; under a roof or by night it did
    // (research/captures/bds-spiders.txt).
    let head = [body.at[0], body.at[1] + body.size[1], body.at[2]].map(|c| c.floor() as i32);
    let dazzled = sapling_data::mob_traits(kind).hunts?.by_dark && super::harm::sun_on(world, layer, head);
    let looks = !sapling_data::mob_traits(kind).hunts?.provoked && !dazzled;
    if found.is_none() && hunts && looks && (now + id).is_multiple_of(LOOKS) {
        found = nearest(world, layer, body, sight);
    }
    let prey = found.map(|(prey, _)| prey);
    if prey != had {
        take(world, entity, layer, id, kind, body, prey);
    }
    found.map(|(_, feet)| feet)
}

/// Gives the hunter `prey` and tells its viewers, with a growl if it is some.
fn take(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<Entity>) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let mut hunter = world.get_mut::<Hunter>(entity)?;
    (hunter.prey, hunter.since) = (prey, now);
    let target = prey.and_then(|prey| world.get::<NetId>(prey)).map_or(0, |id| id.0 as i64);
    let cell = body.at.map(|c| c.floor() as i32);
    let mut told = vec![targets(id, target)];
    told.extend(super::bullets::peek(world, layer, id, kind, body.at, prey.is_some()));
    entities::tell_viewers(world, entity, layer, cell, told);
    if prey.is_some() {
        late_sounds::play_later(world, 1, layer, cell, sound("mob.warning", id, kind, body, HEAD));
    }
    if prey.is_some() && ANGERED.contains(&kind) {
        late_sounds::play_later(world, 2, layer, cell, sound("angry", id, kind, body, HEAD));
    }
    Some(())
}

/// A player hit the mob: one that hunts none but its hitter, or a spider that hunts by dark
/// (research/captures/bds-provoke.txt, bds-provoke2.txt), takes it for its prey at once, if it may be hunted.
pub(super) fn provoke(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, player: Entity) {
    let fights = super::genes::fights(world, entity) && !super::bees::is_spent(world, entity);
    let Some(hunts) = sapling_data::mob_traits(kind).hunts.filter(|hunts| (hunts.provoked || hunts.by_dark) && fights) else { return };
    let hunted = world.resource::<Settings>().0.difficulty != Difficulty::Peaceful && seen(world, player, layer, body.at, hunts.sees).is_some() && !super::husbandry::is_owner(world, entity, player);
    if hunted && world.get::<Hunter>(entity).is_some_and(|hunter| hunter.prey != Some(player)) {
        take(world, entity, layer, id, kind, body, Some(player));
        // BDS's first hit back came a second or more after, a llama's spit as long as it aims.
        let now = world.resource::<Tick>().0;
        world.get_mut::<Hunter>(entity).expect("a hunter, above").ready = now + hunts.aims.map_or(COOLDOWN, u64::from);
        super::bees::spread_anger(world, entity, layer, body.at, player);
    }
}

/// Where a hunter walks to this tick, and how near it stops: straight at its prey where the
/// floor lets it, else to the next cell of a way round, looked for anew every [`REPLANS`] ticks.
fn heads(world: &mut World, entity: Entity, layer: Entity, body: &Body, feet: [f32; 3]) -> ([f32; 2], f32) {
    let now = world.resource::<Tick>().0;
    let straight = ([feet[0], feet[2]], TOUCHING);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return straight };
    if paths::is_clear(level, body.at, feet) {
        return straight;
    }
    let stale = world.get::<Hunter>(entity).is_none_or(|hunter| hunter.way.is_empty() || now >= hunter.planned + REPLANS);
    let found = stale.then(|| paths::route(level, body.at.map(|c| c.floor() as i32), feet.map(|c| c.floor() as i32)).unwrap_or_default());
    let Some(mut hunter) = world.get_mut::<Hunter>(entity) else { return straight };
    if let Some(way) = found {
        (hunter.way, hunter.planned) = (way, now);
    }
    let middle = |cell: &[i32; 3]| [cell[0] as f32 + 0.5, cell[2] as f32 + 0.5];
    while hunter.way.first().is_some_and(|cell| (middle(cell)[0] - body.at[0]).hypot(middle(cell)[1] - body.at[2]) < ARRIVED) {
        hunter.way.remove(0);
    }
    hunter.way.first().map_or(straight, |cell| (middle(cell), 0.0))
}

/// Before the hunter's move: it walks at its prey, if it has any.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let (Some(body), Some(speed)) = (world.get::<Body>(entity).copied(), sapling_data::mob_traits(kind).hunts.map(|hunts| hunts.speed)) else { return false };
    let prey = pick(world, entity, layer, id, kind, &body);
    match sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunts.fights) {
        Some("spells") => return super::spells::steer(world, entity, layer, id, kind, &body, prey),
        Some("leaps") => return super::breezes::steer(world, entity, layer, id, kind, &body, prey),
        Some("fireballs") => return super::ghasts::steer(world, entity, layer, id, kind, &body, prey),
        Some("swims") => return super::dolphins::steer(world, entity, layer, &body, prey),
        Some("hops") => return super::slimes::steer(world, entity, prey),
        Some("flies") => return super::bees::steer(world, entity, layer, &body, prey),
        Some("lasers") => return super::guardians::steer(world, entity, layer, id, kind, &body, prey),
        Some("charges") => {
            let prey = prey.and(world.get::<Hunter>(entity).and_then(|hunter| hunter.prey));
            return super::vexes::steer(world, entity, layer, id, kind, &body, prey);
        }
        _ => {}
    }
    let Some(feet) = prey else { return blast::is_lit(world, entity) };
    // A lit creeper stands where it is, and an archer within its range.
    let now = world.resource::<Tick>().0;
    let aims = sapling_data::mob_traits(kind).hunts.is_some_and(|hunts| hunts.shoots.is_some() && !hunts.closes) && world.get::<Hunter>(entity).is_some_and(|hunter| now >= hunter.since + NEARS);
    let stands = blast::is_lit(world, entity) || (aims && distance(feet, body.at) < SHOOTS_FROM);
    match stands {
        true => legs::rest(world, entity),
        false => {
            let (to, near) = heads(world, entity, layer, &body, feet);
            legs::walk(world, entity, layer, &body, to, speed, near);
        }
    }
    true
}

/// What a mob's hit of `harm` on normal costs on the world's difficulty; none on peaceful.
pub(crate) fn on_difficulty(world: &World, harm: f32) -> Option<f32> {
    match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => None,
        Difficulty::Easy => Some(harm.min(harm / 2.0 + 1.0)),
        Difficulty::Normal => Some(harm),
        Difficulty::Hard => Some(harm * 1.5),
    }
}

/// The hunter's prey and the tick it took it in.
pub(super) fn quarry(world: &World, entity: Entity) -> Option<(Entity, u64)> {
    let hunter = world.get::<Hunter>(entity)?;
    Some((hunter.prey?, hunter.since))
}

/// The hunter lets its prey go, which its viewers are told; it may take it again at its next look.
pub(super) fn give_up(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    take(world, entity, layer, id, kind, body, None);
}
