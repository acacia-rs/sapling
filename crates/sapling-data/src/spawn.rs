//! Where players appear, in BDS's two packets about it (`sapling` examples `bds_entity_probe` for
//! the join, `bds_mob_recorder` for `/setworldspawn`).

use acacia_proto::packets::{SetSpawnPosition, SetSpawnPositionSpawnType as SpawnType};
use acacia_proto::types::BlockCoordinates;

/// The dimension BDS names when there is no spawn.
const NO_DIMENSION: i32 = 3;
const OVERWORLD: i32 = 0;

fn unset() -> BlockCoordinates {
    BlockCoordinates { x: i32::MIN, y: i32::MIN, z: i32::MIN }
}

/// What a joining player without a bed is told: it has no spawn of its own.
pub fn no_player_spawn() -> SetSpawnPosition {
    SetSpawnPosition { spawn_type: SpawnType::Player, player_position: unset(), dimension: NO_DIMENSION, world_position: unset() }
}

/// The world's spawn moved to `block`.
pub fn world_spawn(block: [i32; 3]) -> SetSpawnPosition {
    let [x, y, z] = block;
    SetSpawnPosition { spawn_type: SpawnType::World, player_position: BlockCoordinates { x, y, z }, dimension: OVERWORLD, world_position: unset() }
}
