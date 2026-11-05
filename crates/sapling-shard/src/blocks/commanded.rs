//! What BDS's `/setblock` (and so `ShardCtx::update_block`) writes beyond the block named
//! (research/captures/bds-fixset.txt, bds-bkticks.txt): a bed is laid whole with its head in the
//! named cell, a small dripleaf with its upper half; coral plants and fans play `place`; nether vines, cave vines and kelp under water take a random age; a live coral fan
//! or kelp keeps a water source beside itself, which then runs; a comparator set again in its own
//! block type gives out what it gave (redstone/kept.rs); a rail joins the rails around it as a placed one does.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face};

use super::{Dice, doors, level, liquid, send};

/// Blocks given a random age, by their age property, when the command names none.
const AGED: [(&str, &str); 4] = [
    ("minecraft:weeping_vines", "weeping_vines_age"),
    ("minecraft:twisting_vines", "twisting_vines_age"),
    ("minecraft:cave_vines", "growing_plant_age"),
    ("minecraft:kelp", "kelp_age"),
];
/// The ages drawn, 0 to 25: BDS's commanded ones were 2 to 17, its hand-placed glow berries 0 to 25
/// (research/captures/bds-p2place.txt).
const AGES: u32 = 26;
const TORCH_FACING: &str = "torch_facing_direction";

/// A bed `id` laid with its head at `pos` and its foot behind it, where that cell is air, both
/// shown in one UpdateSubchunkBlocks. `None` for any other block or a bed with no room.
pub(super) fn put_bed(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> Option<bool> {
    let level = level(world, layer)?;
    let registry = level.registry();
    let head = doors::with(registry, id, "head_piece_bit", "1")?;
    let foot_cell = doors::ahead(registry, id, pos, -1)?;
    let foot = doors::with(registry, id, "head_piece_bit", "0")?;
    let [x, y, z] = foot_cell;
    let is_free = level.in_height(y) && level.block(x, y, z) == registry.air_id();
    if !is_free {
        return None;
    }
    if level.block(pos[0], pos[1], pos[2]) == head {
        return Some(false);
    }
    if !send::set_quietly(world, layer, pos, head) || !send::set_quietly(world, layer, foot_cell, foot) {
        return None;
    }
    send::show_together(world, layer, &[(pos, send::FLAGS), (foot_cell, send::FLAGS)]);
    for cell in [pos, foot_cell] {
        super::set_entities::furnish(world, layer, cell, "minecraft:bed");
    }
    Some(true)
}

/// Writes `block` at `pos` as [`send::put_block`] does, aged or wetted as BDS's command does. An
/// aged or wetted block is shown with its water as BDS shows the writes, each listed
/// ([`send::logged`]): BDS writes an aged one twice (bds-fixset.txt, bds-bkticks.txt).
pub(super) fn put(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> Option<bool> {
    let (block, is_aged) = match standing_torch(world, layer, pos, block) {
        Some(torch) => (torch, true),
        None => aged(world, layer, pos, block),
    };
    // Kelp set in air got a source beside itself too (research/captures/bds-plantpop.txt).
    let wets = is_live_fan(&block.name) || block.name == "minecraft:kelp";
    if is_aged || wets {
        return send::logged(world, |world| put_now(world, layer, pos, &block, is_aged, wets));
    }
    put_now(world, layer, pos, &block, false, false)
}

fn put_now(world: &mut World, layer: Entity, pos: BlockPos, block: &Block, is_aged: bool, wets: bool) -> Option<bool> {
    let same_comparator = block.name.ends_with("_comparator") && send::block_at(world, layer, pos).is_some_and(|old| old.name == block.name);
    let put = send::put_block(world, layer, pos, block)?;
    if put && same_comparator {
        super::redstone::hold_comparator(world, layer, pos);
    }
    if put && is_aged {
        send::log_again(world, layer, pos);
    }
    if put && wets {
        liquid::pour(world, layer, pos, liquid::Liquid::Water, true);
    }
    if put {
        super::daylight::left_for_the_second(world, layer, pos);
        super::coral::heard(world, layer, pos);
        // BDS's /fill turned a set slope flat beside the rails it joined (bds-i2rails.txt, bds-l2carts.txt).
        super::rails::join_placed(world, layer, pos);
    }
    Some(put)
}

/// A small dripleaf's lower half `id` put at `pos` with its upper half over it, whatever was there,
/// both shown in one UpdateSubchunkBlocks, the upper first (research/captures/bds-e3turn.txt). `None`
/// for any other block, `Some(false)` where it already stands.
pub(super) fn put_dripleaf(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> Option<bool> {
    let level = level(world, layer)?;
    let registry = level.registry();
    let is_lower = registry.get(id).is_some_and(|state| state.name == "minecraft:small_dripleaf_block" && state.property("upper_block_bit") == Some("0"));
    let above = Face::Up.neighbour(pos);
    let upper = doors::with(registry, id, "upper_block_bit", "1").filter(|_| is_lower && level.in_height(above[1]))?;
    if level.block(pos[0], pos[1], pos[2]) == id {
        return Some(false);
    }
    if !send::set_quietly(world, layer, pos, id) {
        return None;
    }
    send::set_quietly(world, layer, above, upper);
    send::show_together(world, layer, &[(above, send::FLAGS), (pos, send::FLAGS)]);
    Some(true)
}

/// Whether `pos` holds half a bed, whose other half then shows its break dust at its corner: BDS
/// showed that before both halves, set over, in one UpdateSubchunkBlocks (research/captures/bds-e3sets.txt).
pub(super) fn dust_bed_half(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let is_bed = send::block_at(world, layer, pos).is_some_and(|old| old.name == "minecraft:bed");
    let Some(other) = doors::other_half(world, layer, pos).filter(|_| is_bed) else { return false };
    let Some(hash) = level(world, layer).map(|level| level.registry().network_hash(level.block(other[0], other[1], other[2])) as i32) else { return false };
    let [x, y, z] = other.map(|c| c as f32);
    let dust = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data: hash });
    send::to_viewers(world, layer, other, &[dust]);
    true
}

fn is_live_fan(name: &str) -> bool {
    !name.contains("dead_") && (name.ends_with("_coral_fan") || name.ends_with("_coral_wall_fan"))
}

/// `block` with a random age where it is one BDS ages and none was named. Kelp only under water:
/// a kelp set below glass kept age 0 (bds-fixset.txt), one in a pool two deep was aged (bds-bkticks.txt).
fn aged(world: &mut World, layer: Entity, [x, y, z]: BlockPos, block: &Block) -> (Block, bool) {
    let Some((name, key)) = AGED.iter().find(|(name, _)| *name == block.name) else { return (block.clone(), false) };
    if block.properties.split(',').any(|pair| pair.split_once('=').is_some_and(|(named, _)| named == *key)) {
        return (block.clone(), false);
    }
    let wet_above = level(world, layer).is_some_and(|level| {
        let above = Face::Up.neighbour([x, y, z]);
        level.in_height(above[1]) && level.registry().get(level.block(above[0], above[1], above[2])).is_some_and(|state| state.name == "minecraft:water")
    });
    if *name == "minecraft:kelp" && !wet_above {
        return (block.clone(), false);
    }
    (with_age(world, name, key, &block.properties), true)
}

/// A torch named bare, set over a solid block, stands on it, written twice as an aged block is: BDS's
/// went in `top` there (bds-p2rs.txt), `unknown` with nothing round it (block-defaults.tsv).
// TODO: which side a bare torch takes with no floor but a wall beside it.
fn standing_torch(world: &World, layer: Entity, [x, y, z]: BlockPos, block: &Block) -> Option<Block> {
    let bare = block.properties.is_empty() || block.properties == format!("{TORCH_FACING}=unknown");
    let level = level(world, layer).filter(|level| block.name == "minecraft:torch" && bare && level.in_height(y - 1))?;
    level.registry().get(level.block(x, y - 1, z)).filter(|under| under.is_solid())?;
    Some(Block::new(block.name.as_str()).with(format!("{TORCH_FACING}=top")))
}

fn with_age(world: &mut World, name: &str, key: &str, properties: &str) -> Block {
    let age = world.get_resource_or_insert_with(Dice::default).roll(AGES);
    let kept = properties.split(',').filter(|pair| !pair.is_empty() && pair.split_once('=').is_none_or(|(named, _)| named != key));
    let states = kept.map(str::to_owned).chain([format!("{key}={age}")]).collect::<Vec<_>>().join(",");
    Block::new(name).with(states)
}

/// `block` placed by hand with a random age, where it is one BDS ages: its glow berries, weeping and
/// twisting vines all took one (research/captures/bds-p2place.txt). `None` for any other block.
pub(super) fn aged_by_hand(world: &mut World, block: &Block) -> Option<Block> {
    let (name, key) = AGED.iter().find(|(name, _)| *name == block.name)?;
    Some(with_age(world, name, key, &block.properties))
}
