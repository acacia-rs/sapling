use std::sync::Arc;
use std::time::Instant;

use acacia_world::{BlockRegistry, Chunk, Dimension};

use super::nibbles::Channel;
use super::{FULL_LIGHT, Props, light_column};
use crate::{ChunkPos, Generated, MemoryStorage, Template, World, WorldStorage};

fn id(name: &str, properties: &str) -> u32 {
    BlockRegistry::vanilla().find(name, properties).unwrap_or_else(|| panic!("{name} [{properties}]"))
}

fn stone() -> u32 {
    id("minecraft:stone", "")
}

fn glowstone() -> u32 {
    id("minecraft:glowstone", "")
}

fn air() -> u32 {
    BlockRegistry::vanilla().air_id()
}

fn leaves() -> u32 {
    id("minecraft:oak_leaves", "persistent_bit=0,update_bit=0")
}

fn overworld() -> Dimension {
    Dimension::overworld(air())
}

/// Stone up to y 63 in the column at `(x, z)`.
fn ground(x: i32, z: i32) -> Chunk {
    let mut chunk = Chunk::empty(x, z, overworld());
    (-4..4).for_each(|section| chunk.fill_section(section, stone()));
    chunk
}

/// A world of storage whose `chunks` are all loaded and lit.
fn loaded(chunks: Vec<Chunk>) -> World {
    let storage = MemoryStorage::default();
    let positions: Vec<ChunkPos> = chunks.iter().map(|chunk| ChunkPos::new(chunk.x, chunk.z)).collect();
    chunks.into_iter().for_each(|chunk| storage.insert(0, chunk));
    let mut world = World::with_storage(Arc::new(storage));
    positions.into_iter().for_each(|pos| world.load_now(pos));
    world.relight();
    world
}

fn light(world: &World, [x, y, z]: [i32; 3]) -> (u8, u8) {
    world.light_at(x, y, z).expect("loaded")
}

#[test]
fn a_column_alone_has_the_sky_down_to_its_ground_and_its_lamps_light() {
    let mut chunk = ground(0, 0);
    chunk.set(8, 64, 8, 0, glowstone());
    (1..=5).for_each(|x| (1..=5).for_each(|z| chunk.set(x, 70, z, 0, leaves())));
    let light = light_column(&chunk, &Props::of(BlockRegistry::vanilla()));
    let at = |channel, [x, y, z]: [i32; 3]| light.get(channel, x, y, z).unwrap();
    assert_eq!((at(Channel::Sky, [0, 64, 0]), at(Channel::Sky, [0, 63, 0]), at(Channel::Sky, [0, 319, 0])), (15, 0, 15));
    assert_eq!((at(Channel::Sky, [3, 70, 3]), at(Channel::Sky, [3, 69, 3])), (13, 12), "leaves take 2 from the sky going down");
    assert_eq!((at(Channel::Block, [8, 64, 8]), at(Channel::Block, [9, 64, 8]), at(Channel::Block, [8, 66, 8]), at(Channel::Block, [8, 63, 8])), (15, 14, 13, 0));
    assert_eq!(at(Channel::Block, [15, 64, 8]), 8, "and to the column's side, not past it");
    assert!(light.arrays() <= 6, "only the sections around the ground and the lamp hold arrays: {}", light.arrays());
}

#[test]
fn a_lamp_lights_across_a_column_border_and_its_removal_darkens_both() {
    let mut world = loaded(vec![ground(0, 0), ground(1, 0)]);
    world.set_block(15, 64, 8, glowstone());
    world.relight();
    assert_eq!((light(&world, [15, 64, 8]).1, light(&world, [16, 64, 8]).1, light(&world, [20, 64, 8]).1), (15, 14, 10));
    world.set_block(15, 64, 8, air());
    world.relight();
    assert_eq!((light(&world, [15, 64, 8]), light(&world, [16, 64, 8]), light(&world, [20, 65, 8])), ((15, 0), (15, 0), (15, 0)));
}

#[test]
fn a_roof_shades_across_borders_and_a_hole_in_it_lets_the_sky_down() {
    let mut world = loaded((-1..=1).flat_map(|x| (-1..=1).map(move |z| ground(x, z))).collect());
    world.fill([-12, 70, -12], [31, 70, 31], stone());
    world.relight();
    assert_eq!(light(&world, [8, 64, 8]).0, 0, "deep under the roof");
    assert_eq!(light(&world, [-14, 64, 8]).0, 15);
    assert_eq!(light(&world, [-10, 64, 8]).0, 12, "the sky from the side");
    world.set_block(16, 70, 8, air());
    world.relight();
    assert_eq!((light(&world, [16, 64, 8]).0, light(&world, [15, 64, 8]).0, light(&world, [12, 64, 8]).0), (15, 14, 11));
    world.set_block(16, 70, 8, stone());
    world.relight();
    assert_eq!((light(&world, [16, 64, 8]).0, light(&world, [15, 64, 8]).0), (0, 0));
}

#[test]
fn light_waits_at_a_column_not_loaded_and_crosses_once_it_is() {
    let mut lamp = ground(0, 0);
    lamp.set(15, 64, 8, 0, glowstone());
    let storage = MemoryStorage::default();
    storage.insert(0, lamp);
    storage.insert(0, ground(1, 0));
    let mut world = World::with_storage(Arc::new(storage));
    world.load_now(ChunkPos::new(0, 0));
    world.relight();
    assert_eq!((light(&world, [15, 64, 8]).1, world.light_at(16, 64, 8)), (15, None));
    world.load_now(ChunkPos::new(1, 0));
    assert_eq!(light(&world, [16, 64, 8]).1, 0, "lit alone until relit");
    world.relight();
    assert_eq!((light(&world, [16, 64, 8]).1, light(&world, [17, 65, 8]).1), (14, 12));
}

/// Writes blocks at random over nine columns, relit now and then, and compares the light with
/// that of the same blocks lit from scratch.
#[test]
fn mended_light_is_the_light_of_the_blocks_lit_afresh() {
    let positions: Vec<ChunkPos> = (-1..=1).flat_map(|x| (-1..=1).map(move |z| ChunkPos::new(x, z))).collect();
    let mut world = loaded(positions.iter().map(|pos| ground(pos.x, pos.z)).collect());
    let blocks = [stone(), glowstone(), air(), air(), leaves(), id("minecraft:torch", "torch_facing_direction=top"), id("minecraft:water", "liquid_depth=0")];
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut roll = |below: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 33) % below
    };
    for round in 0..40 {
        for _ in 0..60 {
            let (x, y, z) = (roll(48) as i32 - 16, 60 + roll(14) as i32, roll(48) as i32 - 16);
            world.set_block(x, y, z, blocks[roll(blocks.len() as u64) as usize]);
        }
        if round % 3 != 2 {
            world.relight();
        }
    }
    world.relight();
    let afresh = loaded(positions.iter().map(|pos| world.column(*pos).unwrap().stored().chunk).collect());
    for x in -16..32 {
        for z in -16..32 {
            for y in 50..90 {
                assert_eq!(light(&world, [x, y, z]), light(&afresh, [x, y, z]), "at {x} {y} {z}");
            }
        }
    }
}

#[test]
fn an_arena_mends_its_own_light_and_its_template_keeps_its() {
    let mut level = World::void();
    level.fill([-8, 64, -8], [7, 64, 7], stone());
    let template = Template::of(level);
    let (mut played, untouched) = (World::arena(&template), World::arena(&template));
    // Under the platform the sky comes in from its edge, eight blocks off.
    assert_eq!((light(&played, [0, 63, 0]).0, light(&played, [0, 65, 0]).0), (7, 15));
    played.set_block(0, 64, 0, air());
    played.relight();
    assert_eq!((light(&played, [0, 63, 0]).0, light(&untouched, [0, 63, 0]).0, played.changed_columns()), (15, 7, 1));
    played.reset();
    assert_eq!(light(&played, [0, 63, 0]).0, 7);
}

#[test]
fn the_nether_has_no_sky_light_and_brightness_dims_with_the_sky() {
    let mut nether = World::void_in(crate::NETHER);
    nether.set_block(0, 64, 0, glowstone());
    nether.relight();
    assert_eq!((light(&nether, [0, 100, 0]), light(&nether, [1, 64, 0])), ((0, 0), (0, 14)));

    let mut world = World::void();
    world.set_block(0, 64, 0, stone());
    world.set_block(10, 64, 0, glowstone());
    world.relight();
    assert_eq!((world.brightness_at(0, 65, 0, 0), world.brightness_at(0, 65, 0, 11)), (Some(15), Some(4)));
    assert_eq!(world.brightness_at(9, 65, 0, 11), Some(13), "a lamp near beats the night sky");
    assert_eq!(world.brightness_at(0, 64, 0, 0), Some(0), "inside stone");
    assert_eq!(world.light_at(0, 400, 0), Some((FULL_LIGHT, 0)));
}

/// How long a generated column takes to light on an I/O thread; printed, and checked loosely.
#[test]
fn a_generated_column_is_lit_on_load() {
    let storage = Generated::new(MemoryStorage::default(), 20261003);
    let chunk = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap().chunk;
    let props = Props::of(BlockRegistry::vanilla());
    let start = Instant::now();
    let light = light_column(&chunk, &props);
    eprintln!("lit a generated column in {:?}, {} arrays of 2 KiB", start.elapsed(), light.arrays());
    assert_eq!((light.get(Channel::Sky, 0, 319, 0), light.get(Channel::Sky, 0, -60, 0)), (Some(15), Some(0)));
    assert!(light.arrays() < 24, "{}", light.arrays());
}
