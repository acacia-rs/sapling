//! A sub-chunk record: `version | [layer count] | [y]` then one paletted storage per layer.
//! A storage is `bits << 1` (bit 0 clear: a persistent palette), packed u32 LE indices in XZY
//! order with no index spanning words, an `i32` entry count (absent when `bits` is 0), and the
//! entries as NBT (states.rs).
//!
//! Versions 0 and 2 to 7 are the pre-palette record: 4096 numeric block ids, then 2048 bytes of
//! metas a nibble each (an even index the low nibble), both in XZY order; light may follow and
//! is not read (PocketMine's `LevelDB::deserializeLegacy`). The ids and metas resolve through
//! their 1.12 states (upgrade.rs).

use std::collections::HashMap;

use acacia_proto::codec::{read_li32, read_u8, take};
use acacia_world::SECTION_VOLUME;

use crate::Error;
use crate::states::BlockStates;

const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

/// Runtime ids in XZY order, `(x << 8) | (z << 4) | y`: the order `Chunk::set_section` takes.
pub type Layer = Box<[u32; SECTION_VOLUME]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubChunk {
    /// The section index (world y >> 4) version 9 records carry.
    pub y: Option<i8>,
    /// Blocks, then the liquid a block is waterlogged with; later layers are unused by vanilla.
    pub layers: Vec<Layer>,
}

impl SubChunk {
    pub fn decode(mut bytes: &[u8], states: &BlockStates) -> Result<Self, Error> {
        let r = &mut bytes;
        let (count, y) = match read_u8(r)? {
            0 | 2..=7 => return Ok(Self { y: None, layers: vec![legacy_layer(r, states)?] }),
            1 => (1, None),
            8 => (read_u8(r)?, None),
            9 => (read_u8(r)?, Some(read_u8(r)? as i8)),
            version => return Err(Error::SubChunkVersion(version)),
        };
        let layers = (0..count).map(|_| layer(r, states)).collect::<Result<_, _>>()?;
        Ok(Self { y, layers })
    }

    /// The version 9 record. Fails without `y`, and on a state [`BlockStates::write`] refuses.
    pub fn encode(&self, states: &BlockStates) -> Result<Vec<u8>, Error> {
        let y = self.y.ok_or(Error::SubChunk("a version 9 record carries its y"))?;
        let mut out = vec![9, self.layers.len() as u8, y as u8];
        self.layers.iter().try_for_each(|ids| write_layer(ids, states, &mut out))?;
        Ok(out)
    }
}

fn write_layer(ids: &[u32; SECTION_VOLUME], states: &BlockStates, out: &mut Vec<u8>) -> Result<(), Error> {
    let mut palette = Vec::new();
    let mut slots = HashMap::new();
    let mut slot = |id: u32| {
        *slots.entry(id).or_insert_with(|| {
            palette.push(id);
            palette.len() as u32 - 1
        })
    };
    let indices: Vec<u32> = ids.iter().map(|&id| slot(id)).collect();
    if let [only] = palette[..] {
        out.push(0);
        return states.write(only, out);
    }
    let bits = *VALID_BITS.iter().find(|&&bits| palette.len() <= 1 << bits).expect("4096 entries fit 16 bits");
    out.push(bits << 1);
    for word in indices.chunks(32 / bits as usize) {
        let packed = word.iter().enumerate().fold(0u32, |packed, (i, index)| packed | index << (i * bits as usize));
        out.extend(packed.to_le_bytes());
    }
    out.extend((palette.len() as i32).to_le_bytes());
    palette.iter().try_for_each(|&id| states.write(id, out))
}

fn legacy_layer(r: &mut &[u8], states: &BlockStates) -> Result<Layer, Error> {
    let ids = take(r, SECTION_VOLUME)?;
    let metas = take(r, SECTION_VOLUME / 2)?;
    Ok(Box::new(std::array::from_fn(|i| states.resolve_legacy(ids[i], metas[i / 2] >> (i % 2 * 4) & 0xF))))
}

fn layer(r: &mut &[u8], states: &BlockStates) -> Result<Layer, Error> {
    let header = read_u8(r)?;
    if header & 1 != 0 {
        return Err(Error::SubChunk("a runtime-id palette on disk"));
    }
    let bits = header >> 1;
    if bits == 0 {
        return Ok(Box::new([states.read(r)?; SECTION_VOLUME]));
    }
    if !VALID_BITS.contains(&bits) {
        return Err(Error::SubChunk("an invalid index width"));
    }
    let per_word = 32 / bits as usize;
    let words = take(r, SECTION_VOLUME.div_ceil(per_word) * 4)?;
    let length = read_li32(r)?;
    if !(1..=SECTION_VOLUME as i32).contains(&length) {
        return Err(Error::SubChunk("an invalid palette length"));
    }
    let palette = (0..length).map(|_| states.read(r)).collect::<Result<Vec<u32>, _>>()?;
    let mask = (1u32 << bits) - 1;
    let mut ids = Box::new([palette[0]; SECTION_VOLUME]);
    for (blocks, word) in ids.chunks_mut(per_word).zip(words.chunks_exact(4)) {
        let word = u32::from_le_bytes(word.try_into().expect("four bytes"));
        for (i, id) in blocks.iter_mut().enumerate() {
            // An index past the palette keeps the first entry, as acacia-world's storage reads it.
            if let Some(&entry) = palette.get(((word >> (i * bits as usize)) & mask) as usize) {
                *id = entry;
            }
        }
    }
    Ok(ids)
}

#[cfg(test)]
pub(crate) mod tests {
    use acacia_proto::nbt::Value;
    use acacia_world::BlockRegistry;

    use super::*;
    use crate::states::tests::entry;

    /// A disk storage of palette entries and one palette index per block.
    pub fn storage(palette: &[Vec<u8>], indices: &[usize; SECTION_VOLUME]) -> Vec<u8> {
        if let [only] = palette {
            return [&[0][..], only].concat();
        }
        let bits = *VALID_BITS.iter().find(|&&b| palette.len() <= 1 << b).unwrap();
        let per_word = 32 / bits as usize;
        let mut words = vec![0u32; SECTION_VOLUME.div_ceil(per_word)];
        for (i, &index) in indices.iter().enumerate() {
            words[i / per_word] |= (index as u32) << ((i % per_word) * bits as usize);
        }
        let mut out = vec![bits << 1];
        out.extend(words.iter().flat_map(|w| w.to_le_bytes()));
        out.extend((palette.len() as i32).to_le_bytes());
        out.extend(palette.concat());
        out
    }

    fn states() -> BlockStates {
        BlockStates::new(BlockRegistry::vanilla_arc())
    }

    #[test]
    fn layers_unpack_in_xzy_order_at_every_width() {
        let states = states();
        let registry = states.registry().clone();
        let names = ["minecraft:air", "minecraft:stone", "minecraft:dirt", "minecraft:bedrock", "minecraft:gravel", "minecraft:sand", "minecraft:glass"];
        let ids: Vec<u32> = names.iter().map(|n| registry.states_of(n).next().unwrap().0).collect();
        for distinct in [2, 3, 5, 7] {
            let palette: Vec<Vec<u8>> = names[..distinct].iter().map(|n| entry(n, &[])).collect();
            let indices = std::array::from_fn(|i| (i * 7 + i / 16) % distinct);
            let record = [&[9, 1, 0xfd][..], &storage(&palette, &indices)].concat();
            let decoded = SubChunk::decode(&record, &states).unwrap();
            assert_eq!((decoded.y, decoded.layers.len()), (Some(-3), 1));
            assert!((0..SECTION_VOLUME).all(|i| decoded.layers[0][i] == ids[indices[i]]), "{distinct} entries");
        }
    }

    #[test]
    fn versions_one_and_eight_and_uniform_layers_decode() {
        let states = states();
        let water = entry("minecraft:water", &[("liquid_depth", Value::Int(0))]);
        let stone = entry("minecraft:stone", &[]);
        let uniform = storage(std::slice::from_ref(&stone), &[0; SECTION_VOLUME]);
        let one = SubChunk::decode(&[&[1][..], &uniform].concat(), &states).unwrap();
        assert_eq!((one.y, one.layers.len()), (None, 1));
        let stone_id = states.registry().find("minecraft:stone", "").unwrap();
        assert!(one.layers[0].iter().all(|&id| id == stone_id));

        let wet = storage(&[entry("minecraft:air", &[]), water], &std::array::from_fn(|i| i % 2));
        let eight = SubChunk::decode(&[&[8, 2][..], &uniform, &wet].concat(), &states).unwrap();
        assert_eq!((eight.y, eight.layers.len()), (None, 2));
        let water_id = states.registry().find("minecraft:water", "liquid_depth=0").unwrap();
        assert_eq!((eight.layers[1][0], eight.layers[1][1]), (states.registry().air_id(), water_id));
    }

    #[test]
    fn pre_palette_records_resolve_numeric_ids_and_metas() {
        let states = states();
        let registry = states.registry().clone();
        // Granite (stone:1) at even indices, a birch log (log:2) at odd ones, light after.
        let ids: Vec<u8> = (0..SECTION_VOLUME).map(|i| if i % 2 == 0 { 1 } else { 17 }).collect();
        let record = [&[0][..], &ids, &[0x21; SECTION_VOLUME / 2], &[0xff; SECTION_VOLUME]].concat();
        for version in [0, 2, 7] {
            let decoded = SubChunk::decode(&[&[version][..], &record[1..]].concat(), &states).unwrap();
            assert_eq!((decoded.y, decoded.layers.len()), (None, 1));
            let granite = registry.find("minecraft:granite", "").unwrap();
            let birch = registry.find("minecraft:birch_log", "pillar_axis=y").unwrap();
            assert_eq!((decoded.layers[0][0], decoded.layers[0][1], decoded.layers[0][4095]), (granite, birch, birch), "version {version}");
        }
    }

    #[test]
    fn encoded_records_decode_to_the_same_layers_at_every_width() {
        let states = states();
        let count = states.registry().len();
        for distinct in [1, 2, 3, 5, 9, 17, 33, 65, 257, 4096] {
            let blocks: Layer = Box::new(std::array::from_fn(|i| ((i * 31 + i / 16) % distinct * 7 % count) as u32));
            let section = SubChunk { y: Some(-4), layers: vec![blocks.clone(), blocks] };
            let record = section.encode(&states).unwrap();
            assert_eq!(record[..3], [9, 2, 0xfc]);
            assert_eq!(SubChunk::decode(&record, &states).unwrap(), section, "{distinct} entries");
        }
        assert!(SubChunk { y: None, layers: Vec::new() }.encode(&states).is_err());
    }

    #[test]
    fn malformed_records_are_errors_not_panics() {
        let states = states();
        let stone = entry("minecraft:stone", &[]);
        let good = [&[9, 1, 0][..], &storage(&[stone.clone(), entry("minecraft:dirt", &[])], &[0; SECTION_VOLUME])].concat();
        assert!(SubChunk::decode(&good, &states).is_ok());
        assert!(matches!(SubChunk::decode(&[10, 1, 0], &states), Err(Error::SubChunkVersion(10))));
        assert!(SubChunk::decode(&[7, 1, 0], &states).is_err(), "a cut pre-palette record");
        assert!(matches!(SubChunk::decode(&[9, 1, 0, 3], &states), Err(Error::SubChunk(_))), "a network palette");
        assert!(matches!(SubChunk::decode(&[9, 1, 0, 7 << 1], &states), Err(Error::SubChunk(_))), "7 bits");
        for cut in [1, 2, 3, 100, good.len() - 1] {
            assert!(SubChunk::decode(&good[..cut], &states).is_err(), "cut at {cut}");
        }
    }
}
