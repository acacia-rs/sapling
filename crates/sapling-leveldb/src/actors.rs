//! A column's actors (mobs, items, ...). Since 1.18.30 `digp<chunk key>` lists their unique ids,
//! 8 bytes each, and `actorprefix<id>` holds one compound per actor; older worlds keep the
//! compounds under the chunk's `Entity` tag. Either way this crate hands them out as one record
//! (records.rs). Sources: research/worlds-and-ecs.md "Actors".

use std::collections::HashSet;

use acacia_proto::nbt::{self, LittleEndian, List, Value};

use crate::keys::{ChunkKey, Tag};
use crate::records::compounds;
use crate::{Batch, Db, Error, ItemStack};

const ACTOR: &[u8] = b"actorprefix";
/// NBT tag numbers of a list's items.
const FLOAT_TAG: u8 = 5;
const STRING_TAG: u8 = 8;

/// What every stored actor says about itself; the rest of its compound is its kind's business.
#[derive(Debug, Clone, PartialEq)]
pub struct Actor {
    pub unique_id: i64,
    /// Namespaced, e.g. `minecraft:cow`.
    pub identifier: String,
    /// Feet position.
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// The kind's look, e.g. a horse's coat (`Variant`) and its markings (`MarkVariant`).
    pub variant: i32,
    pub mark_variant: i32,
    /// E.g. a sheep's wool.
    pub color: i8,
    pub baby: bool,
    /// The name tag (`CustomName`); empty for none.
    pub name: String,
    /// What a `minecraft:item` lies there as.
    pub item: Option<ItemStack>,
    /// What a `minecraft:painting` shows.
    pub painting: Option<Painting>,
    /// What the mob wears or has become, of [`STATES`].
    pub states: Vec<State>,
    /// What it wears (`Armor`): helmet, chestplate, leggings, boots, in that order.
    pub armor: Vec<Option<ItemStack>>,
    pub mainhand: Option<ItemStack>,
    pub offhand: Option<ItemStack>,
    /// The `minecraft:health` it has left, when it stores any.
    pub health: Option<f32>,
}

/// A stored byte that shows on the mob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Saddled,
    Tamed,
    Sheared,
    Chested,
    Sitting,
}

const STATES: [(&str, State); 5] =
    [("Saddled", State::Saddled), ("IsTamed", State::Tamed), ("Sheared", State::Sheared), ("Chested", State::Chested), ("Sitting", State::Sitting)];

#[derive(Debug, Clone, PartialEq)]
pub struct Painting {
    /// The picture's name, e.g. `Kebab` (`Motif`; `Motive` in older worlds).
    pub motif: String,
    /// The way it faces (`Direction`): 0 south, 1 west, 2 north, 3 east.
    pub direction: i8,
}

impl Actor {
    fn parse(actor: &Value) -> Option<Self> {
        let int = |key: &str| if let Some(Value::Int(int)) = actor.get(key) { *int } else { 0 };
        let byte = |key: &str| if let Some(Value::Byte(byte)) = actor.get(key) { *byte } else { 0 };
        let name = if let Some(Value::String(name)) = actor.get("CustomName") { name.clone() } else { String::new() };
        let floats = |key: &str| match actor.get(key)? {
            Value::List(list) => list.items.iter().map(|item| if let Value::Float(float) = item { Some(*float) } else { None }).collect::<Option<Vec<f32>>>(),
            _ => None,
        };
        let (Some(Value::Long(unique_id)), Some(Value::String(identifier))) = (actor.get("UniqueID"), actor.get("identifier")) else { return None };
        let (position, rotation) = (floats("Pos")?, floats("Rotation")?);
        let list = |key: &str| if let Some(Value::List(list)) = actor.get(key) { &list.items[..] } else { &[] };
        let slots = |key: &str| list(key).iter().map(ItemStack::parse).collect::<Vec<_>>();
        let health = list("Attributes").iter().find(|attribute| matches!(attribute.get("Name"), Some(Value::String(name)) if name == "minecraft:health"));
        Some(Self {
            armor: slots("Armor"),
            mainhand: slots("Mainhand").into_iter().next().flatten(),
            offhand: slots("Offhand").into_iter().next().flatten(),
            health: health.and_then(|health| if let Some(Value::Float(current)) = health.get("Current") { Some(*current) } else { None }),
            unique_id: *unique_id,
            identifier: identifier.clone(),
            position: position.get(..3)?.try_into().ok()?,
            yaw: *rotation.first()?,
            pitch: *rotation.get(1)?,
            variant: int("Variant"),
            mark_variant: int("MarkVariant"),
            color: byte("Color"),
            baby: byte("IsBaby") != 0,
            name,
            item: actor.get("Item").and_then(ItemStack::parse),
            painting: match actor.get("Motif").or(actor.get("Motive")) {
                Some(Value::String(motif)) => Some(Painting { motif: motif.clone(), direction: byte("Direction") }),
                _ => None,
            },
            states: STATES.iter().filter(|(key, _)| byte(key) != 0).map(|(_, state)| *state).collect(),
        })
    }
}

/// The actors of a record; a compound that lacks one of [`Actor`]'s fields is left out.
pub fn actors(record: &[u8]) -> Result<Vec<Actor>, Error> {
    Ok(compounds(record)?.iter().filter_map(|(_, actor)| Actor::parse(actor)).collect())
}

/// Each compound of a record as stored, with what [`Actor`] reads of it (`None` where that is not
/// enough) and its `UniqueID`. Writing the compounds one after another gives the record back.
pub fn stored_actors(record: &[u8]) -> Result<Vec<(Option<Actor>, Option<i64>, Vec<u8>)>, Error> {
    let id = |actor: &Value| if let Some(Value::Long(id)) = actor.get("UniqueID") { Some(*id) } else { None };
    Ok(compounds(record)?.into_iter().map(|(raw, actor)| (Actor::parse(&actor), id(&actor), raw.to_vec())).collect())
}

fn floats(items: &[f32]) -> Value {
    Value::List(List { tag: FLOAT_TAG, items: items.iter().map(|&item| Value::Float(item)).collect() })
}

fn written(value: Value) -> Vec<u8> {
    let mut out = bytes::BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &nbt::Nbt { name: String::new(), value });
    out.to_vec()
}

/// One stored actor's compound with its feet at `position` and moving at `motion`; the rest of
/// what the game keeps of it as it was.
pub fn moved_actor(compound: &[u8], position: [f32; 3], motion: [f32; 3]) -> Result<Vec<u8>, Error> {
    let Value::Compound(mut fields) = nbt::read::<LittleEndian>(&mut &compound[..])?.value else { return Err(Error::Actor("an actor that is no compound")) };
    for (key, value) in [("Pos", floats(&position)), ("Motion", floats(&motion))] {
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, old)) => *old = value,
            None => fields.push((key.to_owned(), value)),
        }
    }
    Ok(written(Value::Compound(fields)))
}

/// A new actor of `identifier`: its id, kind, where it is and how it moves, the least the game
/// reads to know one. Its `definitions` are BDS's way of naming the kind's behaviour.
pub fn new_actor(unique_id: i64, identifier: &str, position: [f32; 3], motion: [f32; 3]) -> Vec<u8> {
    let definitions = Value::List(List { tag: STRING_TAG, items: vec![Value::String(format!("+{identifier}"))] });
    let fields = [
        ("UniqueID", Value::Long(unique_id)),
        ("identifier", Value::String(identifier.to_owned())),
        ("definitions", definitions),
        ("Pos", floats(&position)),
        ("Rotation", floats(&[0.0, 0.0])),
        ("Motion", floats(&motion)),
    ];
    written(Value::Compound(fields.into_iter().map(|(key, value)| (key.to_owned(), value)).collect()))
}

fn actor_key(id: &[u8]) -> Vec<u8> {
    [ACTOR, id].concat()
}

pub(crate) fn load(db: &Db, key: ChunkKey) -> Result<Vec<u8>, Error> {
    let Some(ids) = db.get(&key.actor_digest())? else { return Ok(db.get(&key.tag(Tag::Entity))?.unwrap_or_default()) };
    let mut record = Vec::new();
    for id in ids.chunks_exact(8) {
        record.extend(db.get(&actor_key(id))?.unwrap_or_default());
    }
    Ok(record)
}

/// Adds what makes `record` the column's actors to `batch`; nothing when it already is.
pub(crate) fn store(db: &Db, batch: &mut Batch, key: ChunkKey, record: &[u8]) -> Result<(), Error> {
    if load(db, key)? == record {
        return Ok(());
    }
    let mut ids = Vec::new();
    for (raw, actor) in compounds(record)? {
        let Some(Value::Long(id)) = actor.get("UniqueID") else { return Err(Error::Actor("an actor without a UniqueID")) };
        ids.extend(id.to_le_bytes());
        batch.put(&actor_key(&id.to_le_bytes()), raw);
    }
    let kept: HashSet<&[u8]> = ids.chunks_exact(8).collect();
    let before = db.get(&key.actor_digest())?.unwrap_or_default();
    before.chunks_exact(8).filter(|id| !kept.contains(id)).for_each(|id| batch.delete(&actor_key(id)));
    batch.put(&key.actor_digest(), &ids);
    batch.delete(&key.tag(Tag::Entity));
    Ok(())
}

#[cfg(test)]
mod tests {
    use acacia_proto::nbt::List;

    use super::*;
    use crate::records::compound;

    fn floats(items: &[f32]) -> Value {
        Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() })
    }

    fn fields(fields: Vec<(&str, Value)>) -> Value {
        Value::Compound(fields.into_iter().map(|(key, value)| (key.to_string(), value)).collect())
    }

    fn stacks(names: &[&str]) -> Value {
        let stack = |name: &&str| fields(vec![("Name", Value::String(name.to_string())), ("Count", Value::Byte(1)), ("Damage", Value::Short(0))]);
        Value::List(List { tag: 10, items: names.iter().map(stack).collect() })
    }

    #[test]
    fn actors_are_read_from_their_compounds() {
        let cow = compound(vec![
            ("UniqueID", Value::Long(-4_294_967_295)),
            ("identifier", Value::String("minecraft:cow".into())),
            ("Pos", floats(&[1.5, 64.0, -2.5])),
            ("Rotation", floats(&[90.0, -10.0])),
            ("Variant", Value::Int(3)),
            ("IsBaby", Value::Byte(1)),
            ("Saddled", Value::Byte(1)),
            ("Sheared", Value::Byte(0)),
            ("Armor", stacks(&["", "minecraft:golden_boots"])),
            ("Mainhand", stacks(&[""])),
            ("Attributes", Value::List(List { tag: 10, items: vec![fields(vec![("Name", Value::String("minecraft:health".into())), ("Current", Value::Float(7.5))])] })),
            ("CustomName", Value::String("Bill".into())),
        ]);
        let nameless = compound(vec![("UniqueID", Value::Long(3))]);
        let found = actors(&[&cow[..], &nameless].concat()).unwrap();
        let expected = Actor {
            unique_id: -4_294_967_295,
            identifier: "minecraft:cow".into(),
            position: [1.5, 64.0, -2.5],
            yaw: 90.0,
            pitch: -10.0,
            variant: 3,
            mark_variant: 0,
            color: 0,
            baby: true,
            name: "Bill".into(),
            item: None,
            painting: None,
            states: vec![State::Saddled],
            armor: vec![None, Some(ItemStack { name: "minecraft:golden_boots".into(), count: 1, damage: 0, block: None, tag: None })],
            mainhand: None,
            offhand: None,
            health: Some(7.5),
        };
        assert_eq!(found, [expected]);
        assert!(actors(&cow[..cow.len() - 1]).is_err());
    }

    #[test]
    fn a_moved_actor_keeps_the_rest_and_a_new_one_reads_back() {
        let cow = compound(vec![
            ("UniqueID", Value::Long(9)),
            ("identifier", Value::String("minecraft:cow".into())),
            ("Pos", floats(&[1.5, 64.0, -2.5])),
            ("Rotation", floats(&[90.0, 0.0])),
            ("Variant", Value::Int(3)),
        ]);
        let moved = moved_actor(&cow, [1.5, 60.0, -2.5], [0.0, -0.2, 0.0]).unwrap();
        let read = actors(&moved).unwrap().remove(0);
        assert_eq!((read.position, read.variant, read.unique_id), ([1.5, 60.0, -2.5], 3, 9));
        let pig = new_actor(-7, "minecraft:pig", [0.5, 65.0, 0.5], [0.0; 3]);
        let record = [&moved[..], &pig].concat();
        let stored = stored_actors(&record).unwrap();
        assert_eq!(stored.iter().map(|(actor, id, raw)| (actor.as_ref().map(|a| a.identifier.clone()), *id, raw.len())).collect::<Vec<_>>(), [
            (Some("minecraft:cow".into()), Some(9), moved.len()),
            (Some("minecraft:pig".into()), Some(-7), pig.len())
        ]);
    }

    #[test]
    fn an_item_entity_says_what_lies_there() {
        let block = fields(vec![("name", Value::String("minecraft:stone".into())), ("states", fields(vec![])), ("version", Value::Int(1))]);
        let stack = fields(vec![("Name", Value::String("minecraft:stone".into())), ("Count", Value::Byte(12)), ("Damage", Value::Short(0)), ("Block", block)]);
        let item = compound(vec![
            ("UniqueID", Value::Long(7)),
            ("identifier", Value::String("minecraft:item".into())),
            ("Pos", floats(&[0.5, 64.0, 0.5])),
            ("Rotation", floats(&[0.0, 0.0])),
            ("Item", stack),
        ]);
        let found = actors(&item).unwrap().remove(0).item.expect("an item");
        let stone = acacia_world::BlockRegistry::vanilla();
        let hash = (0..stone.len() as u32).filter_map(|id| stone.get(id)).find(|state| state.name == "minecraft:stone").unwrap().network_hash;
        assert_eq!((found.name.as_str(), found.count, found.damage, found.block, found.tag), ("minecraft:stone", 12, 0, Some(hash), None));
    }
}
