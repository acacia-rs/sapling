//! A loom's craft (`CraftLoom`): the banner in its input gains the named pattern in the dye's
//! colour, a `{Color, Pattern}` compound at the end of its tag's `Patterns`, keys in byte order as
//! vanilla's result (docs/research/workstations.md, "Loom"; research/captures/bds-stations.txt).
//! A pattern item in the third slot allows its own pattern alone and is kept; without one the
//! loom offers the plain patterns. BDS answered with the banner and dye slots emptied and the
//! result placed, nothing resent.
// TODO: the limit of six patterns is the Java game's; BDS's was not drilled.

use acacia_proto::nbt::{List, Value};
use sapling_data::Made;

use super::{Forge, Forged, held, lay};
use crate::inventory::{Inventory, Place};

pub(super) const LOOM_INPUT: usize = 9;
pub(super) const LOOM_DYE: usize = 10;
pub(super) const LOOM_MATERIAL: usize = 11;
const BANNER: &str = "minecraft:banner";
const MAX_PATTERNS: usize = 6;
const COMPOUND_TAG: u8 = 10;
/// The patterns a loom makes without a pattern item.
const PLAIN: [&str; 32] = [
    "bl", "br", "tl", "tr", "bs", "ts", "ls", "rs", "cs", "ms", "drs", "dls", "ss", "cr", "sc", "bt", "tt", "bts", "tts", "ld", "rd", "lud", "rud", "mc", "mr", "vh", "hh", "vhr", "hhb", "bo",
    "gra", "gru",
];
/// The pattern each `minecraft:<name>_banner_pattern` item allows.
const ITEM_PATTERNS: [(&str, &str); 10] = [
    ("flower", "flo"), ("creeper", "cre"), ("skull", "sku"), ("mojang", "moj"), ("field_masoned", "bri"),
    ("bordure_indented", "cbo"), ("piglin", "pig"), ("globe", "glb"), ("flow", "flw"), ("guster", "gus"),
];

fn allows(material: Option<&str>, pattern: &str) -> bool {
    let Some(name) = material else { return PLAIN.contains(&pattern) };
    let kind = name.strip_prefix("minecraft:").and_then(|bare| bare.strip_suffix("_banner_pattern"));
    ITEM_PATTERNS.iter().any(|(item, given)| Some(*item) == kind && *given == pattern)
}

/// `tag` with `{Color, Pattern}` added to its `Patterns`; `None` where it has its fill.
fn patterned(tag: Option<&Value>, pattern: &str, colour: u8) -> Option<Value> {
    let mut fields = match tag {
        Some(Value::Compound(fields)) => fields.clone(),
        _ => Vec::new(),
    };
    let layer = Value::Compound(vec![("Color".into(), Value::Int(colour.into())), ("Pattern".into(), Value::String(pattern.into()))]);
    match fields.iter_mut().find(|(key, _)| key == "Patterns") {
        Some((_, Value::List(list))) if list.items.len() >= MAX_PATTERNS => return None,
        Some((_, Value::List(list))) => list.items.push(layer),
        _ => {
            fields.push(("Patterns".into(), Value::List(List { tag: COMPOUND_TAG, items: vec![layer] })));
            fields.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }
    Some(Value::Compound(fields))
}

impl Forge {
    pub fn loom(inventory: &mut Inventory, pattern: &str) -> Result<Self, ()> {
        let banner = held(inventory, LOOM_INPUT).filter(|held| held.stack.name == BANNER).ok_or(())?.stack.clone();
        let colour = held(inventory, LOOM_DYE).and_then(|held| crate::fireworks::dye(&held.stack.name)).ok_or(())?;
        if !allows(held(inventory, LOOM_MATERIAL).map(|held| held.stack.name.as_str()), pattern) {
            return Err(());
        }
        let tag = patterned(banner.tag.as_ref(), pattern, colour).ok_or(())?;
        lay(inventory, &banner, Made { name: banner.name.clone(), tag: Some(tag) }, 1)?;
        Ok(Self::new(vec![(Place::Ui(LOOM_INPUT), 1), (Place::Ui(LOOM_DYE), 1)], Forged::Loomed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_goes_after_the_others_and_patterns_before_type() {
        let tag = Value::Compound(vec![("Type".into(), Value::Int(0))]);
        let once = patterned(Some(&tag), "bo", 1).unwrap();
        let Value::Compound(fields) = &once else { panic!() };
        assert_eq!(fields.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["Patterns", "Type"]);
        let mut full = once;
        for _ in 1..MAX_PATTERNS {
            full = patterned(Some(&full), "cr", 0).unwrap();
        }
        assert!(patterned(Some(&full), "cr", 0).is_none());
        assert!(allows(None, "bo") && !allows(None, "cre") && allows(Some("minecraft:creeper_banner_pattern"), "cre"));
    }
}
