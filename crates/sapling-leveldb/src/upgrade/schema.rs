//! One block-state upgrade schema (pmmp/BedrockBlockUpgradeSchema `nbt_upgrade_schema/*.json`,
//! format in its `nbt_upgrade_schema_schema.json`) and how it moves a state one version on.
//! Ported from df-mc/worldupgrader's `blockupgrader` (MIT); the rules' order is its README's.

use std::collections::{BTreeMap, HashMap};

use acacia_proto::nbt::Value;
use serde_json::Value as Json;

/// A state's properties by name.
pub type States = BTreeMap<String, Value>;

/// How a property's value becomes part of the block's name.
struct Flatten {
    prefix: String,
    property: String,
    suffix: String,
    remaps: HashMap<String, String>,
}

/// A whole state mapped anew: `old` is a filter, the first remap whose filter fits wins.
struct Remap {
    old: Vec<(String, Value)>,
    name: Result<String, Flatten>,
    new: States,
    copied: Vec<String>,
}

pub struct Schema {
    /// `major << 24 | minor << 16 | patch << 8 | revision` of the version it upgrades to.
    pub id: i32,
    renamed_ids: HashMap<String, String>,
    added: HashMap<String, States>,
    removed: HashMap<String, Vec<String>>,
    renamed: HashMap<String, Vec<(String, String)>>,
    remapped_values: HashMap<String, HashMap<String, Vec<(Value, Value)>>>,
    flattened: HashMap<String, Flatten>,
    remapped: HashMap<String, Vec<Remap>>,
}

fn tag(json: &Json) -> Option<Value> {
    let (kind, value) = json.as_object()?.iter().next()?;
    match kind.as_str() {
        "byte" => Some(Value::Byte(value.as_u64()? as u8 as i8)),
        "int" => Some(Value::Int(i32::try_from(value.as_i64()?).ok()?)),
        "string" => Some(Value::String(value.as_str()?.into())),
        _ => None,
    }
}

fn tags(json: Option<&Json>) -> Option<Vec<(String, Value)>> {
    json.and_then(Json::as_object).map_or(Some(Vec::new()), |fields| fields.iter().map(|(key, value)| Some((key.clone(), tag(value)?))).collect())
}

fn strings(json: &Json) -> HashMap<String, String> {
    json.as_object().into_iter().flatten().filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned()))).collect()
}

fn flatten(json: &Json) -> Option<Flatten> {
    let text = |key: &str| json.get(key).and_then(Json::as_str).map(str::to_owned);
    let remaps = json.get("flattenedValueRemaps").map(strings).unwrap_or_default();
    Some(Flatten { prefix: text("prefix")?, property: text("flattenedProperty")?, suffix: text("suffix")?, remaps })
}

/// Each block's entry of the schema's map `key`, read by `read`.
fn by_block<T>(json: &Json, key: &str, read: impl Fn(&Json) -> Option<T>) -> Option<HashMap<String, T>> {
    json.get(key).and_then(Json::as_object).into_iter().flatten().map(|(block, value)| Some((block.clone(), read(value)?))).collect()
}

impl Schema {
    pub fn parse(text: &str) -> Option<Self> {
        let json: Json = serde_json::from_str(text).ok()?;
        let part = |key: &str| json.get(key).and_then(Json::as_i64).map(|part| part as i32);
        let id = part("maxVersionMajor")? << 24 | part("maxVersionMinor")? << 16 | part("maxVersionPatch")? << 8 | part("maxVersionRevision")?;
        let index: HashMap<String, Vec<(Value, Value)>> = by_block(&json, "remappedPropertyValuesIndex", |pairs| {
            pairs.as_array()?.iter().map(|pair| Some((tag(pair.get("old")?)?, tag(pair.get("new")?)?))).collect()
        })?;
        let remap = |value: &Json| -> Option<Remap> {
            let name = match value.get("newName").and_then(Json::as_str) {
                Some(name) => Ok(name.to_owned()),
                None => Err(flatten(value.get("newFlattenedName")?)?),
            };
            let copied = value.get("copiedState").and_then(Json::as_array).into_iter().flatten().filter_map(|key| key.as_str().map(str::to_owned)).collect();
            Some(Remap { old: tags(value.get("oldState"))?, name, new: tags(value.get("newState"))?.into_iter().collect(), copied })
        };
        Some(Self {
            id,
            renamed_ids: json.get("renamedIds").map(strings).unwrap_or_default(),
            added: by_block(&json, "addedProperties", |fields| Some(tags(Some(fields))?.into_iter().collect()))?,
            removed: by_block(&json, "removedProperties", |keys| keys.as_array()?.iter().map(|key| key.as_str().map(str::to_owned)).collect())?,
            renamed: by_block(&json, "renamedProperties", |names| Some(strings(names).into_iter().collect()))?,
            remapped_values: by_block(&json, "remappedPropertyValues", |properties| strings(properties).into_iter().map(|(key, name)| Some((key, index.get(&name)?.clone()))).collect())?,
            flattened: by_block(&json, "flattenedProperties", flatten)?,
            remapped: by_block(&json, "remappedStates", |remaps| remaps.as_array()?.iter().map(remap).collect())?,
        })
    }

    /// The state `(name, states)` as this schema's version has it, if it changes it.
    pub fn apply(&self, name: &str, states: &States) -> Option<(String, States)> {
        if let Some(remaps) = self.remapped.get(name) {
            let fits = |remap: &&Remap| remap.old.iter().all(|(key, value)| states.get(key) == Some(value));
            if let Some(remap) = remaps.iter().find(fits) {
                let mut new = remap.new.clone();
                new.extend(remap.copied.iter().filter_map(|key| Some((key.clone(), states.get(key)?.clone()))));
                let renamed = match &remap.name {
                    Ok(renamed) => renamed.clone(),
                    Err(rule) => flattened(rule, name, &mut states.clone()),
                };
                return Some((renamed, new));
            }
        }
        let mut new = states.clone();
        let renamed = match (self.renamed_ids.get(name), self.flattened.get(name)) {
            (Some(renamed), _) => renamed.clone(),
            (None, Some(rule)) => flattened(rule, name, &mut new),
            (None, None) => name.to_owned(),
        };
        let remap_value = |key: &str, value: &Value| {
            let pairs = self.remapped_values.get(name).and_then(|properties| properties.get(key));
            pairs.and_then(|pairs| pairs.iter().find(|(old, _)| old == value)).map_or_else(|| value.clone(), |(_, new)| new.clone())
        };
        for (key, value) in self.added.get(name).into_iter().flatten() {
            new.entry(key.clone()).or_insert_with(|| value.clone());
        }
        for key in self.removed.get(name).into_iter().flatten() {
            new.remove(key);
        }
        for (old, renamed) in self.renamed.get(name).into_iter().flatten() {
            if let Some(value) = new.remove(old) {
                new.insert(renamed.clone(), remap_value(old, &value));
            }
        }
        for key in self.remapped_values.get(name).into_iter().flat_map(HashMap::keys) {
            if let Some(value) = new.get(key).cloned() {
                new.insert(key.clone(), remap_value(key, &value));
            }
        }
        (renamed != name || new != *states).then_some((renamed, new))
    }
}

/// The name `rule` makes of `name` from its property, which it takes out of `states`; `name`
/// itself when the property is missing.
fn flattened(rule: &Flatten, name: &str, states: &mut States) -> String {
    let embedded = match states.get(&rule.property) {
        Some(Value::String(text)) => text.to_string(),
        Some(Value::Int(number)) => number.to_string(),
        Some(Value::Byte(byte)) => (*byte as u8).to_string(),
        _ => return name.to_owned(),
    };
    states.remove(&rule.property);
    let value = rule.remaps.get(&embedded).cloned().unwrap_or(embedded);
    format!("{}{value}{}", rule.prefix, rule.suffix)
}
