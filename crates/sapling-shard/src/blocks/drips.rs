//! A stalactite under water or lava drips, and fills the cauldron under its tip: water a third at a
//! time, lava all at once (`PointedDripstoneBlock` random tick 1465fe4b0 as decompiled;
//! research/captures/bds-dripstone.txt for the timing and what viewers are told).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{level, send};
use crate::body::Liquid;

const DRIPSTONE: &str = "minecraft:pointed_dripstone";
const CAULDRON: &str = "minecraft:cauldron";
/// The odds a root's random tick drips water or lava: not the Java game's.
const WATER_ODDS: f32 = 0.42;
const LAVA_ODDS: f32 = 0.07;
/// A dripstone grows on this much of its random ticks, drawn before the drip; sapling grows none.
const GROWTH_ODDS: f32 = 0.011_377_778;
/// How far a tip is looked for under its root, and a cauldron under its tip.
const REACH: i32 = 10;
/// Ticks from a drip to the cauldron filling, and one more for each block it falls.
const DRIP_TICKS: u64 = 50;
const FULL: u8 = 6;
const WATER_STEP: u8 = 2;
/// The colour BDS sent with each fill, water or lava.
const FILL_DATA: i32 = -1_087_090_343;
const LAVA_FILL: LevelEventEvent = LevelEventEvent::Unknown(3512);

type Level = sapling_world::World;

fn state(level: &Level, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str)> {
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| (state.name, state.properties))
}

fn hangs(level: &Level, pos: BlockPos) -> bool {
    state(level, pos).is_some_and(|(name, properties)| name == DRIPSTONE && properties.contains("hanging=1"))
}

fn number(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// The source over the block a stalactite whose root is `root` hangs from.
fn source_over(level: &Level, root: BlockPos) -> Option<Liquid> {
    let (name, properties) = state(level, Face::Up.neighbour(Face::Up.neighbour(root)))?;
    (number(properties, "liquid_depth") == Some(0)).then(|| Liquid::of(name)).flatten()
}

/// The tip of the stalactite whose root is `root`.
fn tip_of(level: &Level, root: BlockPos) -> Option<BlockPos> {
    (0..REACH).map(|down| [root[0], root[1] - down, root[2]]).take_while(|at| hangs(level, *at)).find(|at| state(level, *at).is_some_and(|(_, properties)| properties.contains("dripstone_thickness=tip")))
}

/// The cauldron under `tip` past nothing but air, if one.
fn cauldron_under(level: &Level, tip: BlockPos) -> Option<BlockPos> {
    let below = (1..=REACH).map(|down| [tip[0], tip[1] - down, tip[2]]).find(|at| state(level, *at).is_none_or(|(name, _)| name != "minecraft:air"))?;
    state(level, below).is_some_and(|(name, _)| name == CAULDRON).then_some(below)
}

/// What the cauldron at `pos` becomes from a drip of `liquid`, where it takes one: water while it
/// holds water and is not full, lava only empty (`CauldronBlock::canReceiveStalactiteDrip`).
fn filled(level: &Level, pos: BlockPos, liquid: Liquid) -> Option<(u32, u8)> {
    let (_, properties) = state(level, pos)?;
    let level_now = number(properties, "fill_level")?;
    let holds_water = properties.contains("cauldron_liquid=water");
    let (kind, fill) = match liquid {
        Liquid::Water if holds_water && level_now < FULL => ("water", (level_now + WATER_STEP).min(FULL)),
        Liquid::Lava if level_now == 0 => ("lava", FULL),
        _ => return None,
    };
    let id = level.registry().find(CAULDRON, &format!("cauldron_liquid={kind},fill_level={fill}"))?;
    Some((id, fill))
}

/// The random tick of the hanging dripstone at `pos`: a root under a source drips by the odds,
/// and a cauldron under its tip is due to fill.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, roll: &mut impl FnMut() -> f32) {
    let Some(level) = level(world, layer) else { return };
    if !hangs(level, pos) || hangs(level, Face::Up.neighbour(pos)) {
        return;
    }
    let _grows = roll() < GROWTH_ODDS;
    let Some(liquid) = source_over(level, pos) else { return };
    if roll() > if liquid == Liquid::Water { WATER_ODDS } else { LAVA_ODDS } {
        return;
    }
    let Some(tip) = tip_of(level, pos) else { return };
    let cauldron = cauldron_under(level, tip).filter(|at| filled(level, *at, liquid).is_some());
    // BDS told the drip with 28 for water and 29 for lava.
    let kind = if liquid == Liquid::Water { 28 } else { 29 };
    let [x, y, z] = tip.map(|c| c as f32);
    let packet = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDripstoneDrip, position: Vec3f { x, y, z }, data: kind });
    send::to_viewers(world, layer, tip, &[packet]);
    if let Some(cauldron) = cauldron {
        queued::after(world, layer, cauldron, DRIP_TICKS + (tip[1] - cauldron[1]) as u64, Job::Drip);
    }
}

/// A drip reaches the cauldron at `pos`: it fills from the stalactite over it, if that still drips.
pub(super) fn fill(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let tip = (1..=REACH).map(|up| [pos[0], pos[1] + up, pos[2]]).find(|at| state(level, *at).is_none_or(|(name, _)| name != "minecraft:air"));
    let root = tip.filter(|tip| hangs(level, *tip)).map(|tip| (0..).map(|up| [tip[0], tip[1] + up, tip[2]]).take_while(|at| hangs(level, *at)).last().unwrap_or(tip));
    let Some((liquid, (id, fill))) = root.and_then(|root| source_over(level, root)).and_then(|liquid| Some((liquid, filled(level, pos, liquid)?))) else { return };
    let (event, sound) = match liquid {
        Liquid::Water => (LevelEventEvent::CauldronFillWater, "cauldron_drip.water.pointed_dripstone"),
        Liquid::Lava => (LAVA_FILL, "cauldron_drip.lava.pointed_dripstone"),
    };
    let [x, y, z] = pos.map(|c| c as f32);
    let surface = Vec3f { x: x + 0.5, y: y + 0.375 + 0.125 * f32::from(fill), z: z + 0.5 };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event, position: surface, data: FILL_DATA })]);
    if send::set(world, layer, pos, id) {
        send::to_viewers(world, layer, pos, &[send::noise(sound, Vec3f { x, y, z }, -1)]);
    }
}
