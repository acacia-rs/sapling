//! Raids, as BDS keeps them: each under `VILLAGE_<village uuid>_RAID` (`Village::_saveVillageRaid`;
//! read back by `VillageManager::loadAllVillages`, 0x142a52630), its compound written by the shard.
//! Other `VILLAGE_` records (BDS's `_INFO`, `_DWELLERS`, `_PLAYERS`, `_POI`) are left as they are.

pub const VILLAGE_PREFIX: &[u8] = b"VILLAGE_";
pub const RAID_SUFFIX: &[u8] = b"_RAID";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidRecord {
    /// The village's uuid, as text.
    pub village: String,
    /// The raid's little-endian NBT compound.
    pub compound: Vec<u8>,
}

impl RaidRecord {
    pub fn key(&self) -> Vec<u8> {
        [VILLAGE_PREFIX, self.village.as_bytes(), RAID_SUFFIX].concat()
    }

    /// The record kept under `key`, if it is a raid's.
    pub fn decode(key: &[u8], value: &[u8]) -> Option<Self> {
        let village = key.strip_prefix(VILLAGE_PREFIX)?.strip_suffix(RAID_SUFFIX)?;
        Some(Self { village: String::from_utf8(village.to_vec()).ok()?, compound: value.to_vec() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_raid_keys_are_raids() {
        let raid = RaidRecord { village: "0000000a-0000-0000-0000-000000000001".into(), compound: vec![10, 0, 0, 0] };
        assert_eq!(RaidRecord::decode(&raid.key(), &raid.compound), Some(raid.clone()));
        assert_eq!(RaidRecord::decode(b"VILLAGE_0000000a-0000-0000-0000-000000000001_INFO", &[]), None);
    }
}
