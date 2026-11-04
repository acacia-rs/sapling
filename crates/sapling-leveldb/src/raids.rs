//! Village records, as BDS keeps them: each under `VILLAGE_<village uuid>_<part>`, read back by
//! `VillageManager::loadAllVillages` (0x142a52630), their compounds written by the shard. sapling
//! keeps a village's `RAID` (`Village::_saveVillageRaid`) and `PLAYERS` (the standings, BDS's
//! 0x142a2c140: `Players`, each `ID` and `S`); BDS's `_INFO`, `_DWELLERS` and `_POI` are left as
//! they are.

pub const VILLAGE_PREFIX: &[u8] = b"VILLAGE_";

/// The part of a village a record keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VillagePart {
    Raid,
    Players,
}

impl VillagePart {
    const ALL: [Self; 2] = [Self::Raid, Self::Players];

    fn suffix(self) -> &'static [u8] {
        match self {
            Self::Raid => b"_RAID",
            Self::Players => b"_PLAYERS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VillageRecord {
    /// The village's uuid, as text.
    pub village: String,
    pub part: VillagePart,
    /// The part's little-endian NBT compound.
    pub compound: Vec<u8>,
}

impl VillageRecord {
    pub fn key(&self) -> Vec<u8> {
        [VILLAGE_PREFIX, self.village.as_bytes(), self.part.suffix()].concat()
    }

    /// The record kept under `key`, if it is of a part sapling keeps.
    pub fn decode(key: &[u8], value: &[u8]) -> Option<Self> {
        let rest = key.strip_prefix(VILLAGE_PREFIX)?;
        let (village, part) = VillagePart::ALL.into_iter().find_map(|part| rest.strip_suffix(part.suffix()).map(|village| (village, part)))?;
        Some(Self { village: String::from_utf8(village.to_vec()).ok()?, part, compound: value.to_vec() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_kept_parts_are_read() {
        let raid = VillageRecord { village: "0000000a-0000-0000-0000-000000000001".into(), part: VillagePart::Raid, compound: vec![10, 0, 0, 0] };
        assert_eq!(VillageRecord::decode(&raid.key(), &raid.compound), Some(raid.clone()));
        let players = VillageRecord { part: VillagePart::Players, ..raid.clone() };
        assert_eq!(VillageRecord::decode(b"VILLAGE_0000000a-0000-0000-0000-000000000001_PLAYERS", &raid.compound), Some(players));
        assert_eq!(VillageRecord::decode(b"VILLAGE_0000000a-0000-0000-0000-000000000001_INFO", &[]), None);
    }
}
