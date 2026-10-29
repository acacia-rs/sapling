//! The resource pack a server's custom content needs, made from the content itself as dragonfly's
//! `internal/packbuilder` makes its own: item and block textures with their atlases, block models,
//! `blocks.json` and the names. Its uuid is the same for every server; its version is the
//! content's hash, so clients download it again only when the content changes.

use std::collections::BTreeMap;

use sapling_api::{Content, CustomBlock, CustomItem, Geometry};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::archive::{self, Files};

const HEADER_UUID: &str = "5a9f1c3e-7b2d-4e8a-9c61-0d4b8e2f7a15";
const MODULE_UUID: &str = "5a9f1c3e-7b2d-4e8a-9c61-0d4b8e2f7a16";
/// The oldest client the pack's layout is for.
const MIN_ENGINE: [u32; 3] = [1, 21, 0];

/// The generated pack: its uuid, version and zip; `None` for content without items or blocks.
pub(crate) struct Generated {
    pub uuid: &'static str,
    pub version: String,
    pub zip: Vec<u8>,
    pub unpacked: u64,
}

/// Where a texture of `identifier` lies in the pack, without `.png`.
fn texture_path(folder: &str, name: &str) -> String {
    format!("textures/{folder}/{}", name.replace(':', "_"))
}

fn json_file(path: &str, value: &Value) -> (String, Vec<u8>) {
    (path.to_owned(), serde_json::to_vec_pretty(value).expect("json in memory"))
}

fn item_files(items: &[CustomItem], files: &mut Files, lang: &mut Vec<String>) {
    let mut atlas = serde_json::Map::new();
    for item in items {
        let path = texture_path("items", &item.identifier);
        files.push((format!("{path}.png"), item.texture.clone()));
        atlas.insert(item.identifier.clone(), json!({ "textures": path }));
        lang.push(format!("item.{}.name={}", item.identifier, item.display_name));
    }
    if !items.is_empty() {
        files.push(json_file("textures/item_texture.json", &json!({ "resource_pack_name": "vanilla", "texture_name": "atlas.items", "texture_data": atlas })));
    }
}

/// The models of a block and its permutations, by file name.
fn models(block: &CustomBlock) -> impl Iterator<Item = (&str, &[u8])> {
    let geometries = std::iter::once(&block.geometry).chain(block.permutations.iter().filter_map(|permutation| permutation.geometry.as_ref()));
    geometries.filter_map(|geometry| match geometry {
        Geometry::Model { identifier, json } => Some((identifier.as_str(), json.as_slice())),
        Geometry::Cube => None,
    })
}

fn block_files(blocks: &[CustomBlock], files: &mut Files, lang: &mut Vec<String>) {
    let (mut atlas, mut sounds) = (serde_json::Map::new(), serde_json::Map::new());
    sounds.insert("format_version".into(), json!([1, 1, 0]));
    let mut written = BTreeMap::new();
    for block in blocks {
        for (name, png) in &block.textures {
            let path = texture_path("blocks", name);
            files.push((format!("{path}.png"), png.clone()));
            atlas.insert(name.clone(), json!({ "textures": path }));
        }
        for (identifier, json) in models(block) {
            let file = format!("models/blocks/{}.geo.json", identifier.trim_start_matches("geometry.").replace(':', "_"));
            written.entry(file).or_insert_with(|| json.to_vec());
        }
        sounds.insert(block.identifier.clone(), json!({ "sound": "stone" }));
        lang.push(format!("tile.{}.name={}", block.identifier, block.display_name));
    }
    files.extend(written);
    if !blocks.is_empty() {
        let terrain = json!({ "resource_pack_name": "vanilla", "texture_name": "atlas.terrain", "padding": 8, "num_mip_levels": 4, "texture_data": atlas });
        files.push(json_file("textures/terrain_texture.json", &terrain));
        files.push(json_file("blocks.json", &Value::Object(sounds)));
    }
}

fn manifest(version: [u32; 3]) -> Value {
    let description = "Textures and names of this server's custom items and blocks, made by sapling";
    json!({
        "format_version": 2,
        "header": { "name": "sapling custom content", "description": description, "uuid": HEADER_UUID, "version": version, "min_engine_version": MIN_ENGINE },
        "modules": [{ "type": "resources", "description": description, "uuid": MODULE_UUID, "version": version }],
    })
}

/// The pack for `content`.
pub(crate) fn build(content: &Content) -> Option<Generated> {
    if content.is_empty() {
        return None;
    }
    let (mut files, mut lang) = (Files::new(), Vec::new());
    item_files(&content.items, &mut files, &mut lang);
    block_files(&content.blocks, &mut files, &mut lang);
    lang.push(String::new());
    files.push(("texts/en_US.lang".into(), lang.join("\n").into_bytes()));
    files.push(json_file("texts/languages.json", &json!(["en_US"])));
    files.sort();
    let mut hash = Sha256::new();
    for (name, data) in &files {
        hash.update(name.as_bytes());
        hash.update((data.len() as u64).to_le_bytes());
        hash.update(data);
    }
    let hash = hash.finalize();
    let version = [1, u32::from(u16::from_le_bytes([hash[0], hash[1]])), u32::from(u16::from_le_bytes([hash[2], hash[3]]))];
    files.insert(0, json_file("manifest.json", &manifest(version)));
    let unpacked = files.iter().map(|(_, data)| data.len() as u64).sum();
    Some(Generated { uuid: HEADER_UUID, version: version.map(|part| part.to_string()).join("."), zip: archive::write(&files), unpacked })
}

#[cfg(test)]
mod tests {
    use sapling_api::CustomBlock;

    use super::*;
    use crate::packs::manifest::Manifest;

    fn content(texture: &[u8]) -> Content {
        let mut content = Content::default();
        content.item(CustomItem::new("test:ruby", "Ruby", texture.to_vec()));
        let model = Geometry::Model { identifier: "geometry.test.lamp".into(), json: br#"{"format_version":"1.12.0"}"#.to_vec() };
        content.block(CustomBlock::new("test:lamp", "Lamp", b"lamp png".to_vec()).geometry(model));
        content
    }

    #[test]
    fn the_pack_holds_textures_atlases_models_and_names_and_its_version_follows_the_content() {
        let pack = build(&content(b"ruby png")).unwrap();
        let files = archive::read(&pack.zip).unwrap();
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        for expected in ["manifest.json", "textures/items/test_ruby.png", "textures/item_texture.json", "textures/blocks/test_lamp.png", "textures/terrain_texture.json", "models/blocks/test.lamp.geo.json", "blocks.json", "texts/en_US.lang"] {
            assert!(names.contains(&expected), "{expected} in {names:?}");
        }
        let manifest = Manifest::parse(&files[0].1).unwrap();
        assert_eq!((manifest.uuid.as_str(), manifest.version.as_str()), (HEADER_UUID, pack.version.as_str()));
        assert!(manifest.has_module("resources"));
        let lang = String::from_utf8(files.iter().find(|(name, _)| name == "texts/en_US.lang").unwrap().1.clone()).unwrap();
        assert!(lang.contains("item.test:ruby.name=Ruby") && lang.contains("tile.test:lamp.name=Lamp"), "{lang}");
        let atlas: Value = serde_json::from_slice(&files.iter().find(|(name, _)| name == "textures/item_texture.json").unwrap().1).unwrap();
        assert_eq!(atlas["texture_data"]["test:ruby"]["textures"], "textures/items/test_ruby");

        assert_eq!(build(&content(b"ruby png")).unwrap().version, pack.version, "the same content, the same version");
        assert_ne!(build(&content(b"another ruby")).unwrap().version, pack.version, "a changed texture, another version");
        assert!(build(&Content::default()).is_none());
    }
}
