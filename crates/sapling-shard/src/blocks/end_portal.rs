//! End portal frames: an eye put in the last frame of a ring of twelve, each with its eye and
//! facing the 3 x 3 inside, fills the inside with end portal blocks (research/captures/
//! bds-end-frame.txt; docs/research/end-dimension.md). Travel through them: portals/end.rs.

use acacia_proto::packets::UpdateBlock;
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send};

pub(crate) const END_PORTAL: &str = "minecraft:end_portal";
const FRAME: &str = "minecraft:end_portal_frame";
/// What BDS sent the filled frame and the portal with: the network bit alone.
const NETWORK: UpdateBlockFlags = UpdateBlockFlags::NETWORK;
/// The way each `cardinal_direction` faces, on x and z.
const FACINGS: [(&str, [i32; 2]); 4] = [("south", [0, 1]), ("north", [0, -1]), ("east", [1, 0]), ("west", [-1, 0])];

/// The way a frame faces, if `pos` holds one with its eye in.
fn filled_facing(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<[i32; 2]> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| state.name == FRAME)?;
    let has = |pair: &str| state.properties.split(',').any(|one| one == pair);
    if !has("end_portal_eye_bit=1") {
        return None;
    }
    FACINGS.iter().find(|(name, _)| has(&format!("minecraft:cardinal_direction={name}"))).map(|(_, facing)| *facing)
}

/// The twelve frames round the 3 x 3 whose middle is `centre`, each with the way it must face.
fn ring([x, y, z]: BlockPos) -> impl Iterator<Item = (BlockPos, [i32; 2])> {
    (-1..=1).flat_map(move |i| [([x + i, y, z - 2], [0, 1]), ([x + i, y, z + 2], [0, -1]), ([x - 2, y, z + i], [1, 0]), ([x + 2, y, z + i], [-1, 0])])
}

/// The middle of the inside a whole ring round it makes, of the rings `frame` can be in.
fn whole_ring(level: &sapling_world::World, frame: BlockPos) -> Option<BlockPos> {
    let [dx, dz] = filled_facing(level, frame)?;
    let [x, y, z] = frame;
    // The frame is one of three on its side; the inside starts in front of it.
    let centres = (-1..=1).map(move |k| [x + 2 * dx + k * dz.abs(), y, z + 2 * dz + k * dx.abs()]);
    centres.into_iter().find(|centre| ring(*centre).all(|(pos, facing)| filled_facing(level, pos) == Some(facing)))
}

/// Writes the eye `filled` into the frame at `pos` and shows it: alone as BDS does, or, when it
/// completes a ring, in one update with the portal it makes (x then z, from the far corner),
/// followed by the portal's sound. False when nothing was written.
pub(super) fn fill(world: &mut World, layer: Entity, pos: BlockPos, filled: u32) -> bool {
    if !send::set_quietly(world, layer, pos, filled) {
        return false;
    }
    let Some(level) = level(world, layer) else { return false };
    let (portal, centre) = (level.registry().find(END_PORTAL, ""), whole_ring(level, pos));
    let (Some(portal), Some([cx, cy, cz])) = (portal, centre) else {
        let [x, y, z] = pos;
        let shown = sapling_net::encode(&UpdateBlock { position: BlockCoordinates { x, y, z }, block_runtime_id: level.registry().network_hash(filled), flags: NETWORK, layer: 0 });
        send::to_viewers(world, layer, pos, &[shown]);
        return true;
    };
    let inside = (-1..=1).rev().flat_map(|dx| (-1..=1).rev().map(move |dz| ([cx + dx, cy, cz + dz], portal)));
    let written: Vec<(BlockPos, u32)> = std::iter::once((pos, filled)).chain(inside.filter(|(at, id)| send::set_quietly(world, layer, *at, *id))).collect();
    super::portals::show_in_sections(world, layer, &written, NETWORK.0);
    let sound = send::noise("block.end_portal.spawn", Vec3f { x: cx as f32 + 0.5, y: cy as f32 + 1.0, z: cz as f32 + 0.5 }, -1);
    send::to_viewers(world, layer, [cx, cy, cz], &[sound]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_is_twelve_frames_facing_in() {
        let mut level = sapling_world::World::void();
        let registry = sapling_world::BlockRegistry::vanilla();
        let frame = |facing: &str, eye: u8| registry.find(FRAME, &format!("end_portal_eye_bit={eye},minecraft:cardinal_direction={facing}")).unwrap();
        let named = |facing: [i32; 2]| FACINGS.iter().find(|(_, way)| *way == facing).unwrap().0;
        let cells: Vec<(BlockPos, u32)> = ring([11, 64, 11]).map(|(pos, facing)| (pos, frame(named(facing), 1))).collect();
        for (pos, id) in &cells {
            level.set_block(pos[0], pos[1], pos[2], *id);
        }
        assert_eq!(whole_ring(&level, [11, 64, 9]), Some([11, 64, 11]));
        assert_eq!(whole_ring(&level, [9, 64, 12]), Some([11, 64, 11]), "from a corner's neighbour");
        level.set_block(13, 64, 10, frame("east", 1));
        assert_eq!(whole_ring(&level, [11, 64, 9]), None, "one facing out");
        level.set_block(13, 64, 10, frame("west", 0));
        assert_eq!(whole_ring(&level, [11, 64, 9]), None, "one without its eye");
    }
}
