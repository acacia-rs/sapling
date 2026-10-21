//! The exit portal (`EndPodiumFeature::place` 144c12de0, research/dragon-fight-decompile.md 3), as
//! BDS wrote it when a fight began and when its dragon died (research/captures/bds-dragon-fight.txt):
//! round `origin`, x then y then z, by the squared distance to it: bedrock below within 7, end stone
//! to 13; at its level the middle air, then end portal if lit, and a bedrock ring; air above. Then
//! the bedrock pillar and a torch on each side of it two up.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use crate::components::Layer;

const REACH: i32 = 4;
const BELOW: i32 = 1;
const ABOVE: i32 = 4;
const INNER: i32 = 7;
const OUTER: i32 = 13;
const PILLAR: i32 = 4;
pub(super) const BEDROCK: (&str, &str) = ("minecraft:bedrock", "infiniburn_bit=1");
const END_STONE: (&str, &str) = ("minecraft:end_stone", "");
const AIR: (&str, &str) = ("minecraft:air", "");
const PORTAL: (&str, &str) = ("minecraft:end_portal", "");
const EGG: (&str, &str) = ("minecraft:dragon_egg", "");
/// Each torch's side of the pillar and its `torch_facing_direction`, in BDS's order.
const TORCHES: [([i32; 2], &str); 4] = [([0, -1], "south"), ([0, 1], "north"), ([1, 0], "west"), ([-1, 0], "east")];

type Named = (BlockPos, (&'static str, String));

/// The podium's writes round `origin` (the portal's middle cell), in BDS's order.
pub(super) fn cells([ox, oy, oz]: BlockPos, active: bool) -> Vec<Named> {
    let mut cells: Vec<Named> = Vec::new();
    let mut put = |pos: BlockPos, (name, state): (&'static str, &str)| cells.push((pos, (name, state.to_owned())));
    for dx in -REACH..=REACH {
        for dy in -BELOW..=ABOVE {
            for dz in -REACH..=REACH {
                let (far, pos) = (dx * dx + dy * dy + dz * dz, [ox + dx, oy + dy, oz + dz]);
                match dy {
                    _ if far >= OUTER => {}
                    -1 => put(pos, if far < INNER { BEDROCK } else { END_STONE }),
                    0 if far < INNER => {
                        put(pos, AIR);
                        if active {
                            put(pos, PORTAL);
                        }
                    }
                    0 => put(pos, BEDROCK),
                    _ => put(pos, AIR),
                }
            }
        }
    }
    for up in 0..PILLAR {
        put([ox, oy + up, oz], BEDROCK);
    }
    for ([dx, dz], facing) in TORCHES {
        cells.push(([ox + dx, oy + 2, oz + dz], ("minecraft:torch", format!("torch_facing_direction={facing}"))));
    }
    cells
}

/// Where the portal goes: over the highest solid block of column (0, 0), down past bedrock there
/// as BDS's `spawnExitPortal` goes; `None` while the column is not loaded.
pub(super) fn origin(world: &World, layer: Entity) -> Option<BlockPos> {
    let level = &world.get::<Layer>(layer)?.world;
    let mut y = level.surface(0, 0)? + 1;
    while y > level.heights().start && level.registry().get(level.block(0, y, 0)).is_some_and(|state| state.name == BEDROCK.0) {
        y -= 1;
    }
    Some([0, y, 0])
}

/// The podium round `origin`, lit or not, as written cells.
pub(super) fn ids(world: &World, layer: Entity, origin: BlockPos, active: bool) -> Vec<(BlockPos, u32)> {
    super::resolve(world, layer, &cells(origin, active))
}

/// The dragon egg on top of the pillar of the podium at `origin`.
pub(super) fn egg(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Vec<(BlockPos, u32)> {
    super::resolve(world, layer, &[([x, y + PILLAR, z], (EGG.0, EGG.1.to_owned()))])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(cells: &[Named], pos: BlockPos) -> Vec<&'static str> {
        cells.iter().filter(|(p, _)| *p == pos).map(|(_, (name, _))| name.trim_start_matches("minecraft:")).collect()
    }

    #[test]
    fn the_podium_has_bdss_floor_ring_and_pillar() {
        let cells = cells([0, 62, 0], false);
        assert_eq!(at(&cells, [2, 61, 1]), ["bedrock"], "within 7 below");
        assert_eq!(at(&cells, [2, 61, 2]), ["end_stone"]);
        assert_eq!(at(&cells, [3, 62, 1]), ["bedrock"], "the ring");
        assert_eq!(at(&cells, [2, 62, 2]), ["bedrock"]);
        assert!(at(&cells, [3, 62, 2]).is_empty(), "outside 13");
        assert_eq!(at(&cells, [1, 62, 0]), ["air"]);
        assert_eq!(at(&cells, [0, 62, 0]), ["air", "bedrock"]);
        assert_eq!(at(&cells, [0, 65, 0]), ["air", "bedrock"]);
        assert!(at(&cells, [3, 64, 0]).is_empty(), "the air above narrows");
        let first: Vec<BlockPos> = cells.iter().map(|(pos, _)| *pos).filter(|pos| pos[0] == 0 && pos[2] >= 0).take(4).collect();
        assert_eq!(first, [[0, 61, 0], [0, 61, 1], [0, 61, 2], [0, 61, 3]], "x, then y, then z");
        assert_eq!(cells.last().map(|(pos, _)| *pos), Some([-1, 64, 0]));
        let lit = super::cells([0, 62, 0], true);
        assert_eq!(at(&lit, [0, 62, 0]), ["air", "end_portal", "bedrock"], "BDS sent all three");
        assert_eq!(at(&lit, [1, 62, -2]), ["air", "end_portal"]);
    }
}
