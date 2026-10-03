//! Chunk keys: `x:i32 | z:i32 | [dimension:i32, absent for the overworld] | tag:u8 | [y:i8]`,
//! little-endian (research/worlds-and-ecs.md "Chunk keys").

/// The record kinds of a chunk column this crate reads; the full list is in the research note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    /// Heightmap and per-section biomes.
    Data3d = 0x2b,
    /// One byte: the chunk format version. Its presence marks the column as saved.
    Version = 0x2c,
    /// One section; the key continues with the section's y index.
    SubChunk = 0x2f,
    /// Concatenated little-endian NBT compounds.
    BlockEntities = 0x31,
    PendingTicks = 0x33,
    FinalizedState = 0x36,
    /// Where worlds older than 1.16.100 keep the version byte.
    LegacyVersion = 0x76,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub x: i32,
    pub z: i32,
    /// 0 overworld, 1 nether, 2 end.
    pub dimension: i32,
}

/// A chunk key taken apart. `tag` is raw: worlds hold kinds [`Tag`] does not list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedKey {
    pub chunk: ChunkKey,
    pub tag: u8,
    /// The section index of a [`Tag::SubChunk`] key.
    pub sub_chunk: Option<i8>,
}

impl ChunkKey {
    pub const fn new(x: i32, z: i32, dimension: i32) -> Self {
        Self { x, z, dimension }
    }

    fn prefix(&self) -> Vec<u8> {
        let mut key = Vec::with_capacity(14);
        key.extend_from_slice(&self.x.to_le_bytes());
        key.extend_from_slice(&self.z.to_le_bytes());
        if self.dimension != 0 {
            key.extend_from_slice(&self.dimension.to_le_bytes());
        }
        key
    }

    pub fn tag(&self, tag: Tag) -> Vec<u8> {
        let mut key = self.prefix();
        key.push(tag as u8);
        key
    }

    /// The key of section `y` (world y >> 4).
    pub fn sub_chunk(&self, y: i8) -> Vec<u8> {
        let mut key = self.tag(Tag::SubChunk);
        key.push(y as u8);
        key
    }

    /// Takes a chunk key apart; `None` for the database's other keys (players, villages, ...).
    pub fn parse(key: &[u8]) -> Option<ParsedKey> {
        let int = |at: usize| i32::from_le_bytes(key[at..at + 4].try_into().expect("four bytes"));
        let (dimension, rest) = match key.len() {
            9 | 10 => (0, &key[8..]),
            13 | 14 => (int(8), &key[12..]),
            _ => return None,
        };
        let tag = rest[0];
        let sub_chunk = rest.get(1).map(|&y| y as i8);
        // Text keys of these lengths ("mobevents", "~local_player") end in letters, not tags.
        let is_tag = matches!(tag, 0x2b..=0x41 | 0x76..=0x78);
        let shaped = sub_chunk.is_some() == (tag == Tag::SubChunk as u8);
        (is_tag && shaped && (key.len() < 13 || matches!(dimension, 1 | 2))).then_some(ParsedKey { chunk: ChunkKey { x: int(0), z: int(4), dimension }, tag, sub_chunk })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_the_vanilla_layout() {
        let overworld = ChunkKey::new(1, -2, 0);
        assert_eq!(overworld.tag(Tag::Version), [1, 0, 0, 0, 0xfe, 0xff, 0xff, 0xff, 0x2c]);
        assert_eq!(overworld.sub_chunk(-4), [1, 0, 0, 0, 0xfe, 0xff, 0xff, 0xff, 0x2f, 0xfc]);
        let nether = ChunkKey::new(3, 4, 1);
        assert_eq!(nether.tag(Tag::Data3d), [3, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 0x2b]);
        assert_eq!(nether.sub_chunk(7).len(), 14);
    }

    #[test]
    fn parsing_inverts_building_and_skips_other_keys() {
        for chunk in [ChunkKey::new(1, -2, 0), ChunkKey::new(-300, 77, 1), ChunkKey::new(0, 0, 2)] {
            let version = ChunkKey::parse(&chunk.tag(Tag::Version)).unwrap();
            assert_eq!(version, ParsedKey { chunk, tag: 0x2c, sub_chunk: None });
            let section = ChunkKey::parse(&chunk.sub_chunk(-4)).unwrap();
            assert_eq!(section, ParsedKey { chunk, tag: 0x2f, sub_chunk: Some(-4) });
        }
        for other in ["mobevents", "scoreboard", "Overworld", "BiomeData", "~local_player", "portals", "AutonomousEntities"] {
            assert_eq!(ChunkKey::parse(other.as_bytes()), None, "{other}");
        }
        let actor_digest = [b"digp".as_slice(), &ChunkKey::new(1, 2, 0).prefix()].concat();
        assert_eq!(ChunkKey::parse(&actor_digest), None);
    }
}
