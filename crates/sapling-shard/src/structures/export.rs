//! A template as BDS sent it in StructureTemplateDataExportResponse (research/captures/
//! bds-structblocks.txt): `format_version` 2, and its block indices as int arrays, the second
//! layer only where it holds anything.
// TODO: a template with liquids was not exported on BDS; the second layer's form is a guess.

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Network, Raw, Value};
use bytes::BytesMut;
use sapling_world::StructureTemplate;

const INT_ARRAY: u8 = 11;

pub(crate) fn network_nbt(template: &StructureTemplate) -> Option<Raw<Network>> {
    let mut root = nbt::read::<LittleEndian>(&mut &template.encode()[..]).ok()?.value;
    let Value::Compound(fields) = &mut root else { return None };
    for (key, value) in fields.iter_mut() {
        match &**key {
            "format_version" => *value = Value::Int(2),
            "structure" => {
                let indices = template.layers.iter().enumerate().filter(|(layer, cells)| *layer == 0 || cells.iter().any(|&cell| cell >= 0));
                let indices = Value::List(List { tag: INT_ARRAY, items: indices.map(|(_, cells)| Value::IntArray(cells.clone())).collect() });
                let Value::Compound(structure) = value else { continue };
                for (key, kept) in structure.iter_mut() {
                    match (&**key, kept) {
                        ("block_indices", kept) => *kept = indices.clone(),
                        // BDS sent no entities as an untyped empty list (research/captures/bds-structpower.txt).
                        ("entities", Value::List(list)) if list.items.is_empty() => list.tag = 0,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = BytesMut::new();
    nbt::write::<Network>(&mut out, &Nbt { name: String::new(), value: root });
    Raw::<Network>::read(&mut &out[..]).ok()
}
