//! Structure templates, the `.mcstructure` format: one little-endian NBT compound holding
//! `format_version` 1, `size` and `structure_world_origin` as lists of three ints, and `structure`:
//! `block_indices` (two layers, the second the liquid one, of palette indices, -1 for none), the
//! `entities` as saved actors and `palette.default` with the `block_palette` and, under each cell
//! index as text, the `block_position_data` holding a cell's `block_entity_data`. Cells run with z
//! fastest, then y, then x. BDS keeps the same compound in its world under `structuretemplate_<name>`.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Str, Value};
use bytes::BytesMut;

use crate::Error;

/// The LevelDB key prefix of a structure saved in a world.
pub const STRUCTURE_PREFIX: &[u8] = b"structuretemplate_";
const FORMAT_VERSION: i32 = 1;
const INT: u8 = 3;
const LIST: u8 = 9;
const COMPOUND: u8 = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct StructureTemplate {
    /// Cells along x, y and z.
    pub size: [i32; 3],
    /// Where its lowest corner was in the world it was saved from.
    pub origin: [i32; 3],
    /// `{name, states, version}` compounds, as chunk palettes keep them.
    pub palette: Vec<Value>,
    /// Layer 0 then layer 1, one palette index per cell (-1: the cell is left as it is).
    pub layers: [Vec<i32>; 2],
    /// The block entities by cell index.
    pub block_entities: Vec<(usize, Value)>,
    /// Saved actors, each with its world position in `Pos`.
    pub entities: Vec<Value>,
}

impl StructureTemplate {
    pub fn volume(&self) -> usize {
        self.size.iter().map(|&side| side.max(0) as usize).product()
    }

    /// The index of the cell at `offset` from the lowest corner.
    pub fn index(&self, [x, y, z]: [i32; 3]) -> usize {
        ((x * self.size[1] + y) * self.size[2] + z) as usize
    }

    /// The offset from the lowest corner of the cell at `index`.
    pub fn offset(&self, index: usize) -> [i32; 3] {
        let index = index as i32;
        let plane = self.size[1] * self.size[2];
        [index / plane, index % plane / self.size[2], index % self.size[2]]
    }

    pub fn encode(&self) -> Vec<u8> {
        let ints = |values: &[i32]| Value::List(List { tag: INT, items: values.iter().copied().map(Value::Int).collect() });
        let compounds = |items: Vec<Value>| Value::List(List { tag: COMPOUND, items });
        let positions = self.block_entities.iter().map(|(index, entity)| (Str::from(index.to_string().as_str()), Value::Compound(vec![("block_entity_data".into(), entity.clone())])));
        let default = Value::Compound(vec![("block_palette".into(), compounds(self.palette.clone())), ("block_position_data".into(), Value::Compound(positions.collect()))]);
        let structure = Value::Compound(vec![
            ("block_indices".into(), Value::List(List { tag: LIST, items: self.layers.iter().map(|layer| ints(layer)).collect() })),
            ("entities".into(), compounds(self.entities.clone())),
            ("palette".into(), Value::Compound(vec![("default".into(), default)])),
        ]);
        let root = Value::Compound(vec![
            ("format_version".into(), Value::Int(FORMAT_VERSION)),
            ("size".into(), ints(&self.size)),
            ("structure".into(), structure),
            ("structure_world_origin".into(), ints(&self.origin)),
        ]);
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
        out.to_vec()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let bad = |what| Error::Structure(what);
        let root = nbt::read::<LittleEndian>(&mut &bytes[..])?.value;
        let size = three(root.get("size")).ok_or(bad("no size"))?;
        if size.iter().any(|&side| side < 0) {
            return Err(bad("a negative size"));
        }
        let origin = three(root.get("structure_world_origin")).unwrap_or([0; 3]);
        let structure = root.get("structure").ok_or(bad("no structure"))?;
        let volume: usize = size.iter().map(|&side| side as usize).product();
        let mut layers = [vec![-1; volume], vec![-1; volume]];
        if let Some(Value::List(indices)) = structure.get("block_indices") {
            for (layer, read) in layers.iter_mut().zip(&indices.items) {
                let read = ints(read).ok_or(bad("block indices that are no ints"))?;
                if read.len() != volume {
                    return Err(bad("block indices of another size"));
                }
                *layer = read;
            }
        }
        let default = structure.get("palette").and_then(|palette| palette.get("default"));
        let palette = match default.and_then(|default| default.get("block_palette")) {
            Some(Value::List(list)) => list.items.clone(),
            _ => Vec::new(),
        };
        if layers.iter().flatten().any(|&index| index >= palette.len() as i32 || index < -1) {
            return Err(bad("a block index outside the palette"));
        }
        let mut block_entities = Vec::new();
        if let Some(Value::Compound(cells)) = default.and_then(|default| default.get("block_position_data")) {
            for (index, data) in cells {
                let (Ok(index), Some(entity)) = (index.parse::<usize>(), data.get("block_entity_data")) else { continue };
                if index < volume {
                    block_entities.push((index, entity.clone()));
                }
            }
        }
        let entities = match structure.get("entities") {
            Some(Value::List(list)) => list.items.clone(),
            _ => Vec::new(),
        };
        Ok(Self { size, origin, palette, layers, block_entities, entities })
    }
}

fn ints(value: &Value) -> Option<Vec<i32>> {
    match value {
        Value::List(list) => list.items.iter().map(|item| if let Value::Int(n) = item { Some(*n) } else { None }).collect(),
        Value::IntArray(values) => Some(values.clone()),
        _ => None,
    }
}

fn three(value: Option<&Value>) -> Option<[i32; 3]> {
    ints(value?)?.try_into().ok()
}

/// A structure's name as BDS keys it: `mystructure:` before a name without a namespace.
pub fn structure_name(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("mystructure:{name}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::states::tests::entry;

    fn sample() -> StructureTemplate {
        let palette = [entry("minecraft:stone", &[]), entry("minecraft:water", &[("liquid_depth", Value::Int(0))])];
        let palette = palette.iter().map(|bytes| nbt::read::<LittleEndian>(&mut &bytes[..]).unwrap().value).collect();
        let chest = Value::Compound(vec![("id".into(), Value::String("Chest".into())), ("x".into(), Value::Int(4))]);
        let pig = Value::Compound(vec![("identifier".into(), Value::String("minecraft:pig".into()))]);
        let mut layers = [vec![-1; 12], vec![-1; 12]];
        (layers[0][0], layers[0][11], layers[1][11]) = (0, 0, 1);
        StructureTemplate { size: [2, 3, 2], origin: [4, 99, -6], palette, layers, block_entities: vec![(11, chest)], entities: vec![pig] }
    }

    #[test]
    fn a_template_reads_back_as_it_was_written() {
        let template = sample();
        assert_eq!(StructureTemplate::decode(&template.encode()).unwrap(), template);
        assert_eq!((template.volume(), template.index([1, 2, 1]), template.offset(11)), (12, 11, [1, 2, 1]));
        assert_eq!((template.index([0, 1, 1]), template.offset(3)), (3, [0, 1, 1]), "z runs fastest, then y");
    }

    #[test]
    fn damaged_templates_are_errors() {
        let mut template = sample();
        template.layers[0][3] = 7;
        assert!(StructureTemplate::decode(&template.encode()).is_err(), "an index past the palette");
        template.layers[0].pop();
        assert!(StructureTemplate::decode(&template.encode()).is_err(), "a layer of another size");
        assert!(StructureTemplate::decode(&[10, 0]).is_err());
        assert_eq!((structure_name("s1"), structure_name("mine:s1")), ("mystructure:s1".to_owned(), "mine:s1".to_owned()));
    }
}
