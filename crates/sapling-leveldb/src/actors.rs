//! A column's actors (mobs, items, ...). Since 1.18.30 `digp<chunk key>` lists their unique ids,
//! 8 bytes each, and `actorprefix<id>` holds one compound per actor; older worlds keep the
//! compounds under the chunk's `Entity` tag. Either way this crate hands them out as one record
//! (records.rs). Sources: research/worlds-and-ecs.md "Actors".

use std::collections::HashSet;

use acacia_proto::nbt::Value;

use crate::keys::{ChunkKey, Tag};
use crate::records::compounds;
use crate::{Batch, Db, Error};

const ACTOR: &[u8] = b"actorprefix";

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
        Some(Self {
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
        })
    }
}

/// The actors of a record; a compound that lacks one of [`Actor`]'s fields is left out.
pub fn actors(record: &[u8]) -> Result<Vec<Actor>, Error> {
    Ok(compounds(record)?.iter().filter_map(|(_, actor)| Actor::parse(actor)).collect())
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

    #[test]
    fn actors_are_read_from_their_compounds() {
        let cow = compound(vec![
            ("UniqueID", Value::Long(-4_294_967_295)),
            ("identifier", Value::String("minecraft:cow".into())),
            ("Pos", floats(&[1.5, 64.0, -2.5])),
            ("Rotation", floats(&[90.0, -10.0])),
            ("Variant", Value::Int(3)),
            ("IsBaby", Value::Byte(1)),
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
        };
        assert_eq!(found, [expected]);
        assert!(actors(&cow[..cow.len() - 1]).is_err());
    }
}
