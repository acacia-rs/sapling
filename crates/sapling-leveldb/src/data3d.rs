//! The Data3D record: a 256-entry heightmap, then one biome storage per section, bottom first.
//! Biome storages have the block storage layout with `i32` biome ids as palette entries; the
//! header 0xff repeats the storage below.

use acacia_proto::codec::{read_li16, read_li32, read_u8, read_zigzag32, take, write_zigzag32};
use acacia_world::{BlockRegistry, Chunk};
use bytes::{BufMut, BytesMut};

use crate::Error;

const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];
const COPY_BELOW: u8 = 0xff;
/// Bit 0 of a storage header on the wire: the palette holds ids, not NBT.
const NETWORK: u8 = 1;
const VOLUME: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data3d {
    /// Per column at index `(z << 4) | x`: blocks from the dimension's floor up to and including
    /// the highest one that dims light (plants do not, water does); 0 for an empty column.
    pub heightmap: Box<[i16; 256]>,
    /// The biome storages re-encoded as the network writes them: what `Chunk::set_biomes` reads.
    pub biomes: Vec<u8>,
}

impl Data3d {
    pub fn decode(mut bytes: &[u8]) -> Result<Self, Error> {
        let r = &mut bytes;
        let mut heightmap = Box::new([0; 256]);
        for height in heightmap.iter_mut() {
            *height = read_li16(r)?;
        }
        let mut biomes = BytesMut::new();
        while !r.is_empty() {
            storage(r, &mut biomes)?;
        }
        Ok(Self { heightmap, biomes: biomes.to_vec() })
    }

    /// What `chunk` saves: its biomes, and its heightmap by the rule on [`Data3d::heightmap`].
    pub fn of(chunk: &Chunk, registry: &BlockRegistry) -> Self {
        let heights = chunk.column_heights(&|id| registry.get(id).is_some_and(|state| state.light_filter > 0));
        let min_y = chunk.dimension().min_y;
        let heightmap = std::array::from_fn(|i| (heights.get(i as i32 & 15, i as i32 >> 4) - min_y) as i16);
        Self { heightmap: Box::new(heightmap), biomes: chunk.biome_bytes() }
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut out: Vec<u8> = self.heightmap.iter().flat_map(|height| height.to_le_bytes()).collect();
        let r = &mut &self.biomes[..];
        while !r.is_empty() {
            disk_storage(r, &mut out)?;
        }
        Ok(out)
    }

    /// The y of the highest counted block of column `(x, z)` (taken modulo 16), `None` when it is empty.
    pub fn top(&self, x: i32, z: i32, min_y: i32) -> Option<i32> {
        let height = i32::from(self.heightmap[(((z & 15) << 4) | (x & 15)) as usize]);
        (height > 0).then_some(min_y + height - 1)
    }
}

fn storage(r: &mut &[u8], out: &mut BytesMut) -> Result<(), Error> {
    let header = read_u8(r)?;
    let bits = header >> 1;
    if header == COPY_BELOW {
        out.put_u8(COPY_BELOW);
        return Ok(());
    }
    out.put_u8(header | NETWORK);
    if bits == 0 {
        write_zigzag32(out, read_li32(r)?);
        return Ok(());
    }
    if !VALID_BITS.contains(&bits) {
        return Err(Error::Biomes("an invalid index width"));
    }
    out.put_slice(take(r, VOLUME.div_ceil(32 / bits as usize) * 4)?);
    let length = read_li32(r)?;
    if !(1..=VOLUME as i32).contains(&length) {
        return Err(Error::Biomes("an invalid palette length"));
    }
    write_zigzag32(out, length);
    for _ in 0..length {
        write_zigzag32(out, read_li32(r)?);
    }
    Ok(())
}

/// The inverse of [`storage`]: one network biome storage off `r`, appended in the disk form.
/// The header keeps bit 0: BDS sets it on disk too (tests/world.rs).
fn disk_storage(r: &mut &[u8], out: &mut Vec<u8>) -> Result<(), Error> {
    let header = read_u8(r)?;
    out.push(header);
    if header == COPY_BELOW {
        return Ok(());
    }
    let bits = header >> 1;
    if bits == 0 {
        out.extend(read_zigzag32(r)?.to_le_bytes());
        return Ok(());
    }
    if !VALID_BITS.contains(&bits) {
        return Err(Error::Biomes("an invalid index width"));
    }
    out.extend_from_slice(take(r, VOLUME.div_ceil(32 / bits as usize) * 4)?);
    let length = read_zigzag32(r)?;
    if !(1..=VOLUME as i32).contains(&length) {
        return Err(Error::Biomes("an invalid palette length"));
    }
    out.extend(length.to_le_bytes());
    for _ in 0..length {
        out.extend(read_zigzag32(r)?.to_le_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use acacia_world::Dimension;

    use super::*;

    fn record(storages: &[u8]) -> Vec<u8> {
        let heights = (0..256i16).flat_map(|i| (i + 60).to_le_bytes());
        heights.chain(storages.iter().copied()).collect()
    }

    #[test]
    fn biomes_reach_a_chunk_through_the_network_form() {
        // Section 0: biome 4 everywhere. Section 1: 1-bit storage, biome 7 where x is odd.
        let mut storages = vec![NETWORK];
        storages.extend(4i32.to_le_bytes());
        storages.push(1 << 1 | NETWORK);
        let odd_x: Vec<u32> = (0..128).map(|word| if (word / 8) % 2 == 1 { u32::MAX } else { 0 }).collect();
        storages.extend(odd_x.iter().flat_map(|w| w.to_le_bytes()));
        storages.extend(2i32.to_le_bytes());
        storages.extend([4i32, 7].iter().flat_map(|id| id.to_le_bytes()));
        storages.push(COPY_BELOW);

        let data = Data3d::decode(&record(&storages)).unwrap();
        assert_eq!(data.encode().unwrap(), record(&storages), "the record is written back as it was read");
        assert_eq!((data.top(0, 0, -64), data.top(3, 1, -64), data.top(19, 17, -64)), (Some(-5), Some(14), Some(14)));
        let mut chunk = Chunk::empty(0, 0, Dimension::overworld(BlockRegistry::vanilla().air_id()));
        chunk.set_biomes(&data.biomes);
        assert_eq!((chunk.biome(0, -64, 0), chunk.biome(15, -50, 3)), (Some(4), Some(4)));
        assert_eq!((chunk.biome(2, -48, 9), chunk.biome(3, -48, 9), chunk.biome(3, -20, 9)), (Some(4), Some(7), Some(7)));
        assert_eq!(chunk.biome(0, 0, 0), None, "only three sections were stored");
    }

    #[test]
    fn empty_columns_and_damage() {
        let mut flat = vec![0u8; 512];
        flat.extend([0, 1, 0, 0, 0]);
        let data = Data3d::decode(&flat).unwrap();
        assert_eq!((data.top(5, 5, -64), data.biomes.as_slice()), (None, &[1, 2][..]));
        assert!(Data3d::decode(&flat[..100]).is_err());
        assert!(Data3d::decode(&flat[..514]).is_err());
        assert!(matches!(Data3d::decode(&record(&[7 << 1])), Err(Error::Biomes(_))));
    }
}
