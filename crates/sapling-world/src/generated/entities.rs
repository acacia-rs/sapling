//! Generated block entities (spawners, loot containers, trial spawners, vaults) as BDS saves them.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bytes::BytesMut;
use sapling_leveldb::{generated_loot, generated_pot, generated_spawner, generated_trial_spawner, generated_vault};
use sapling_worldgen::{BlockEntity, ChunkShape};

/// `entity` of the block `block` (its name) at world position `pos`.
pub(crate) fn block_entity_nbt(pos: [i32; 3], block: &str, entity: BlockEntity) -> Option<Value> {
    Some(match entity {
        BlockEntity::Spawner(mob) => generated_spawner(pos, mob),
        BlockEntity::Loot { table, seed } => generated_loot(pos, block, table, seed)?,
        BlockEntity::Pot { table, seed, sherds } => generated_pot(pos, table, seed, sherds),
        BlockEntity::TrialSpawner { normal, ominous } => generated_trial_spawner(pos, normal, ominous),
        BlockEntity::Vault { table, key } => generated_vault(pos, table, key),
    })
}

/// The block entities of the generated chunk at `(chunk_x, chunk_z)`, as a `BlockEntities` record.
pub(crate) fn generated_block_entities(chunk_x: i32, chunk_z: i32, shape: &ChunkShape) -> Vec<u8> {
    let mut out = BytesMut::new();
    for ([x, y, z], entity) in shape.block_entities() {
        let block = shape.block(x as usize, y, z as usize);
        if let Some(value) = block_entity_nbt([chunk_x * 16 + x, y, chunk_z * 16 + z], block.name(), entity) {
            nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value });
        }
    }
    out.to_vec()
}
