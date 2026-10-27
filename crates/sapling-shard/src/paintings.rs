//! Paintings players hang: a painting used on the side of a block takes, at random, one of the
//! largest pictures that fit there, as BDS's painting does (docs/research/mob-kinds.md
//! "Paintings"). A hung painting is kept with its column and shown as the stored ones are (mobs.rs).
// TODO: breaking a painting (a hit, or its wall going) and its drop are not built.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::{ChunkPos, World as Level};

use crate::blocks::Dice;
use crate::components::Layer;
use crate::{entities, mobs};

pub(crate) const PAINTING: &str = "minecraft:painting";
/// BDS's pictures in its order with their width and height in blocks, and whether a hung
/// painting may take them (`142a1ce60`, each `Motive` built at `1429f4250`).
const MOTIVES: &[(&str, i32, i32, bool)] = &[
    ("Kebab", 1, 1, true), ("Aztec", 1, 1, true), ("Alban", 1, 1, true), ("Aztec2", 1, 1, true), ("Bomb", 1, 1, true), ("Plant", 1, 1, true),
    ("Wasteland", 1, 1, true), ("Pool", 2, 1, true), ("Courbet", 2, 1, true), ("Sea", 2, 1, true), ("Sunset", 2, 1, true), ("Creebet", 2, 1, true),
    ("Wanderer", 1, 2, true), ("Graham", 1, 2, true), ("Match", 2, 2, true), ("Bust", 2, 2, true), ("Stage", 2, 2, true), ("Void", 2, 2, true),
    ("SkullAndRoses", 2, 2, true), ("Wither", 2, 2, true), ("Fighters", 4, 2, true), ("Pointer", 4, 4, true), ("Pigscene", 4, 4, true),
    ("BurningSkull", 4, 4, true), ("Skeleton", 4, 3, true), ("DonkeyKong", 4, 3, true), ("Earth", 2, 2, false), ("Wind", 2, 2, false),
    ("Fire", 2, 2, false), ("Water", 2, 2, false), ("baroque", 2, 2, true), ("humble", 2, 2, true), ("meditative", 1, 1, true),
    ("prairie_ride", 1, 2, true), ("unpacked", 4, 4, true), ("backyard", 3, 4, true), ("bouquet", 3, 3, true), ("cavebird", 3, 3, true),
    ("changing", 4, 2, true), ("cotan", 3, 3, true), ("endboss", 3, 3, true), ("fern", 3, 3, true), ("finding", 4, 2, true), ("lowmist", 4, 2, true),
    ("orb", 4, 4, true), ("owlemons", 3, 3, true), ("passage", 4, 2, true), ("pond", 3, 4, true), ("sunflowers", 3, 3, true), ("tides", 3, 3, true),
    ("dennis", 3, 3, true),
];
/// How far from its wall's face a painting's middle is.
const THICKNESS: f32 = 0.03125;

/// A painting's wall side, by BDS's `Direction`: 0 south, 1 west, 2 north, 3 east.
fn direction(face: Face) -> Option<i8> {
    match face {
        Face::South => Some(0),
        Face::West => Some(1),
        Face::North => Some(2),
        Face::East => Some(3),
        Face::Up | Face::Down => None,
    }
}

/// The way a painting facing `direction` is widened: to its viewer's left (`[dx, dz]`).
fn across(direction: i8) -> [i32; 2] {
    [[1, 0], [0, 1], [-1, 0], [0, -1]][direction as usize & 3]
}

fn facing(direction: i8) -> [i32; 2] {
    [[0, 1], [-1, 0], [0, -1], [1, 0]][direction as usize & 3]
}

/// Where a `w` by `h` painting hung in front of the cell `front` facing `direction` has its
/// middle, and the lowest corner of the cells it covers.
fn placed(front: BlockPos, direction: i8, w: i32, h: i32) -> ([f32; 3], BlockPos) {
    let shift = |size: i32| if size % 2 == 0 { 0.5 } else { 0.0 };
    let ([ax, az], [fx, fz]) = (across(direction), facing(direction));
    let middle = [
        front[0] as f32 + 0.5 + ax as f32 * shift(w) - fx as f32 * (0.5 - THICKNESS),
        front[1] as f32 + 0.5 + shift(h),
        front[2] as f32 + 0.5 + az as f32 * shift(w) - fz as f32 * (0.5 - THICKNESS),
    ];
    (middle, corner(middle, direction, w, h))
}

/// The lowest corner of the cells covered by a `w` by `h` painting with its middle at `middle`.
fn corner(middle: [f32; 3], direction: i8, w: i32, h: i32) -> BlockPos {
    let ([ax, az], [fx, fz]) = (across(direction), facing(direction));
    let low = |c: f32, size: i32| (c - size as f32 / 2.0).floor() as i32;
    let front = |c: f32, f: i32| (c + f as f32 * 0.5).floor() as i32;
    [if ax == 0 { front(middle[0], fx) } else { low(middle[0], w) }, low(middle[1], h), if az == 0 { front(middle[2], fz) } else { low(middle[2], w) }]
}

/// The cells a `w` by `h` painting from `corner` covers.
fn cells(corner: BlockPos, direction: i8, w: i32, h: i32) -> impl Iterator<Item = BlockPos> {
    let [ax, az] = across(direction).map(i32::abs);
    (0..w).flat_map(move |i| (0..h).map(move |j| [corner[0] + ax * i, corner[1] + j, corner[2] + az * i]))
}

/// BDS's `survives`: a solid block behind each cell, nothing solid in it, and no other painting there.
fn fits(level: &Level, corner: BlockPos, direction: i8, w: i32, h: i32, hung: &[(BlockPos, BlockPos)]) -> bool {
    let registry = level.registry();
    let solid = |[x, y, z]: BlockPos| registry.get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    let [fx, fz] = facing(direction);
    let mut own = cells(corner, direction, w, h);
    own.all(|cell| solid([cell[0] - fx, cell[1], cell[2] - fz]) && !solid(cell) && !hung.iter().any(|(low, high)| (0..3).all(|axis| (low[axis]..=high[axis]).contains(&cell[axis]))))
}

/// The cells each painting kept in the columns about `front` covers, lowest and highest corner.
fn hung_about(level: &Level, front: BlockPos) -> Vec<(BlockPos, BlockPos)> {
    let home = ChunkPos::of_block(front[0], front[2]);
    let columns = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz)));
    let actors = columns.filter_map(|pos| sapling_world::actors(level.actors(pos)).ok()).flatten();
    actors
        .filter_map(|actor| {
            let painting = actor.painting?;
            let &(_, w, h, _) = MOTIVES.iter().find(|motive| motive.0 == painting.motif)?;
            let corner = corner(actor.position, painting.direction, w, h);
            let [ax, az] = across(painting.direction).map(i32::abs);
            Some((corner, [corner[0] + ax * (w - 1), corner[1] + h - 1, corner[2] + az * (w - 1)]))
        })
        .collect()
}

/// A painting used on the `face` side of the block at `wall` in `layer`: false where none fits.
pub(crate) fn hang(world: &mut World, layer: Entity, wall: BlockPos, face: Face) -> bool {
    let Some(direction) = direction(face) else { return false };
    let front = face.neighbour(wall);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let hung = hung_about(level, front);
    let mut largest: Vec<&(&str, i32, i32, bool)> = Vec::new();
    for motive in MOTIVES.iter().filter(|motive| motive.3) {
        let (_, corner) = placed(front, direction, motive.1, motive.2);
        let area = motive.1 * motive.2;
        if fits(level, corner, direction, motive.1, motive.2, &hung) && largest.first().is_none_or(|best| area >= best.1 * best.2) {
            largest.retain(|best| best.1 * best.2 >= area);
            largest.push(motive);
        }
    }
    if largest.is_empty() {
        return false;
    }
    let &(motif, w, h, _) = largest[world.get_resource_or_insert_with(Dice::default).roll(largest.len() as u32) as usize];
    let (middle, _) = placed(front, direction, w, h);
    let shown = mobs::spawn_shown(world, layer, middle, |id| Some(vec![sapling_net::encode(&sapling_data::add_painting(id, middle, direction.into(), motif))]));
    let Some(unique_id) = shown.and_then(|painting| entities::new_unique_id(world, painting)) else { return false };
    let home = ChunkPos::of_block(middle[0].floor() as i32, middle[2].floor() as i32);
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let mut record = level.world.actors(home).to_vec();
    record.extend(sapling_world::new_painting_actor(unique_id, middle, direction, motif));
    level.world.set_actors(home, record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paintings_sit_where_bds_put_them() {
        // research/captures/bds-paintings.txt: west faces of a wall at x 3, the clicked cells in front at x 2.
        assert_eq!(placed([2, -24, -2], 1, 4, 4), ([2.96875, -23.0, -1.0], [2, -25, -3]));
        assert_eq!(placed([2, -25, -2], 1, 2, 1), ([2.96875, -24.5, -1.0], [2, -25, -2]));
        assert_eq!(placed([2, -25, 3], 1, 2, 2), ([2.96875, -24.0, 4.0], [2, -25, 3]));
    }
}
