//! What a stored vehicle or mount keeps of its riders and its goods: `LinksTag`, a list of
//! `{entityID, linkID}` naming each rider's `UniqueID` by seat, and `ChestItems`, a chest boat's or
//! minecart's slots (`{Slot, ...item}`, as a container block's `Items`). Bytes like `Saddled` say
//! what it wears.

use acacia_proto::nbt::{self, List, LittleEndian, Str, Value};
use acacia_world::BlockRegistry;

use crate::actors::written;
use crate::{Error, ItemStack};

const LINKS: &str = "LinksTag";
const LINK_ID: &str = "linkID";
const CHEST_ITEMS: &str = "ChestItems";
const COMPOUND: u8 = 10;
const FLOAT: u8 = 5;

fn fields_of(compound: &[u8]) -> Result<Vec<(Str, Value)>, Error> {
    match nbt::read::<LittleEndian>(&mut &compound[..])?.value {
        Value::Compound(fields) => Ok(fields),
        _ => Err(Error::Actor("an actor that is no compound")),
    }
}

/// `fields` with `key` holding `value`, or without it for `None`.
fn put(fields: &mut Vec<(Str, Value)>, key: &str, value: Option<Value>) {
    fields.retain(|(name, _)| name != key);
    fields.extend(value.map(|value| (key.into(), value)));
}

/// The `UniqueID`s of a stored actor's riders, by seat.
pub fn actor_links(compound: &[u8]) -> Vec<i64> {
    let Ok(fields) = fields_of(compound) else { return Vec::new() };
    let Some((_, Value::List(links))) = fields.iter().find(|(name, _)| name == LINKS) else { return Vec::new() };
    let mut riders: Vec<(i32, i64)> = links
        .items
        .iter()
        .filter_map(|link| match (link.get("entityID"), link.get(LINK_ID)) {
            (Some(Value::Long(id)), Some(Value::Int(seat))) => Some((*seat, *id)),
            _ => None,
        })
        .collect();
    riders.sort_unstable_by_key(|(seat, _)| *seat);
    riders.into_iter().map(|(_, id)| id).collect()
}

/// A stored actor carrying `riders` (their `UniqueID`s by seat); none takes the field away.
pub fn linked_actor(compound: &[u8], riders: &[i64]) -> Result<Vec<u8>, Error> {
    let mut fields = fields_of(compound)?;
    let link = |(seat, id): (usize, &i64)| Value::Compound(vec![("entityID".into(), Value::Long(*id)), (LINK_ID.into(), Value::Int(seat as i32))]);
    let links = (!riders.is_empty()).then(|| Value::List(List { tag: COMPOUND, items: riders.iter().enumerate().map(link).collect() }));
    put(&mut fields, LINKS, links);
    Ok(written(Value::Compound(fields)))
}

/// The goods of a stored chest boat or minecart, with their slots.
pub fn actor_chest(compound: &[u8]) -> Vec<(u8, ItemStack)> {
    let Ok(fields) = fields_of(compound) else { return Vec::new() };
    let Some((_, Value::List(items))) = fields.iter().find(|(name, _)| name == CHEST_ITEMS) else { return Vec::new() };
    let slotted = |stored: &Value| match stored.get("Slot") {
        Some(Value::Byte(slot)) => Some((u8::try_from(*slot).ok()?, ItemStack::parse(stored)?)),
        _ => None,
    };
    items.items.iter().filter_map(slotted).collect()
}

/// A stored actor holding `items` as its goods.
pub fn chested_actor(compound: &[u8], items: &[(u8, ItemStack)], registry: &BlockRegistry) -> Result<Vec<u8>, Error> {
    let mut fields = fields_of(compound)?;
    let stored = |(slot, stack): &(u8, ItemStack)| {
        let mut fields = stack.fields(registry);
        fields.push(("Slot".into(), Value::Byte(*slot as i8)));
        Value::Compound(fields)
    };
    put(&mut fields, CHEST_ITEMS, Some(Value::List(List { tag: COMPOUND, items: items.iter().map(stored).collect() })));
    Ok(written(Value::Compound(fields)))
}

/// A stored actor of the look `variant` (a boat's wood) facing `yaw`.
pub fn varied_actor(compound: &[u8], variant: i32, yaw: f32) -> Result<Vec<u8>, Error> {
    let mut fields = fields_of(compound)?;
    put(&mut fields, "Variant", Some(Value::Int(variant)));
    put(&mut fields, "Rotation", Some(Value::List(List { tag: FLOAT, items: vec![Value::Float(yaw), Value::Float(0.0)] })));
    Ok(written(Value::Compound(fields)))
}

/// A stored actor with the byte `key` (`Saddled`, `Chested`) set to `on`.
pub fn flagged_actor(compound: &[u8], key: &str, on: bool) -> Result<Vec<u8>, Error> {
    let mut fields = fields_of(compound)?;
    put(&mut fields, key, Some(Value::Byte(on.into())));
    Ok(written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riders_and_goods_go_into_an_actor_and_come_back() {
        let boat = crate::new_actor(7, "minecraft:chest_boat", [0.5, 64.0, 0.5], [0.0; 3]);
        assert!(actor_links(&boat).is_empty());
        let linked = linked_actor(&boat, &[11, -4]).unwrap();
        assert_eq!(actor_links(&linked), [11, -4]);
        assert_eq!(linked_actor(&linked, &[]).unwrap(), boat, "no riders, no field");
        let stack = ItemStack { name: "minecraft:diamond".into(), count: 3, damage: 0, block: None, tag: None };
        let chested = chested_actor(&linked, &[(26, stack.clone())], &BlockRegistry::vanilla()).unwrap();
        assert_eq!(actor_chest(&chested), [(26, stack)]);
        assert_eq!(actor_links(&chested), [11, -4]);
        let saddled = flagged_actor(&boat, "Saddled", true).unwrap();
        assert!(crate::actors(&saddled).unwrap()[0].states.contains(&crate::State::Saddled));
        let birch = &crate::actors(&varied_actor(&boat, 2, 90.0).unwrap()).unwrap()[0];
        assert_eq!((birch.variant, birch.yaw), (2, 90.0));
    }
}
