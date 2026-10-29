//! Custom blocks plugins register (`sapling_api::CustomBlock`): their state compounds, the
//! StartGame `block_properties` entry clients build them from (laid out as dragonfly's
//! `internal/blockinternal` lays it out, with BDS's `molangVersion` and `menu_category` from
//! StartGame.bin), and what the shard asks of them by name.

use std::sync::{Arc, RwLock};

use acacia_proto::nbt::{List, Nbt, Str, Value};
use acacia_proto::types::BlockPropertiesItem;
use sapling_api::{BlockBox, CreativeCategory, CustomBlock, Geometry, Material, Permutation, StateValue, StateValues};

/// BDS's own data-driven blocks' `molangVersion` (StartGame.bin).
const MOLANG_VERSION: i32 = 13;

struct Registered {
    block: Arc<CustomBlock>,
    entry: BlockPropertiesItem,
}

static BLOCKS: RwLock<Vec<Registered>> = RwLock::new(Vec::new());

/// The custom block `name`, as registered.
pub fn custom_block(name: &str) -> Option<Arc<CustomBlock>> {
    if name.starts_with("minecraft:") {
        return None;
    }
    BLOCKS.read().expect("custom blocks lock").iter().find(|known| known.block.identifier == name).map(|known| known.block.clone())
}

/// StartGame's entries for the custom blocks, after BDS's own.
pub(crate) fn block_properties() -> Vec<BlockPropertiesItem> {
    BLOCKS.read().expect("custom blocks lock").iter().map(|known| known.entry.clone()).collect()
}

/// Registers `block`, numbered `block_id` (`vanilla_block_data.block_id`); a name registered
/// before is kept as it was.
pub fn register_block(block: &CustomBlock, block_id: i32) {
    let mut blocks = BLOCKS.write().expect("custom blocks lock");
    if blocks.iter().any(|known| known.block.identifier == block.identifier) {
        return;
    }
    let entry = BlockPropertiesItem { name: block.identifier.clone(), state: Nbt { name: String::new(), value: properties(block, block_id) }.into() };
    blocks.push(Registered { block: Arc::new(block.clone()), entry });
}

/// Each state's compound as a world saves it and a client hashes it: booleans as bytes, numbers
/// as ints, in the order of [`CustomBlock::permutations_of_states`].
pub fn block_states(block: &CustomBlock) -> Vec<Value> {
    let typed = |value: &StateValue| match value {
        StateValue::Bool(on) => Value::Byte(i8::from(*on)),
        StateValue::Int(number) => Value::Int(*number),
        StateValue::Text(text) => Value::String(Str::from(text.as_str())),
    };
    let compound = |values: Vec<(String, StateValue)>| Value::Compound(values.iter().map(|(name, value)| (Str::from(name.as_str()), typed(value))).collect());
    block.permutations_of_states().into_iter().map(compound).collect()
}

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (Str::from(key), value)).collect())
}

fn text(value: &str) -> Value {
    Value::String(Str::from(value))
}

fn floats(values: [f32; 3]) -> Value {
    Value::List(List { tag: 5, items: values.into_iter().map(Value::Float).collect() })
}

fn category_name(category: CreativeCategory) -> &'static str {
    match category {
        CreativeCategory::Construction => "construction",
        CreativeCategory::Nature => "nature",
        CreativeCategory::Equipment => "equipment",
        CreativeCategory::Items => "items",
        CreativeCategory::None => "none",
    }
}

fn collision(found: Option<BlockBox>) -> Value {
    let Some(BlockBox { min, max }) = found else { return compound(vec![("enabled", Value::Byte(0)), ("boxes", Value::List(List { tag: 10, items: Vec::new() }))]) };
    let corners = compound(vec![
        ("minX", Value::Float(min[0])),
        ("minY", Value::Float(min[1])),
        ("minZ", Value::Float(min[2])),
        ("maxX", Value::Float(max[0])),
        ("maxY", Value::Float(max[1])),
        ("maxZ", Value::Float(max[2])),
    ]);
    compound(vec![("enabled", Value::Byte(1)), ("boxes", Value::List(List { tag: 10, items: vec![corners] }))])
}

/// A selection box as an origin from the block's bottom centre and a size.
fn selection(found: Option<BlockBox>) -> Value {
    let Some(BlockBox { min, max }) = found else { return compound(vec![("enabled", Value::Byte(0))]) };
    let origin = [min[0] - 8.0, min[1], min[2] - 8.0];
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    compound(vec![("enabled", Value::Byte(1)), ("origin", floats(origin)), ("size", floats(size))])
}

fn geometry(geometry: &Geometry) -> Value {
    let identifier = match geometry {
        Geometry::Cube => "minecraft:geometry.full_block",
        Geometry::Model { identifier, .. } => identifier,
    };
    compound(vec![("identifier", text(identifier))])
}

fn materials(materials: &[(String, Material)]) -> Value {
    let material = |material: &Material| {
        compound(vec![
            ("texture", text(&material.texture)),
            ("render_method", text(material.render_method.name())),
            ("face_dimming", Value::Byte(i8::from(material.face_dimming))),
            ("ambient_occlusion", Value::Float(if material.ambient_occlusion { 1.0 } else { 0.0 })),
        ])
    };
    let faces = Value::Compound(materials.iter().map(|(face, found)| (Str::from(face.as_str()), material(found))).collect());
    compound(vec![("mappings", compound(Vec::new())), ("materials", faces)])
}

fn rotation(turns: [i32; 3]) -> Value {
    let [x, y, z] = turns.map(|turn| turn * 90);
    let mut fields = vec![("RX", Value::Int(x)), ("RY", Value::Int(y)), ("RZ", Value::Int(z))];
    fields.extend([("SX", 1.0), ("SY", 1.0), ("SZ", 1.0), ("TX", 0.0), ("TY", 0.0), ("TZ", 0.0)].map(|(key, value)| (key, Value::Float(value))));
    compound(fields)
}

fn permutation(found: &Permutation) -> Value {
    let mut fields = Vec::new();
    if let Some(shape) = &found.geometry {
        fields.push(("minecraft:geometry", geometry(shape)));
    }
    if let Some(drawn) = &found.materials {
        fields.push(("minecraft:material_instances", materials(drawn)));
    }
    if let Some(stops) = found.collision {
        fields.push(("minecraft:collision_box", collision(stops)));
    }
    if let Some(aimed) = found.selection {
        fields.push(("minecraft:selection_box", selection(aimed)));
    }
    if let Some(level) = found.light_emission {
        fields.push(("minecraft:light_emission", compound(vec![("emission", Value::Byte(level.min(15) as i8))])));
    }
    if let Some(turns) = found.rotation {
        fields.push(("minecraft:transformation", rotation(turns)));
    }
    compound(vec![("components", compound(fields)), ("condition", text(&found.condition))])
}

fn state_property((name, values): &(String, StateValues)) -> Value {
    let list = match values {
        StateValues::Bool => List { tag: 1, items: vec![Value::Byte(0), Value::Byte(1)] },
        StateValues::Int(values) => List { tag: 3, items: values.iter().copied().map(Value::Int).collect() },
        StateValues::Text(values) => List { tag: 8, items: values.iter().map(|value| text(value)).collect() },
    };
    compound(vec![("enum", Value::List(list)), ("name", text(name))])
}

/// The block's `block_properties` compound.
fn properties(block: &CustomBlock, block_id: i32) -> Value {
    let mut components = vec![
        ("minecraft:collision_box", collision(block.collision)),
        ("minecraft:selection_box", selection(block.selection)),
        ("minecraft:geometry", geometry(&block.geometry)),
        ("minecraft:material_instances", materials(&block.materials)),
        ("minecraft:light_emission", compound(vec![("emission", Value::Byte(block.light_emission.min(15) as i8))])),
        ("minecraft:light_dampening", compound(vec![("lightLevel", Value::Byte(block.light_dampening.min(15) as i8))])),
        ("minecraft:friction", compound(vec![("value", Value::Float(block.friction))])),
        ("minecraft:destructible_by_mining", compound(vec![("value", Value::Float(block.break_seconds.unwrap_or(-1.0)))])),
    ];
    if let Some(colour) = &block.map_colour {
        components.push(("minecraft:map_color", compound(vec![("value", text(colour))])));
    }
    if !block.permutations.is_empty() {
        components.push(("minecraft:on_player_placing", compound(vec![("triggerType", text("placement_trigger"))])));
    }
    components.sort_by(|a, b| a.0.cmp(b.0));
    let menu = compound(vec![("category", text(category_name(block.category))), ("group", text(&block.group)), ("is_hidden_in_commands", Value::Byte(0))]);
    let mut fields = vec![
        ("components", compound(components)),
        ("menu_category", menu),
        ("molangVersion", Value::Int(MOLANG_VERSION)),
        ("vanilla_block_data", compound(vec![("block_id", Value::Int(block_id))])),
    ];
    if !block.states.is_empty() {
        fields.push(("properties", Value::List(List { tag: 10, items: block.states.iter().map(state_property).collect() })));
    }
    if !block.permutations.is_empty() {
        fields.push(("permutations", Value::List(List { tag: 10, items: block.permutations.iter().map(permutation).collect() })));
    }
    compound(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_custom_block_is_listed_with_its_states_and_permutations() {
        let lamp = CustomBlock::new("test_blocks:lamp", "Lamp", Vec::new())
            .state("test_blocks:lit", StateValues::Bool)
            .state("test_blocks:tint", StateValues::Int(vec![0, 1, 2]))
            .permutation(Permutation { light_emission: Some(15), ..Permutation::when("q.block_state('test_blocks:lit') == true") });
        let states = block_states(&lamp);
        assert_eq!(states.len(), 6);
        assert_eq!(states[3], compound(vec![("test_blocks:lit", Value::Byte(1)), ("test_blocks:tint", Value::Int(0))]), "the first state changes slowest");
        register_block(&lamp, 10_000);
        let entry = block_properties().into_iter().find(|entry| entry.name == "test_blocks:lamp").unwrap();
        let entry = entry.state.decode().unwrap().value;
        assert_eq!(entry.get("vanilla_block_data").and_then(|data| data.get("block_id")), Some(&Value::Int(10_000)));
        let Some(Value::List(permutations)) = entry.get("permutations") else { panic!("permutations") };
        assert_eq!(permutations.items[0].get("condition"), Some(&text("q.block_state('test_blocks:lit') == true")));
        assert!(entry.get("components").and_then(|components| components.get("minecraft:on_player_placing")).is_some());
        assert!(custom_block("test_blocks:lamp").is_some() && custom_block("minecraft:stone").is_none());
    }
}
