//! Bees, by their pack's goals and as BDS's flew in the `keephive` and `bees` drills
//! (research/captures/bds-keephive.txt, bds-bees.txt). One flies as an allay does (wings.rs) but
//! pushed 0.98 x 0.15² a tick. Without pollen, by day and dry, it looks for a flower within 6
//! across and 4 up or down with even odds a tick, flies to it, and after 20 s there carries
//! pollen. With pollen, or by night or in rain, it flies to its hive (the one it last came out
//! of), or the nearest within 16 across and 10 up or down, and goes in (hives.rs), growing crops on
//! the way (crops.rs). Wandering, it keeps within 22 of its hive. A bee that finds no flower in
//! 180 s goes home. Hit, it stings (stings.rs).
// TODO: a full hive does not send it looking for another (`hive_full`, 5 to 20 s); its pollen and
// home are not kept with it in its column; it bobs up and down as BDS's did not.

mod crops;
mod hives;
mod stings;

use acacia_proto::types::{
    MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain,
};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::wings;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, NetId, Tick};

use hives::is_hive;
pub(crate) use hives::{broken as hive_broken, taken as bees_in};
use stings::disturbed;
pub(super) use stings::{is_spent, spread_anger, stung};

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
/// It takes pollen after this many ticks this near the flower's spot, counted while near and kept
/// while it drifts off (`MoveToBlockGoal::tick` 0x5277e80; BDS's bees took 19 to 23 s, bds-g14bee.txt,
/// bds-beesout.txt); the spot is this far over the flower's bottom.
const AT_FLOWER: f32 = 1.0;
const POLLINATES: u32 = 400;
const NECTAR: &str = "minecraft:has_nectar";
const OVER_FLOWER: f32 = 0.25;
/// Ticks it looks for flowers before it goes home (`find_flower_timeout`), and how far it looks
/// for a hive.
const GIVES_UP: u64 = 3600;
const HIVES_WITHIN: [i32; 2] = [16, 10];
/// Wandering, it keeps this near its hive (`minecraft:home`'s `restriction_radius`, `move_towards_home_restriction`).
const HOME_BOUND: f32 = 22.0;
/// It goes into a hive this near its middle (`go_home`'s `goal_radius`): BDS's three bees went in
/// 0.75, 1.06 and 1.2 from it (research/captures/bds-keephive-world.txt). `find_hive`'s 0.633
/// cannot be reached beside a hive.
const AT_HOME: f32 = 1.2;
/// The sky darkens this far by night (`sleep::rest`'s night).
const NIGHT: u8 = 4;
/// It flies at this height over its prey's feet, where its sting reaches.
const AIMS_OVER_FEET: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Bee {
    home: Option<BlockPos>,
    nectar: bool,
    /// Crops its pollen grew.
    grown: u8,
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
    !crate::weather::rains(world, layer) && crate::light::darken(world, layer) < NIGHT
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
    let (nectar, grown) = (bee.nectar, bee.grown);
    let goes_home = bee.nectar || bee.homing || !may_fly(world, layer);
    if nectar {
        let grown = crops::grow(world, layer, body, grown);
        if let Some(mut bee) = world.get_mut::<Bee>(entity) {
            bee.grown = grown;
        }
    }
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
        return wander(world, entity, layer, body);
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
        return wander(world, entity, layer, body);
    };
    let spot = [flower[0] as f32 + 0.5, flower[1] as f32 + OVER_FLOWER, flower[2] as f32 + 0.5];
    let there = distance(body.at, spot) < AT_FLOWER;
    let Some(mut bee) = world.get_mut::<Bee>(entity) else { return false };
    if bee.flower != Some(flower) {
        bee.stayed = 0;
    }
    bee.flower = Some(flower);
    bee.stayed += u32::from(there);
    if bee.stayed >= POLLINATES {
        (bee.nectar, bee.flower, bee.stayed, bee.since) = (true, None, 0, now);
        super::properties::set(world, entity, NECTAR, 1, nectar_sounds());
    }
    wings::fly_at(world, entity, KIND, spot)
}

/// Its random flight, back towards its hive while it is more than [`HOME_BOUND`] off.
fn wander(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let home = world.get::<Bee>(entity).and_then(|bee| bee.home);
    match home.filter(|home| distance(body.at, middle(*home)) > HOME_BOUND) {
        Some(home) => wings::fly_at(world, entity, KIND, middle(home)),
        None => wings::steer(world, entity, layer, KIND),
    }
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

/// Bees whose time after a sting is up die, and hives let their bees out.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<u64> = world.query::<(&NetId, &Bee)>().iter(world).filter(|(_, bee)| bee.perishes.is_some_and(|at| now >= at)).map(|(id, _)| id.0).collect();
    for id in due {
        super::kill(world, sapling_api::EntityId(id));
    }
    hives::tick(world);
}
