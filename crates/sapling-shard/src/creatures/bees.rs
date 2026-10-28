//! Bees, by their pack's goals and as BDS's flew in the `keephive` and `bees` drills
//! (research/captures/bds-keephive.txt, bds-bees.txt). One flies as an allay does (wings.rs) but
//! pushed 0.98 x 0.15² a tick. Without pollen, by day and dry, it looks for a flower within 6
//! across and 4 up or down with even odds a tick, flies to it, and after 20 ticks there carries
//! pollen. With pollen, or by night or in rain, it flies to its hive (the one it last came out
//! of), or the nearest within 16 across and 10 up or down, and goes in (hives.rs). A bee that
//! finds no flower in 180 s goes home. Hit, it goes for its hitter for 25 s with every bee within
//! 20, as one within 16 of a broken hive goes for the nearest player within 10; its sting
//! poisons on normal (10 s) and hard (18 s), and it dies 10 to 60 s after.
// TODO: a bee with pollen grows no crops; it keeps no 22-block bound about its hive; a full hive
// does not send it looking for another; its pollen and home are not kept with it in its column;
// it bobs up and down as BDS's did not.

mod hives;

use acacia_proto::types::{
    MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain,
};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, Weather};

use super::{hunting, wings};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick};
use crate::entities::Kind;

use hives::is_hive;
pub(crate) use hives::{broken as hive_broken, taken as bees_in};

pub(super) const KIND: &str = "minecraft:bee";
/// The blocks it takes pollen from (`look_for_food`'s `target_blocks`).
const FLOWERS: &[&str] = &[
    "poppy", "blue_orchid", "allium", "azure_bluet", "red_tulip", "orange_tulip", "white_tulip", "pink_tulip", "oxeye_daisy", "cornflower", "lily_of_the_valley", "dandelion", "wither_rose", "sunflower",
    "lilac", "rose_bush", "peony", "flowering_azalea", "azalea_leaves_flowered", "mangrove_propagule", "pitcher_plant", "torchflower", "cherry_leaves", "pink_petals", "open_eyeblossom", "wildflowers",
    "cactus_flower", "chorus_flower", "spore_blossom",
];
/// How far across and up or down it looks for a flower, and the odds a tick that it looks.
const FLOWERS_WITHIN: [i32; 2] = [6, 4];
const LOOKS: f32 = 0.5;
/// It takes pollen this near the flower's spot, over this many ticks; the spot is this far over
/// the flower's bottom.
const AT_FLOWER: f32 = 1.0;
const POLLINATES: u32 = 20;
const NECTAR: &str = "minecraft:has_nectar";
const OVER_FLOWER: f32 = 0.25;
/// Ticks it looks for flowers before it goes home (`find_flower_timeout`), and how far it looks
/// for a hive.
const GIVES_UP: u64 = 3600;
const HIVES_WITHIN: [i32; 2] = [16, 10];
/// It goes into a hive this near its middle (`go_home`'s `goal_radius`): BDS's three bees went in
/// 0.75, 1.06 and 1.2 from it (research/captures/bds-keephive-world.txt). `find_hive`'s 0.633
/// cannot be reached beside a hive.
const AT_HOME: f32 = 1.2;
/// Ticks from its sting to its death, the least and the most.
const PERISHES: [u32; 2] = [200, 1200];
/// Its anger reaches bees this near; a broken hive angers those this near it at players this near them.
const ANGER_SPREADS: f32 = 20.0;
const HIVE_SENSES: f32 = 16.0;
const PICKS_PLAYER: f32 = 10.0;
/// Poison its sting gives on normal and hard.
const POISONS: [u32; 2] = [200, 360];
/// The sky darkens this far by night (`sleep::rest`'s night).
const NIGHT: u8 = 4;
/// It flies at this height over its prey's feet, where its sting reaches.
const AIMS_OVER_FEET: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Bee {
    home: Option<BlockPos>,
    nectar: bool,
    /// The flower it flies to, and the ticks it has been at it.
    flower: Option<BlockPos>,
    stayed: u32,
    /// Going home without pollen: its flowers ran out.
    homing: bool,
    /// The tick its search for flowers or a hive began.
    since: u64,
    /// It stung: the tick it dies in.
    perishes: Option<u64>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        let since = world.resource::<Tick>().0;
        world.entity_mut(entity).insert(Bee { since, ..Bee::default() });
    }
}

/// A bee just out of the hive at `pos`, its home now.
fn came_home(world: &mut World, bee: Entity, pos: BlockPos) {
    let since = world.resource::<Tick>().0;
    if let Some(mut kept) = world.get_mut::<Bee>(bee) {
        *kept = Bee { home: Some(pos), since, ..Bee::default() };
    }
}

/// Whether bees fly out in `layer` now: by day and dry (`shelter_detection`).
fn may_fly(world: &World, layer: Entity) -> bool {
    crate::weather::of(world, layer) == Weather::Clear && crate::light::darken(world, layer) < NIGHT
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

fn middle(pos: BlockPos) -> [f32; 3] {
    pos.map(|c| c as f32 + 0.5)
}

/// The cells about `at` within `[across, up]`, nearest first, that `takes`.
fn near(level: &sapling_world::World, at: [f32; 3], [across, up]: [i32; 2], takes: impl Fn(&str, u32, BlockPos) -> bool) -> Vec<BlockPos> {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let mut found: Vec<BlockPos> = (-across..=across)
        .flat_map(|dx| (-up..=up).flat_map(move |dy| (-across..=across).map(move |dz| [x + dx, y + dy, z + dz])))
        .filter(|&[bx, by, bz]| level.in_height(by) && level.is_loaded(sapling_world::ChunkPos::of_block(bx, bz)))
        .filter(|&pos| {
            let id = level.block(pos[0], pos[1], pos[2]);
            level.registry().get(id).is_some_and(|state| takes(state.name, level.liquid(pos[0], pos[1], pos[2]), pos))
        })
        .collect();
    found.sort_by(|a, b| distance(at, middle(*a)).total_cmp(&distance(at, middle(*b))));
    found
}

fn is_flower(name: &str, liquid: u32, level: &sapling_world::World) -> bool {
    let dry = level.registry().get(liquid).is_none_or(|state| state.is_air());
    dry && name.strip_prefix("minecraft:").is_some_and(|bare| FLOWERS.contains(&bare))
}

/// Before its move: at its prey, its flower or its hive, else about as wings.rs has it.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, body: &Body, prey: Option<[f32; 3]>) -> bool {
    if let Some(feet) = prey {
        return wings::fly_at(world, entity, KIND, [feet[0], feet[1] + AIMS_OVER_FEET, feet[2]]);
    }
    let Some(bee) = world.get::<Bee>(entity) else { return wings::steer(world, entity, layer, KIND) };
    let goes_home = bee.nectar || bee.homing || !may_fly(world, layer);
    match goes_home {
        true => home(world, entity, layer, body),
        false => forage(world, entity, layer, body),
    }
}

fn forage(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(bee) = world.get::<Bee>(entity) else { return false };
    let (flower, since) = (bee.flower, bee.since);
    if now >= since + GIVES_UP && flower.is_none() {
        if let Some(mut bee) = world.get_mut::<Bee>(entity) {
            (bee.homing, bee.since) = (true, now);
        }
        return wings::steer(world, entity, layer, KIND);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let still = flower.filter(|pos| level.registry().get(level.block(pos[0], pos[1], pos[2])).is_some_and(|state| is_flower(state.name, level.liquid(pos[0], pos[1], pos[2]), level)));
    let flower = match still {
        Some(flower) => Some(flower),
        None if world.get_resource_or_insert_with(Dice::default).fraction() < LOOKS => {
            let level = &world.get::<Layer>(layer).expect("read above").world;
            let flowers = near(level, body.at, FLOWERS_WITHIN, |name, liquid, _| is_flower(name, liquid, level));
            let pick = (!flowers.is_empty()).then(|| world.get_resource_or_insert_with(Dice::default).roll(flowers.len() as u32) as usize);
            pick.map(|pick| flowers[pick])
        }
        None => None,
    };
    let Some(flower) = flower else {
        if let Some(mut bee) = world.get_mut::<Bee>(entity) {
            (bee.flower, bee.stayed) = (None, 0);
        }
        return wings::steer(world, entity, layer, KIND);
    };
    let spot = [flower[0] as f32 + 0.5, flower[1] as f32 + OVER_FLOWER, flower[2] as f32 + 0.5];
    let there = distance(body.at, spot) < AT_FLOWER;
    let Some(mut bee) = world.get_mut::<Bee>(entity) else { return false };
    bee.flower = Some(flower);
    bee.stayed = if there { bee.stayed + 1 } else { 0 };
    let pollinated = bee.stayed >= POLLINATES;
    if pollinated {
        (bee.nectar, bee.flower, bee.stayed, bee.since) = (true, None, 0, now);
        super::properties::set(world, entity, NECTAR, 1, nectar_sounds());
    }
    wings::fly_at(world, entity, KIND, spot)
}

/// The sounds BDS sent with a bee's `has_nectar` (bds-beesout.txt): those of its pollen group.
fn nectar_sounds() -> Vec<MetadataDictionaryItem> {
    let interval = |key| MetadataDictionaryItem { key, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(0.0)) };
    vec![interval(Key::AmbientSoundInterval), interval(Key::AmbientSoundIntervalRange), super::properties::ambient_sound("ambient")]
}

fn home(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let Some(bee) = world.get::<Bee>(entity) else { return false };
    let (own, nectar) = (bee.home, bee.nectar);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let stands = |pos: &BlockPos| level.registry().get(level.block(pos[0], pos[1], pos[2])).is_some_and(|state| is_hive(state.name));
    let hive = own.filter(stands).or_else(|| near(level, body.at, HIVES_WITHIN, |name, _, _| is_hive(name)).first().copied());
    let Some(hive) = hive else {
        // One whose flowers ran out looks for them again.
        let now = world.resource::<Tick>().0;
        if let Some(mut bee) = world.get_mut::<Bee>(entity).filter(|bee| bee.homing) {
            (bee.homing, bee.since) = (false, now);
        }
        return wings::steer(world, entity, layer, KIND);
    };
    if distance(body.at, middle(hive)) <= AT_HOME && hives::enter(world, entity, layer, hive, nectar) {
        return true;
    }
    wings::fly_at(world, entity, KIND, middle(hive))
}

/// A hunter swung at `prey` (hit or not, as `bitten`): a bee's sting poisons on normal and hard,
/// and the bee dies [`PERISHES`] after, hunting no more.
pub(super) fn stung(world: &mut World, bee: Entity, prey: Entity, bitten: bool) {
    if world.get::<Bee>(bee).is_none() {
        return;
    }
    let poison = match world.resource::<Settings>().0.difficulty {
        Difficulty::Normal => Some(POISONS[0]),
        Difficulty::Hard => Some(POISONS[1]),
        _ => None,
    };
    if let Some(ticks) = poison.filter(|_| bitten) {
        crate::vitals::afflict(world, prey, "poison", ticks);
    }
    let after = PERISHES[0] + world.get_resource_or_insert_with(Dice::default).roll(PERISHES[1] - PERISHES[0] + 1);
    let now = world.resource::<Tick>().0;
    if let Some(mut bee) = world.get_mut::<Bee>(bee) {
        bee.perishes = Some(now + u64::from(after));
    }
}

/// Whether the bee stung and hunts no more.
pub(super) fn is_spent(world: &World, bee: Entity) -> bool {
    world.get::<Bee>(bee).is_some_and(|bee| bee.perishes.is_some())
}

/// Bees within `reach` of `at` in `layer`, but `but`, that have not stung.
fn bees_near(world: &mut World, layer: Entity, at: [f32; 3], reach: f32, but: Option<Entity>) -> Vec<(Entity, u64, Body)> {
    world
        .query::<(Entity, &InLayer, &NetId, &Kind, &Body, &Bee)>()
        .iter(world)
        .filter(|(entity, in_layer, _, kind, body, bee)| in_layer.0 == layer && kind.0 == KIND && Some(*entity) != but && bee.perishes.is_none() && distance(body.at, at) <= reach)
        .map(|(entity, _, id, _, body, _)| (entity, id.0, *body))
        .collect()
}

/// A hunter took `player` for its prey: if a bee, so does every bee within 20 of it (`broadcastAnger`).
pub(super) fn spread_anger(world: &mut World, bee: Entity, layer: Entity, at: [f32; 3], player: Entity) {
    if world.get::<Bee>(bee).is_none() {
        return;
    }
    for (other, id, body) in bees_near(world, layer, at, ANGER_SPREADS, Some(bee)) {
        hunting::provoke(world, other, layer, id, KIND, &body, player);
    }
}

/// The hive at `pos` was broken: each bee within 16 goes for the nearest player within 10 of it.
fn disturbed(world: &mut World, layer: Entity, pos: BlockPos) {
    for (bee, id, body) in bees_near(world, layer, middle(pos), HIVE_SENSES, None) {
        let players = crate::components::in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && crate::vitals::is_mortal(world, *player));
        let nearest = players.filter_map(|player| Some((player, distance(world.get::<Position>(player)?.feet, body.at)))).filter(|(_, far)| *far <= PICKS_PLAYER).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((player, _)) = nearest {
            hunting::provoke(world, bee, layer, id, KIND, &body, player);
        }
    }
}

/// Bees whose time after a sting is up die, and hives let their bees out.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<u64> = world.query::<(&NetId, &Bee)>().iter(world).filter(|(_, bee)| bee.perishes.is_some_and(|at| now >= at)).map(|(id, _)| id.0).collect();
    for id in due {
        super::kill(world, sapling_api::EntityId(id));
    }
    hives::tick(world);
}
