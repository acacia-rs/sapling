//! The tags `/tag` puts on a player or an actor, kept as BDS keeps them: `Tags`, a list of
//! strings in the actor's compound (BDSRE: the `Tags` key of 1401ac2d0), left out without any.

use acacia_proto::nbt::{self, List, LittleEndian, Str, Value};

use crate::Error;
use crate::actors::written;

pub(crate) const TAGS: &str = "Tags";
const STRING: u8 = 8;

/// The tags a compound's fields hold, in the order kept.
pub(crate) fn read_tags(compound: &Value) -> Vec<String> {
    match compound.get(TAGS) {
        Some(Value::List(list)) => list.items.iter().filter_map(|item| if let Value::String(tag) = item { Some(tag.to_string()) } else { None }).collect(),
        _ => Vec::new(),
    }
}

/// Writes `tags` into `fields`, or takes the field away when there are none.
pub(crate) fn write_tags(fields: &mut Vec<(Str, Value)>, tags: &[String]) {
    fields.retain(|(name, _)| name != TAGS);
    if !tags.is_empty() {
        fields.push((TAGS.into(), Value::List(List { tag: STRING, items: tags.iter().map(|tag| Value::String(tag.as_str().into())).collect() })));
    }
}

/// The tags of one stored actor's compound.
pub fn actor_tags(compound: &[u8]) -> Vec<String> {
    nbt::read::<LittleEndian>(&mut &compound[..]).map(|tag| read_tags(&tag.value)).unwrap_or_default()
}

/// One stored actor's compound carrying `tags`; the rest as it was.
pub fn tagged_actor(compound: &[u8], tags: &[String]) -> Result<Vec<u8>, Error> {
    let Value::Compound(mut fields) = nbt::read::<LittleEndian>(&mut &compound[..])?.value else { return Err(Error::Actor("an actor that is no compound")) };
    write_tags(&mut fields, tags);
    Ok(written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_go_into_an_actor_and_come_back() {
        let pig = crate::new_actor(7, "minecraft:pig", [0.5, 64.0, 0.5], [0.0; 3]);
        let tagged = tagged_actor(&pig, &["vip".into(), "§cred".into()]).unwrap();
        assert_eq!(actor_tags(&tagged), ["vip", "§cred"]);
        assert_eq!(tagged_actor(&tagged, &[]).unwrap(), pig, "no tags, no field");
        assert!(actor_tags(&pig).is_empty());
    }
}
