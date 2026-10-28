//! A cartography table's craft (`CraftRecipeOptional`, research/captures/bds-maps.txt and
//! bds-stations.txt; docs/research/maps.md "Cartography"). The filled map in the input and the
//! special recipe the request names decide it: an empty map copies it (two of it), paper zooms it
//! out to its next zoom level's id, a glass pane locks it under a fresh id (damage 6); recipe 0 with
//! a name and nothing added renames it. A name given with any of them renames the result too.
// TODO: BDS locked maps under an id outside the unique-id run (255902174); sapling takes a fresh one.

use acacia_proto::nbt::Value;
use sapling_data::Made;
use sapling_data::enchant::anvil::with_name;

use super::{Forge, Forged, held, lay};
use crate::inventory::{Inventory, Place};
use crate::maps::{FILLED, map_id};

pub(crate) const CARTOGRAPHY_INPUT: usize = 12;
pub(super) const CARTOGRAPHY_ADDITIONAL: usize = 13;
const COPY: (&str, &str) = ("442d85ed-8272-4543-a6f1-418f90ded05d", "minecraft:empty_map");
const ZOOM: (&str, &str) = ("8b36268c-1829-483c-a0f1-993b7156a8f2", "minecraft:paper");
const LOCK: (&str, &str) = ("602234e4-cac1-4353-8bb7-b1ebff70024b", "minecraft:glass_pane");
const LOCKED_DAMAGE: i16 = 6;
const MAX_SCALE: u8 = 4;

/// What the table knows of the map in its input, and the id a lock would take.
#[derive(Clone, Debug)]
pub(crate) struct Chart {
    /// The input's map id, scale and whether it is locked, when the map is known.
    pub input: Option<(i64, u8, bool)>,
    pub fresh: i64,
}

/// `tag` naming the map `id`, made by the table, keys in byte order as BDS writes them.
pub(crate) fn remade(tag: Option<&Value>, id: i64) -> Option<Value> {
    let mut fields = match tag {
        Some(Value::Compound(fields)) => fields.clone(),
        _ => Vec::new(),
    };
    fields.retain(|(key, _)| key != "map_is_init" && key != "map_uuid");
    fields.extend([("map_is_init".into(), Value::Byte(1)), ("map_uuid".into(), Value::Long(id))]);
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Some(Value::Compound(fields))
}

impl Forge {
    pub fn chart(inventory: &mut Inventory, chart: &Chart, recipe: u32, name: Option<&str>) -> Result<Self, ()> {
        let map = held(inventory, CARTOGRAPHY_INPUT).filter(|held| held.stack.name == FILLED).ok_or(())?.stack.clone();
        let added = held(inventory, CARTOGRAPHY_ADDITIONAL).map(|held| held.stack.name.clone());
        let name = name.filter(|name| !name.is_empty());
        let named = |tag: Option<Value>| match name {
            Some(name) => with_name(tag, Some(name)),
            None => tag,
        };
        let is = |(uuid, item): (&str, &str)| sapling_data::multi_recipe(uuid) == Some(recipe) && added.as_deref() == Some(item);
        let known = chart.input.filter(|(id, ..)| map_id(&map) == Some(*id));
        let (made, count, damage, done) = if is(COPY) {
            (named(map.tag.clone()), 2, map.damage, Forged::Copied)
        } else if is(ZOOM) {
            let (id, ..) = known.filter(|(_, scale, locked)| *scale < MAX_SCALE && !locked).ok_or(())?;
            (named(remade(map.tag.as_ref(), id + 1)), 1, map.damage, Forged::Charted { from: id, to: id + 1, zoomed: true })
        } else if is(LOCK) {
            let (id, ..) = known.filter(|(.., locked)| !locked).ok_or(())?;
            (named(remade(map.tag.as_ref(), chart.fresh)), 1, LOCKED_DAMAGE, Forged::Charted { from: id, to: chart.fresh, zoomed: false })
        } else if added.is_none() && name.is_some() && !sapling_data::is_special_recipe(recipe) {
            (named(map.tag.clone()), 1, map.damage, Forged::Copied)
        } else {
            return Err(());
        };
        let from = sapling_world::ItemStack { damage, ..map.clone() };
        lay(inventory, &from, Made { name: map.name.clone(), tag: made }, count)?;
        let mut takes = vec![(Place::Ui(CARTOGRAPHY_INPUT), 1)];
        takes.extend(added.map(|_| (Place::Ui(CARTOGRAPHY_ADDITIONAL), 1)));
        Ok(Self::new(takes, done))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remade_map_is_initialised_under_its_new_id() {
        let tag = Value::Compound(vec![("map_name_index".into(), Value::Int(3)), ("map_uuid".into(), Value::Long(-7))]);
        let Some(Value::Compound(fields)) = remade(Some(&tag), -6) else { panic!("a compound") };
        assert_eq!(fields, [("map_is_init".into(), Value::Byte(1)), ("map_name_index".into(), Value::Int(3)), ("map_uuid".into(), Value::Long(-6))]);
    }
}
