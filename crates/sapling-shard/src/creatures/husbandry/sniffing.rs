//! A grown sniffer's sniffing and digging, as the vanilla pack's sniffer.json gives it (docs/
//! research/mob-life.md "Sniffers"): every 400 to 500 seconds it scents for 2 seconds
//! (`timer_flag_1`), then picks open soil within 20 across and 3 up or down ([`TRIES`] a tick), walks to
//! within 2 of it at 1.25 times its pace (searching), and digs 8 to 10 seconds, raising dust 2.25
//! ahead every 4 ticks from 35 ticks in; 6 seconds in it turns up a torchflower seed or a pitcher
//! pod there, and once done it stands up (`timer_flag_2`) and is happy (`timer_flag_3`), each 2 to 5
//! seconds. The client animates by these flags. BDS's digs: research/captures/bds-sniffdig.txt.
//! Its scent's end and `/event ... on_scenting_success` add the search (bds-k2sniff.txt): it looks
//! for a spot each tick until one is found (a single dirt block 5 off took BDS's 14 to 62 ticks, a
//! field one), digs the same spot again if sent again, and walks as long as it needs (27 s seen);
//! one held still gave up after 141 ticks, standing up (`timer_flag_2`) unhappy.
// TODO: the state is not kept in the world folder.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::{Vec3f, EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2 as Flags};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use super::{Raised, shown};
use crate::blocks::Dice;
use crate::components::{Layer, NetId};

/// The extended flags a sniffer is told with besides these: BDS's word after its announcement
/// (research/captures/bds-sniffers.txt).
const BASE: i64 = i64::MIN | 1 << 25;
// BDS's bits sit one above acacia-proto's names of them (as warden.rs's digging; bds-sniffdig.txt).
const SEARCHING: Flags = Flags(1 << 49);
const DIGGING: Flags = Flags(1 << 42);
const TIMER_FLAG_1: Flags = Flags(1 << 51);
const TIMER_FLAG_2: Flags = Flags(1 << 52);
const TIMER_FLAG_3: Flags = Flags(1 << 53);

const SNIFFER: &str = "minecraft:sniffer";
const SOILS: [&str; 9] = [
    "minecraft:dirt", "minecraft:coarse_dirt", "minecraft:grass_block", "minecraft:podzol", "minecraft:dirt_with_roots", "minecraft:moss_block", "minecraft:pale_moss_block", "minecraft:mud", "minecraft:muddy_mangrove_roots",
];
const FINDS: [&str; 2] = ["minecraft:torchflower_seeds", "minecraft:pitcher_pod"];
/// Ticks between scents: the least, and how many more.
const SCENTS_EVERY: (u64, u32) = (8000, 2000);
const SCENTS_FOR: u64 = 40;
/// Columns tried a tick: the pack's 5 retries, each of BDS's land position's 10 candidates, the
/// best of which is soil where any is (an animal's walk target value).
const TRIES: usize = 50;
const ACROSS: i32 = 20;
const UP: i32 = 3;
const NEAR: f32 = 2.0;
const SEARCH_PACE: f32 = 1.25;
/// A searching sniffer that has come less than [`STUCK_WITHIN`] nearer in this many ticks gives up.
const STUCK_FOR: u64 = 140;
const STUCK_WITHIN: f32 = 1.0;
/// Ticks of digging: the least, and how many more; the find comes this far in.
const DIGS_FOR: (u64, u32) = (160, 40);
const FINDS_AT: u64 = 120;
/// Dust from this many ticks into a dig, every [`DUSTS_EVERY`], until the find.
const DUSTS_FROM: u64 = 35;
const DUSTS_EVERY: u64 = 4;
const AHEAD: f32 = 2.25;
/// The find comes this far over the feet.
const FIND_UP: f32 = 0.125;
/// Ticks of standing up after, and of being happy: the least, and how many more.
const AFTER: (u64, u32) = (40, 60);

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Idle,
    Scenting { until: u64 },
    /// Scented, looking each tick for a spot.
    Seeking,
    /// Walking to `target`; at `check` it must have moved [`STUCK_WITHIN`] from `from`.
    Searching { target: BlockPos, check: u64, from: [f32; 3] },
    Digging { started: u64, until: u64 },
    Rising { until: u64 },
}

/// Where a grown sniffer is in its sniffing, the tick it next scents and the tick its joy ends.
#[derive(Component)]
pub(crate) struct Sniffing {
    stage: Stage,
    next: u64,
    happy_until: Option<u64>,
    extended: i64,
}

fn roll(world: &mut World, (least, more): (u64, u32)) -> u64 {
    least + u64::from(world.get_resource_or_insert_with(Dice::default).roll(more))
}

fn soil_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y) && level.in_height(y + 1)) else { return false };
    let name = |y: i32| level.registry().get(level.block(x, y, z)).map(|state| state.name);
    name(y).is_some_and(|name| SOILS.contains(&name)) && name(y + 1) == Some("minecraft:air")
}

/// A soil block near `at` to dig, if one of a few tries finds one: each try a random column, and
/// the highest open soil in it within reach up or down.
fn pick(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<BlockPos> {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    (0..TRIES).find_map(|_| {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let mut offset = |reach: i32| dice.roll((2 * reach + 1) as u32) as i32 - reach;
        let (x, z) = (x + offset(ACROSS), z + offset(ACROSS));
        (y - 1 - UP..=y - 1 + UP).rev().map(|y| [x, y, z]).find(|target| soil_at(world, layer, *target))
    })
}

/// Turns the extended `flag` on or off and tells the sniffer's viewers its whole extended word.
fn flag(world: &mut World, entity: Entity, flag: Flags, on: bool) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let Some(mut sniffing) = world.get_mut::<Sniffing>(entity) else { return };
    sniffing.extended = if on { sniffing.extended | flag.0 } else { sniffing.extended & !flag.0 };
    let item = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(Flags(BASE | sniffing.extended)) };
    let told = SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    shown::tell(world, entity, vec![sapling_net::encode(&told)]);
}

/// The spot on its feet' level [`AHEAD`] in front of the sniffer, where it digs.
fn ahead(world: &World, entity: Entity, body: &crate::body::Body) -> [f32; 3] {
    let yaw = world.get::<super::super::legs::Legs>(entity).map_or(0.0, |legs| legs.yaw).to_radians();
    [body.at[0] - yaw.sin() * AHEAD, body.at[1], body.at[2] + yaw.cos() * AHEAD]
}

/// Dust of the block under the digging spot, told to the sniffer's viewers.
fn dust(world: &mut World, entity: Entity, layer: Entity, body: &crate::body::Body) {
    let [x, y, z] = ahead(world, entity, body);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let data = level.registry().network_hash(level.block(x.floor() as i32, y.floor() as i32 - 1, z.floor() as i32)) as i32;
    let told = LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data };
    shown::tell(world, entity, vec![sapling_net::encode(&told)]);
}

fn into(world: &mut World, entity: Entity, stage: Stage, from: Option<Flags>, to: Option<Flags>) {
    if let Some(mut sniffing) = world.get_mut::<Sniffing>(entity) {
        sniffing.stage = stage;
    }
    from.into_iter().for_each(|from| flag(world, entity, from, false));
    to.into_iter().for_each(|to| flag(world, entity, to, true));
}

/// `on_scenting_success` fired on a grown sniffer: an idle or scenting one starts seeking a spot.
pub(super) fn scented(world: &mut World, entity: Entity, kind: &str, now: u64) {
    if kind != SNIFFER || world.get::<Raised>(entity).is_none_or(Raised::is_baby) {
        return;
    }
    let stage = world.get::<Sniffing>(entity).map(|sniffing| sniffing.stage);
    match stage {
        None => {
            let next = now + roll(world, SCENTS_EVERY);
            world.entity_mut(entity).insert(Sniffing { stage: Stage::Seeking, next, happy_until: None, extended: 0 });
        }
        Some(Stage::Scenting { .. }) => into(world, entity, Stage::Seeking, Some(TIMER_FLAG_1), None),
        Some(Stage::Idle) => into(world, entity, Stage::Seeking, None, None),
        Some(_) => {}
    }
}

/// A tick of a sniffer that thinks: its timers run and its stages turn over.
pub(super) fn tick(world: &mut World, entity: Entity, kind: &str, now: u64) {
    if kind != SNIFFER || world.get::<Raised>(entity).is_none_or(Raised::is_baby) {
        return;
    }
    let Some((layer, _, _, body)) = shown::of(world, entity).filter(|(layer, ..)| crate::rules::on(world, *layer, sapling_data::MOB_AI)) else { return };
    let Some((stage, next, happy_until)) = world.get::<Sniffing>(entity).map(|sniffing| (sniffing.stage, sniffing.next, sniffing.happy_until)) else {
        let next = now + roll(world, SCENTS_EVERY);
        world.entity_mut(entity).insert(Sniffing { stage: Stage::Idle, next, happy_until: None, extended: 0 });
        return;
    };
    if happy_until.is_some_and(|until| now >= until) {
        world.get_mut::<Sniffing>(entity).expect("read above").happy_until = None;
        flag(world, entity, TIMER_FLAG_3, false);
    }
    match stage {
        Stage::Idle if now >= next => into(world, entity, Stage::Scenting { until: now + SCENTS_FOR }, None, Some(TIMER_FLAG_1)),
        Stage::Scenting { until } if now >= until => {
            world.get_mut::<Sniffing>(entity).expect("read above").next = now + roll(world, SCENTS_EVERY);
            into(world, entity, Stage::Seeking, Some(TIMER_FLAG_1), None);
        }
        Stage::Seeking => {
            if let Some(target) = pick(world, layer, body.at) {
                into(world, entity, Stage::Searching { target, check: now + STUCK_FOR, from: body.at }, None, Some(SEARCHING));
            }
        }
        Stage::Searching { target, .. } if (body.at[0] - target[0] as f32 - 0.5).hypot(body.at[2] - target[2] as f32 - 0.5) <= NEAR => {
            let until = now + roll(world, DIGS_FOR);
            into(world, entity, Stage::Digging { started: now, until }, Some(SEARCHING), Some(DIGGING));
        }
        Stage::Searching { check, from, .. } if now >= check && (body.at[0] - from[0]).hypot(body.at[2] - from[2]) < STUCK_WITHIN => {
            let until = now + roll(world, AFTER);
            into(world, entity, Stage::Rising { until }, Some(SEARCHING), Some(TIMER_FLAG_2));
        }
        Stage::Searching { target, check, .. } if now >= check => {
            if let Some(mut sniffing) = world.get_mut::<Sniffing>(entity) {
                sniffing.stage = Stage::Searching { target, check: now + STUCK_FOR, from: body.at };
            }
        }
        Stage::Digging { started, .. } if now == started + FINDS_AT => {
            let find = FINDS[world.get_resource_or_insert_with(Dice::default).roll(FINDS.len() as u32) as usize];
            let [x, y, z] = ahead(world, entity, &body);
            crate::drops::toss(world, layer, ItemStack { name: find.into(), count: 1, damage: 0, block: None, tag: None }, [x, y + FIND_UP, z]);
        }
        Stage::Digging { started, .. } if now >= started + DUSTS_FROM && now < started + FINDS_AT && (now - started - DUSTS_FROM).is_multiple_of(DUSTS_EVERY) => dust(world, entity, layer, &body),
        Stage::Digging { until, .. } if now >= until => {
            let rising = Stage::Rising { until: now + roll(world, AFTER) };
            world.get_mut::<Sniffing>(entity).expect("read above").happy_until = Some(now + roll(world, AFTER));
            into(world, entity, rising, Some(DIGGING), Some(TIMER_FLAG_2));
            flag(world, entity, TIMER_FLAG_3, true);
        }
        Stage::Rising { until } if now >= until => into(world, entity, Stage::Idle, Some(TIMER_FLAG_2), None),
        _ => {}
    }
}

/// Before a sniffer's move: it walks to the soil it searches for, and stands while it scents,
/// digs and rises. False when it is idle.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(stage) = world.get::<Sniffing>(entity).map(|sniffing| sniffing.stage).filter(|stage| !matches!(stage, Stage::Idle | Stage::Seeking)) else { return false };
    let Some(body) = world.get::<crate::body::Body>(entity).copied() else { return false };
    match stage {
        Stage::Searching { target, .. } => {
            let pace = sapling_data::husbandry(kind).map_or(0.09, |rules| rules.walks) * SEARCH_PACE;
            super::super::legs::walk(world, entity, layer, &body, [target[0] as f32 + 0.5, target[2] as f32 + 0.5], pace, NEAR);
        }
        _ => super::super::legs::rest(world, entity),
    }
    true
}
