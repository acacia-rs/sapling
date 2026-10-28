//! What a stored mob keeps of its raising, under BDS's names: its age, love and breeding cooldown,
//! owner, temper, lead holder, name and whether it stays (`Persistent`), and the flags `Actor`
//! reads too. Its `definitions` (the component groups BDS restores it with) follow its events.
// TODO: `Age` is the old worlds' field (negative ticks to grow up), the one BDS's upgraders read; where today's
// BDS keeps a baby's progress was not found.

use acacia_proto::nbt::{self, LittleEndian, List, Value};

use crate::actors::{STRING_TAG, with_fields, written};
use crate::Error;

/// A stored mob's raising; the actor's unique ids name its owner, the one whose food put it in
/// love, and what holds its lead.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Raising {
    /// Ticks a baby has to grow up; `None` for a grown one.
    pub baby: Option<i32>,
    pub in_love: i32,
    pub love_cause: i64,
    pub breed_cooldown: i32,
    pub owner: Option<i64>,
    pub tamed: bool,
    pub sitting: bool,
    pub sheared: bool,
    pub temper: i32,
    pub colour: i8,
    pub mark_variant: i32,
    pub name: String,
    pub persistent: bool,
    pub leasher: Option<i64>,
}

const NONE: i64 = -1;

/// What `compound` keeps of the mob's raising.
pub fn raising(compound: &[u8]) -> Result<Raising, Error> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..])?.value;
    let int = |key: &str| if let Some(Value::Int(int)) = actor.get(key) { *int } else { 0 };
    let byte = |key: &str| if let Some(Value::Byte(byte)) = actor.get(key) { *byte } else { 0 };
    let id = |key: &str| if let Some(Value::Long(id)) = actor.get(key) { Some(*id).filter(|id| *id != NONE) } else { None };
    Ok(Raising {
        baby: (byte("IsBaby") != 0).then(|| -int("Age")).map(|left| left.max(0)),
        in_love: int("InLove"),
        love_cause: id("LoveCause").unwrap_or(0),
        breed_cooldown: int("BreedCooldown"),
        owner: id("OwnerNew"),
        tamed: byte("IsTamed") != 0,
        sitting: byte("Sitting") != 0,
        sheared: byte("Sheared") != 0,
        temper: int("Temper"),
        colour: byte("Color"),
        mark_variant: int("MarkVariant"),
        name: if let Some(Value::String(name)) = actor.get("CustomName") { name.to_string() } else { String::new() },
        persistent: byte("Persistent") != 0,
        leasher: id("LeasherID"),
    })
}

/// `compound` keeping `raising`; the rest as it was. A name is written only where there is one.
pub fn raised_actor(compound: &[u8], raising: &Raising) -> Result<Vec<u8>, Error> {
    let byte = |on: bool| Value::Byte(i8::from(on));
    let fields = [
        ("IsBaby", byte(raising.baby.is_some())),
        ("Age", Value::Int(raising.baby.map_or(0, |left| -left))),
        ("InLove", Value::Int(raising.in_love)),
        ("LoveCause", Value::Long(raising.love_cause)),
        ("BreedCooldown", Value::Int(raising.breed_cooldown)),
        ("OwnerNew", Value::Long(raising.owner.unwrap_or(NONE))),
        ("IsTamed", byte(raising.tamed)),
        ("Sitting", byte(raising.sitting)),
        ("Sheared", byte(raising.sheared)),
        ("Temper", Value::Int(raising.temper)),
        ("Color", Value::Byte(raising.colour)),
        ("MarkVariant", Value::Int(raising.mark_variant)),
        ("Persistent", byte(raising.persistent)),
        ("LeasherID", Value::Long(raising.leasher.unwrap_or(NONE))),
    ];
    let kept = with_fields(compound, fields)?;
    match raising.name.is_empty() {
        true => Ok(kept),
        false => with_fields(&kept, [("CustomName", Value::String(raising.name.clone().into()))]),
    }
}

/// The component groups a stored actor's `definitions` list as on (`+`), without the sign.
pub fn actor_groups(compound: &[u8]) -> Vec<String> {
    let Ok(Value::Compound(fields)) = nbt::read::<LittleEndian>(&mut &compound[..]).map(|read| read.value) else { return Vec::new() };
    let Some((_, Value::List(list))) = fields.iter().find(|(name, _)| name == "definitions") else { return Vec::new() };
    list.items.iter().filter_map(|item| if let Value::String(group) = item { group.to_string().strip_prefix('+').map(str::to_owned) } else { None }).collect()
}

/// `compound` whose `definitions` lose `removed` and gain `added`, component group names that BDS
/// lists with a leading `+`.
pub fn regrouped_actor(compound: &[u8], removed: &[&str], added: &[&str]) -> Result<Vec<u8>, Error> {
    let Value::Compound(mut fields) = nbt::read::<LittleEndian>(&mut &compound[..])?.value else { return Err(Error::Actor("an actor that is no compound")) };
    let entry = |group: &str| format!("+{group}");
    let mut groups: Vec<String> = match fields.iter().find(|(name, _)| name == "definitions") {
        Some((_, Value::List(list))) => list.items.iter().filter_map(|item| if let Value::String(group) = item { Some(group.to_string()) } else { None }).collect(),
        _ => Vec::new(),
    };
    groups.retain(|group| !removed.iter().any(|gone| *group == entry(gone)));
    groups.extend(added.iter().map(|group| entry(group)).filter(|group| !groups.contains(group)).collect::<Vec<_>>());
    let list = Value::List(List { tag: STRING_TAG, items: groups.into_iter().map(|group| Value::String(group.into())).collect() });
    match fields.iter_mut().find(|(name, _)| name == "definitions") {
        Some((_, old)) => *old = list,
        None => fields.push(("definitions".into(), list)),
    }
    Ok(written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::{actors, new_actor};

    #[test]
    fn a_raised_mob_reads_back_as_it_was_kept() {
        let wolf = new_actor(-7, "minecraft:wolf", [0.5, 65.0, 0.5], [0.0; 3]);
        assert_eq!(raising(&wolf).unwrap(), Raising::default());
        let kept = Raising { baby: Some(1200), in_love: 300, love_cause: -9, breed_cooldown: 0, owner: Some(-9), tamed: true, sitting: true, sheared: false, temper: 0, colour: 14, mark_variant: 0, name: "Rex".into(), persistent: true, leasher: Some(42) };
        let raised = raised_actor(&wolf, &kept).unwrap();
        assert_eq!(raising(&raised).unwrap(), kept);
        let read = actors(&raised).unwrap().remove(0);
        assert_eq!((read.baby, read.name.as_str(), read.color, read.unique_id), (true, "Rex", 14, -7));
        assert!(read.states.contains(&crate::State::Tamed) && read.states.contains(&crate::State::Sitting));
    }

    #[test]
    fn an_event_swaps_the_groups_bds_restores_a_mob_with() {
        let wolf = new_actor(-7, "minecraft:wolf", [0.5, 65.0, 0.5], [0.0; 3]);
        let tamed = regrouped_actor(&regrouped_actor(&wolf, &[], &["minecraft:wolf_wild"]).unwrap(), &["minecraft:wolf_wild"], &["minecraft:wolf_tame"]).unwrap();
        let Value::Compound(fields) = nbt::read::<LittleEndian>(&mut &tamed[..]).unwrap().value else { panic!() };
        let Some((_, Value::List(groups))) = fields.iter().find(|(name, _)| name == "definitions") else { panic!() };
        let groups: Vec<String> = groups.items.iter().map(|item| if let Value::String(group) = item { group.to_string() } else { String::new() }).collect();
        assert_eq!(groups, ["+minecraft:wolf", "+minecraft:wolf_tame"]);
    }
}
