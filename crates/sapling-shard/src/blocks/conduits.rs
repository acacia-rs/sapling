//! Conduits waking and sleeping by their frame, by the Java game's `ConduitBlockEntity` rule
//! (docs/research/block-entities.md "Conduits"): every 40 ticks one in water all round, with at
//! least 16 prismarine or sea lanterns where its frame goes, is `Active` in its entity. BDS told
//! clients nothing of it (research/captures/bds-conduits.txt), so the entity changes unshown.
// TODO: an active conduit gives no Conduit Power (the effect is not in sapling yet) and hunts no
// hostile mob (`Target` stays -1).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use super::level;
use crate::components::{InLayer, Layer, Tick};
use crate::view::View;

const ENTITY: &str = "Conduit";
/// Ticks between looks at a conduit's frame.
const EVERY: u64 = 40;
/// Frame blocks a conduit wakes with.
const WAKES: usize = 16;
const FRAME: &[&str] = &["minecraft:prismarine", "minecraft:prismarine_bricks", "minecraft:dark_prismarine", "minecraft:sea_lantern"];
/// Columns this many chunks across from a player's are looked at.
const REACH: i32 = 4;

type Level = sapling_world::World;

/// The offsets where a conduit's frame goes: the three rings, 2 out, around its middle on each
/// axis (the Java game's `ConduitBlockEntity.updateShape`).
fn frame_offsets() -> impl Iterator<Item = [i32; 3]> {
    (-2..=2_i32).flat_map(|x| (-2..=2_i32).flat_map(move |y| (-2..=2_i32).map(move |z| [x, y, z]))).filter(|&[x, y, z]| {
        let [ax, ay, az] = [x.abs(), y.abs(), z.abs()];
        (ax > 1 || ay > 1 || az > 1) && (x == 0 && (ay == 2 || az == 2) || y == 0 && (ax == 2 || az == 2) || z == 0 && (ax == 2 || ay == 2))
    })
}

fn is_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    let water = |id: u32| level.registry().get(id).is_some_and(|state| state.is_water());
    level.in_height(y) && (water(level.block(x, y, z)) || water(level.liquid(x, y, z)))
}

/// Whether the conduit at `pos` wakes: water in the 3 by 3 by 3 round it, itself standing in
/// water, and enough of its frame.
fn wakes(level: &Level, [x, y, z]: BlockPos) -> bool {
    let mut near = (-1..=1).flat_map(|dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [x + dx, y + dy, z + dz])));
    if !near.all(|pos| is_water(level, pos)) {
        return false;
    }
    let framed = frame_offsets().filter(|&[dx, dy, dz]| {
        let [fx, fy, fz] = [x + dx, y + dy, z + dz];
        level.in_height(fy) && level.registry().get(level.block(fx, fy, fz)).is_some_and(|state| FRAME.contains(&state.name))
    });
    framed.count() >= WAKES
}

/// Every 40 ticks, the conduits near players that woke or slept since, kept so.
pub(crate) fn tick(world: &mut World) {
    if world.resource::<Tick>().0 % EVERY != 0 {
        return;
    }
    let mut columns = Vec::new();
    for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
        let Some(center) = view.center() else { continue };
        columns.extend((-REACH..=REACH).flat_map(|dx| (-REACH..=REACH).map(move |dz| (in_layer.0, ChunkPos::new(center.x + dx, center.z + dz)))));
    }
    columns.sort_by_key(|(layer, pos)| (*layer, pos.x, pos.z));
    columns.dedup();
    let mut changed = Vec::new();
    for (layer, column) in columns {
        let Some(level) = level(world, layer) else { continue };
        for entity in level.block_entities_in(column) {
            if !matches!(entity.nbt.get("id"), Some(Value::String(id)) if id.as_str() == ENTITY) {
                continue;
            }
            let was = matches!(entity.nbt.get("Active"), Some(Value::Byte(1)));
            let now = wakes(level, entity.pos);
            if was != now {
                changed.push((layer, entity.pos, entity.nbt.clone(), now));
            }
        }
    }
    for (layer, pos, mut nbt, now) in changed {
        if let Value::Compound(fields) = &mut nbt {
            fields.retain(|(name, _)| name != "Active");
            fields.push(("Active".into(), Value::Byte(i8::from(now))));
            fields.sort_by(|a, b| a.0.cmp(&b.0));
        }
        if let Some(mut level) = world.get_mut::<Layer>(layer) {
            level.world.set_block_entity(pos, &nbt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_frame_is_42_blocks_and_a_box_short_of_three_faces_keeps_enough() {
        assert_eq!(frame_offsets().count(), 42);
        // bds-conduits.txt: the shell's two side faces and top turned to glass.
        let kept = frame_offsets().filter(|&[_, y, z]| z.abs() != 2 && y != 2).count();
        assert!(kept >= WAKES, "{kept}");
    }
}
