//! A column's structure spawn areas, chunk tag 0x77: the boxes BDS 1.26 keeps per structure that
//! reaches the column (its whole box and each piece's, clipped to the column), which natural
//! spawning asks (docs/research/structure-spawns.md). Little-endian, read off BDS saves:
//! `u32 1`; the names (`u32` count, then `u32` index, `u16` length, bytes); the boxes (`u32` count,
//! then per area from the last made down `u32` id and six `i32`); the data-driven structures' areas
//! (`u32` count, then `u32` id, `u32` name index, `u32` whole); the hardcoded ones' (`u32` count,
//! then `u32` id, `u32` name index, `i32` y offset, `u32` whole).

use crate::Error;

const VERSION: u32 = 1;

/// One area of a column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnArea {
    /// `minecraft:swamp_hut`, `minecraft:fortress`, ...
    pub structure: String,
    /// The inclusive box, `[x0, y0, z0, x1, y1, z1]`, within the column.
    pub bounds: [i32; 6],
    /// The structure's whole box; else one piece's.
    pub whole: bool,
    /// Where below a piece's top the spawner starts looking, for the hardcoded structures
    /// (fortress, monument, swamp hut, outpost, ...); `None` for data-driven ones.
    pub y_offset: Option<i32>,
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        if self.0.len() < n {
            return Err(Error::SpawnAreas("record ends early"));
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
}

/// The areas of a tag 0x77 record, in the order BDS made them (whole box first).
pub fn decode_spawn_areas(record: &[u8]) -> Result<Vec<SpawnArea>, Error> {
    let mut read = Reader(record);
    if read.u32()? != VERSION {
        return Err(Error::SpawnAreas("unknown version"));
    }
    let mut names = Vec::new();
    for _ in 0..read.u32()? {
        let index = read.u32()?;
        let length = u16::from_le_bytes(read.take(2)?.try_into().expect("two bytes")) as usize;
        names.push((index, String::from_utf8_lossy(read.take(length)?).into_owned()));
    }
    let mut boxes = Vec::new();
    for _ in 0..read.u32()? {
        let id = read.u32()?;
        let mut bounds = [0; 6];
        for value in &mut bounds {
            *value = read.i32()?;
        }
        boxes.push((id, bounds));
    }
    let mut kinds = Vec::new();
    for hardcoded in [false, true] {
        for _ in 0..read.u32()? {
            let (id, name) = (read.u32()?, read.u32()?);
            let y_offset = if hardcoded { Some(read.i32()?) } else { None };
            kinds.push((id, name, y_offset, read.u32()? != 0));
        }
    }
    let mut areas: Vec<(u32, SpawnArea)> = Vec::new();
    for (id, bounds) in boxes {
        let Some(&(_, name, y_offset, whole)) = kinds.iter().find(|kind| kind.0 == id) else { continue };
        let Some((_, structure)) = names.iter().find(|(index, _)| *index == name) else { return Err(Error::SpawnAreas("unnamed structure")) };
        areas.push((id, SpawnArea { structure: structure.clone(), bounds, whole, y_offset }));
    }
    areas.sort_by_key(|(id, _)| *id);
    Ok(areas.into_iter().map(|(_, area)| area).collect())
}

/// `areas` as BDS saves them: ids from 1 in the given order, names indexed as first met; empty for none.
pub fn encode_spawn_areas(areas: &[SpawnArea]) -> Vec<u8> {
    if areas.is_empty() {
        return Vec::new();
    }
    let mut names: Vec<&str> = Vec::new();
    for area in areas {
        if !names.contains(&area.structure.as_str()) {
            names.push(&area.structure);
        }
    }
    let index = |area: &SpawnArea| names.iter().position(|name| *name == area.structure).expect("listed above") as u32;
    let mut out = Vec::new();
    let mut put = |value: u32| out.extend_from_slice(&value.to_le_bytes());
    put(VERSION);
    put(names.len() as u32);
    let mut text = Vec::new();
    for (i, name) in names.iter().enumerate() {
        text.extend_from_slice(&(i as u32).to_le_bytes());
        text.extend_from_slice(&(name.len() as u16).to_le_bytes());
        text.extend_from_slice(name.as_bytes());
    }
    out.extend_from_slice(&text);
    let numbered: Vec<(u32, &SpawnArea)> = areas.iter().enumerate().rev().map(|(i, area)| (i as u32 + 1, area)).collect();
    let mut put = |value: u32| out.extend_from_slice(&value.to_le_bytes());
    put(numbered.len() as u32);
    for (id, area) in &numbered {
        put(*id);
        area.bounds.iter().for_each(|&value| put(value as u32));
    }
    for hardcoded in [false, true] {
        let kind: Vec<&(u32, &SpawnArea)> = numbered.iter().filter(|(_, area)| area.y_offset.is_some() == hardcoded).collect();
        put(kind.len() as u32);
        for (id, area) in kind {
            put(*id);
            put(index(area));
            if let Some(offset) = area.y_offset {
                put(offset as u32);
            }
            put(u32::from(area.whole));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace().map(|byte| u8::from_str_radix(byte, 16).unwrap()).collect()
    }

    /// Chunk 240,-114 of a BDS 1.26.50 world of seed 20261003 (docs/research/structure-spawns.md).
    const HUT: &str = "01 00 00 00 01 00 00 00 00 00 00 00 13 00 6d 69 6e 65 63 72 61 66 74 3a 73 77 61 6d 70 5f 68 75 74 02 00 00 00 02 00 00 00 00 0f 00 00 46 00 00 00 e0 f8 ff ff 06 0f 00 00 4c 00 00 00 e8 f8 ff ff 01 00 00 00 00 0f 00 00 40 00 00 00 e0 f8 ff ff 06 0f 00 00 46 00 00 00 e8 f8 ff ff 00 00 00 00 02 00 00 00 02 00 00 00 00 00 00 00 fd ff ff ff 00 00 00 00 01 00 00 00 00 00 00 00 fd ff ff ff 01 00 00 00";
    /// Chunk 114,-67 of the same world: an abandoned camp, a data-driven structure.
    const CAMP: &str = "01 00 00 00 01 00 00 00 00 00 00 00 1f 00 6d 69 6e 65 63 72 61 66 74 3a 61 62 61 6e 64 6f 6e 65 64 5f 63 61 6d 70 5f 6d 65 61 64 6f 77 02 00 00 00 02 00 00 00 2b 07 00 00 74 00 00 00 d9 fb ff ff 2f 07 00 00 7b 00 00 00 df fb ff ff 01 00 00 00 2b 07 00 00 74 00 00 00 d8 fb ff ff 2f 07 00 00 7b 00 00 00 df fb ff ff 02 00 00 00 02 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00";

    #[test]
    fn bds_records_read_and_write_back_byte_for_byte() {
        let hut = decode_spawn_areas(&hex(HUT)).unwrap();
        let area = |bounds, whole| SpawnArea { structure: "minecraft:swamp_hut".into(), bounds, whole, y_offset: Some(-3) };
        assert_eq!(hut, [area([3840, 64, -1824, 3846, 70, -1816], true), area([3840, 70, -1824, 3846, 76, -1816], false)]);
        assert_eq!(encode_spawn_areas(&hut), hex(HUT));
        let camp = decode_spawn_areas(&hex(CAMP)).unwrap();
        assert_eq!(camp.len(), 2);
        assert!(camp[0].whole && camp[0].y_offset.is_none() && !camp[1].whole);
        assert_eq!(encode_spawn_areas(&camp), hex(CAMP));
    }

    #[test]
    fn two_structures_share_one_record() {
        let area = |structure: &str, whole, y_offset| SpawnArea { structure: structure.into(), bounds: [0, 1, 2, 3, 4, 5], whole, y_offset };
        let areas = vec![area("minecraft:monument", true, Some(0)), area("minecraft:trial_chambers", true, None), area("minecraft:monument", false, Some(0))];
        assert_eq!(decode_spawn_areas(&encode_spawn_areas(&areas)).unwrap(), areas);
        assert!(encode_spawn_areas(&[]).is_empty());
        assert!(decode_spawn_areas(&[1, 0, 0]).is_err());
    }
}
