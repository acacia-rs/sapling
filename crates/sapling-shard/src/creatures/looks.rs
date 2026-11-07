//! The looks horses and tropical fish come in, as their packs' events set them (bds-h3herds.txt):
//! a horse's `entity_spawned` rolls one of 7 coats (`Variant`) and 5 markings (`MarkVariant`), and
//! its `make_<coat>` sets the coat; a tropical fish's rolls its shape (`Variant` 0-1), pattern
//! (`MarkVariant` 0-5), and base and pattern colours (`Color`, `Color2`, 0-14), and its
//! `become_<name>` sets all four. A summon's event replaces `entity_spawned` (BDS's
//! `make_black` horse came black with no markings); a herd's is fired after it, so a herd's horses
//! share a coat and keep their rolled markings (docs/research/husbandry.md "Herds"). A cat's
//! `spawn_midnight_cat` (the swamp hut's) makes it black; its `entity_spawned` rolls its coat.

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::genes::Gene;
use super::husbandry::Raised;
use crate::blocks::Dice;

const HORSE: &str = "minecraft:horse";
const FISH: &str = "minecraft:tropicalfish";
const CAT: &str = "minecraft:cat";
/// The swamp hut's cat (`spawn_midnight_cat`), as BDS showed it (bds-h3keep.txt).
const BLACK_CAT: i32 = 9;
/// A cat's `entity_spawned` picks one of 11 coats, each 15 in 165 (BDS showed a summoned cat's 6, bds-ar2look.txt).
const CAT_COATS: u32 = 11;
const COATS: [&str; 7] = ["white", "creamy", "chestnut", "brown", "black", "gray", "darkbrown"];
const MARKINGS: u32 = 5;
const SHAPES: u32 = 2;
const PATTERNS: u32 = 6;
const COLOURS: u32 = 15;
/// `become_<name>`: its `Variant`, `MarkVariant`, `Color` and `Color2` (the pack's groups).
const FISH_LOOKS: [(&str, [i32; 4]); 22] = [
    ("anenonme", [1, 1, 1, 7]), ("black_tang", [1, 0, 7, 7]), ("blue_dory", [0, 1, 7, 3]), ("butterfly_fish", [0, 4, 0, 7]), ("cichlid", [0, 1, 11, 7]),
    ("clownfish", [0, 0, 1, 0]), ("cc_betta", [0, 5, 6, 3]), ("dog_fish", [1, 3, 10, 4]), ("e_red_snapper", [1, 5, 0, 14]), ("goat_fish", [0, 5, 0, 4]),
    ("moorish_idol", [1, 2, 0, 7]), ("ornate_butterfly", [1, 5, 0, 1]), ("parrot_fish", [0, 3, 9, 6]), ("queen_angel_fish", [0, 4, 5, 3]), ("red_cichlid", [1, 4, 14, 0]),
    ("red_lipped_benny", [0, 2, 7, 14]), ("red_snapper", [1, 3, 14, 0]), ("threadfin", [1, 0, 0, 4]), ("tomato_clown", [0, 1, 14, 0]), ("triggerfish", [0, 1, 7, 0]),
    ("yellow_tang", [1, 1, 4, 4]), ("yellow_tail_parrot", [0, 3, 9, 4]),
];

/// A mob's `Variant`, `MarkVariant`, `Color` and `Color2`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Look([i32; 4]);

impl Look {
    pub fn gene(self) -> Gene {
        Gene(self.0[0])
    }
}

fn name(event: &str) -> &str {
    event.trim_start_matches("minecraft:")
}

/// The look a new mob of `kind` comes in: rolled by its `entity_spawned`, or what its spawn
/// `event` sets in its place; `None` for other kinds, or an event that sets none.
pub(super) fn of(world: &mut World, kind: &str, event: Option<&str>) -> Option<Look> {
    match event.filter(|event| name(event) != "entity_spawned") {
        None => rolled(world, kind),
        Some(event) => fired(kind, event, Look::default()),
    }
}

fn rolled(world: &mut World, kind: &str) -> Option<Look> {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let mut roll = |of: u32| dice.roll(of) as i32;
    match kind {
        HORSE => Some(Look([roll(COATS.len() as u32), roll(MARKINGS), 0, 0])),
        FISH => Some(Look([roll(SHAPES), roll(PATTERNS), roll(COLOURS), roll(COLOURS)])),
        CAT => Some(Look([roll(CAT_COATS), 0, 0, 0])),
        _ => None,
    }
}

/// `look` once `event` was fired on a mob of `kind`; `None` where the event sets no look.
pub(super) fn fired(kind: &str, event: &str, look: Look) -> Option<Look> {
    match kind {
        HORSE => {
            let coat = COATS.iter().position(|coat| name(event).strip_prefix("make_") == Some(coat))?;
            Some(Look([coat as i32, look.0[1], look.0[2], look.0[3]]))
        }
        FISH => FISH_LOOKS.iter().find(|(fish, _)| name(event).strip_prefix("become_") == Some(fish)).map(|(_, look)| Look(*look)),
        CAT if name(event) == "spawn_midnight_cat" => Some(Look([BLACK_CAT, 0, 0, 0])),
        _ => None,
    }
}

/// The mob's look now, as its gene and raising keep it.
pub(super) fn worn(world: &World, entity: Entity) -> Look {
    let variant = world.get::<Gene>(entity).map_or(0, |gene| gene.0);
    let kept = world.get::<Raised>(entity).map(|raised| &raised.kept);
    Look([variant, kept.map_or(0, |kept| kept.mark_variant), kept.map_or(0, |kept| kept.colour.into()), kept.map_or(0, |kept| kept.colour2.into())])
}

/// Gives the mob `look`, kept with it; its `Variant` is its gene's (genes.rs), saved by the caller.
pub(super) fn wear(world: &mut World, entity: Entity, look: Look) {
    world.entity_mut(entity).insert(look.gene());
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        let [_, mark, colour, colour2] = look.0;
        (raised.kept.mark_variant, raised.kept.colour, raised.kept.colour2) = (mark, colour as i8, colour2 as i8);
    }
}

/// A tropical fish's announcement with its `Color2`, which the recorded announcement fixes.
pub(super) fn dressed(mut shown: Vec<Bytes>, kind: &str, colour2: i8) -> Vec<Bytes> {
    let Some(mut added) = shown.first().filter(|_| kind == FISH).and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok()) else { return shown };
    for item in added.metadata.iter_mut().filter(|item| item.key == Key::Color2) {
        item.value = Value::Default(Plain::Byte(colour2));
    }
    shown[0] = sapling_net::encode(&added);
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_set_the_looks_bds_showed() {
        assert_eq!(fired(HORSE, "minecraft:make_black", Look::default()), Some(Look([4, 0, 0, 0])));
        assert_eq!(fired(HORSE, "minecraft:make_darkbrown", Look([2, 3, 0, 0])), Some(Look([6, 3, 0, 0])));
        assert_eq!(fired(FISH, "minecraft:become_clownfish", Look::default()), Some(Look([0, 0, 1, 0])));
        assert_eq!(fired(FISH, "minecraft:become_red_snapper", Look([0, 5, 2, 2])), Some(Look([1, 3, 14, 0])));
        assert_eq!(fired(HORSE, "minecraft:spawn_adult", Look::default()), None);
        assert_eq!(fired(CAT, "minecraft:spawn_midnight_cat", Look::default()), Some(Look([9, 0, 0, 0])));
    }
}
