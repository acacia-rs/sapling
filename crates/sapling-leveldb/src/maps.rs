//! Maps, each under its own key `map_<id>`: a compound of the map's 128 x 128 `colors` (RGBA bytes),
//! `decorations`, `dimension`, `fullyExplored`, `height`, `mapId`, `mapLocked`, `parentMapId` (the
//! map a scale up, id + 1; -1 at scale 4), `scale`, `unlimitedTracking`, `width`, `xCenter` and `zCenter`,
//! as BDS 1.26 saved them (a BDS world's `map_-708669603834`..`838`: one map's five scales, the
//! four above made blank with it). An explorer map's record keeps its pin as its one decoration and
//! `unlimitedTracking` 1 (bds-ax1maps.txt's world: `data` {rot 8, type 21, x -76, y -28}, `key`
//! {blockX -984, blockY 64, blockZ 136, type 1}); players' markers are not kept.

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

pub const MAP_PREFIX: &[u8] = b"map_";
const SIDE: i16 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapRecord {
    pub id: i64,
    pub parent: i64,
    pub dimension: i8,
    pub scale: i8,
    pub locked: bool,
    /// Block x and z of its centre.
    pub centre: [i32; 2],
    /// R, G, B, A of each pixel, rows from the north.
    pub colours: Vec<u8>,
    pub pin: Option<MapPin>,
}

/// An explorer map's structure: its marker (type, rotation and place in half pixels from the
/// middle) and the block it tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPin {
    pub kind: i32,
    pub rotation: i32,
    pub x: i32,
    pub y: i32,
    pub block: [i32; 3],
}

/// `key`'s `type` for a tracked block.
const BLOCK_KEY: i32 = 1;

impl MapPin {
    fn encode(&self) -> Value {
        let [x, y, z] = self.block;
        let data = Value::Compound(vec![("rot".into(), Value::Int(self.rotation)), ("type".into(), Value::Int(self.kind)), ("x".into(), Value::Int(self.x)), ("y".into(), Value::Int(self.y))]);
        let key = Value::Compound(vec![("blockX".into(), Value::Int(x)), ("blockY".into(), Value::Int(y)), ("blockZ".into(), Value::Int(z)), ("type".into(), Value::Int(BLOCK_KEY))]);
        Value::Compound(vec![("data".into(), data), ("key".into(), key)])
    }

    /// The first decoration tracking a block; `None` when none does.
    fn decode(decorations: Option<&Value>) -> Option<Self> {
        let Some(Value::List(list)) = decorations else { return None };
        list.items.iter().find_map(|decoration| {
            let int = |part: &str, key: &str| match decoration.get(part)?.get(key)? {
                Value::Int(value) => Some(*value),
                _ => None,
            };
            (int("key", "type")? == BLOCK_KEY).then_some(())?;
            Some(Self { kind: int("data", "type")?, rotation: int("data", "rot")?, x: int("data", "x")?, y: int("data", "y")?, block: [int("key", "blockX")?, int("key", "blockY")?, int("key", "blockZ")?] })
        })
    }
}

impl MapRecord {
    pub fn key(&self) -> Vec<u8> {
        [MAP_PREFIX, self.id.to_string().as_bytes()].concat()
    }

    pub fn encode(&self) -> Vec<u8> {
        let root = Value::Compound(vec![
            ("colors".into(), Value::ByteArray(self.colours.clone())),
            ("decorations".into(), Value::List(List { tag: if self.pin.is_some() { 10 } else { 0 }, items: self.pin.iter().map(MapPin::encode).collect() })),
            ("dimension".into(), Value::Byte(self.dimension)),
            ("fullyExplored".into(), Value::Byte(0)),
            ("height".into(), Value::Short(SIDE)),
            ("mapId".into(), Value::Long(self.id)),
            ("mapLocked".into(), Value::Byte(i8::from(self.locked))),
            ("parentMapId".into(), Value::Long(self.parent)),
            ("scale".into(), Value::Byte(self.scale)),
            ("unlimitedTracking".into(), Value::Byte(i8::from(self.pin.is_some()))),
            ("width".into(), Value::Short(SIDE)),
            ("xCenter".into(), Value::Int(self.centre[0])),
            ("zCenter".into(), Value::Int(self.centre[1])),
        ]);
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
        out.to_vec()
    }

    /// The map kept under `key`; `None` for another key.
    pub fn decode(key: &[u8], record: &[u8]) -> Result<Option<Self>, Error> {
        if !key.strip_prefix(MAP_PREFIX).is_some_and(|id| std::str::from_utf8(id).is_ok_and(|id| id.parse::<i64>().is_ok())) {
            return Ok(None);
        }
        let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
        let long = |key: &str| match root.get(key) {
            Some(Value::Long(value)) => Ok(*value),
            _ => Err(Error::Map("an id that is no long")),
        };
        let int = |key: &str| match root.get(key) {
            Some(Value::Int(value)) => Ok(*value),
            _ => Err(Error::Map("a centre that is no int")),
        };
        let byte = |key: &str| match root.get(key) {
            Some(Value::Byte(value)) => *value,
            _ => 0,
        };
        let colours = match root.get("colors") {
            Some(Value::ByteArray(bytes)) => bytes.clone(),
            _ => return Err(Error::Map("no colours")),
        };
        let centre = [int("xCenter")?, int("zCenter")?];
        Ok(Some(Self { id: long("mapId")?, parent: long("parentMapId")?, dimension: byte("dimension"), scale: byte("scale"), locked: byte("mapLocked") != 0, centre, colours, pin: MapPin::decode(root.get("decorations")) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_reads_back_as_it_was_written() {
        let map = MapRecord { id: -4294967279, parent: -1, dimension: 1, scale: 2, locked: true, centre: [320, -192], colours: (0..=255).cycle().take(128 * 128 * 4).collect(), pin: None };
        let pinned = MapRecord { pin: Some(MapPin { kind: 21, rotation: 8, x: -76, y: -28, block: [-984, 64, 136] }), ..map.clone() };
        assert_eq!(MapRecord::decode(&pinned.key(), &pinned.encode()).unwrap(), Some(pinned.clone()));
        assert_eq!(map.key(), b"map_-4294967279");
        assert_eq!(MapRecord::decode(&map.key(), &map.encode()).unwrap(), Some(map.clone()));
        assert_eq!(MapRecord::decode(b"mapsomething", &map.encode()).unwrap(), None);
        assert!(MapRecord::decode(&map.key(), &[10, 0, 0, 0]).is_err());
    }
}
