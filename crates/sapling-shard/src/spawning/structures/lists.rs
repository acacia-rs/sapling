//! The structures' own spawn lists, as `StructureFeature` registers them (141f82140; each entry a
//! weight, an actor type and a packed `MobSpawnRules` that 143906b20 unpacks). See
//! docs/research/structure-spawns.md "The lists".

use sapling_data::Medium;

/// One kind a structure's list spawns.
pub(super) struct Entry {
    pub kind: &'static str,
    pub event: Option<&'static str>,
    pub weight: u32,
    pub herd: (u32, u32),
    /// The `population_control` of the kind's spawn rule file: the category its caps count in.
    pub category: &'static str,
    /// Its surface spawner flag: which category cap it is counted against, and what it spawns as.
    pub surface: bool,
    pub medium: Medium,
    /// Its own per-kind caps, surface then underground (`mSurfaceCap`, `mUndergroundCap`).
    pub own_caps: Option<[u32; 2]>,
    pub checks: Checks,
}

/// What the kind's own `checkSpawnRules` asks of its spot's raw brightness.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Checks {
    /// Nothing (its own rules, or `isDarkEnoughToSpawn` overridden to true and no walk target).
    None,
    /// `Monster::_getWalkTargetValue` (14298f730): 0.5 less the spot's brightness, not below 0.
    Walks,
    /// That and `Monster::isDarkEnoughToSpawn` (14298f930): raw brightness under 8.
    Dark,
}

const fn entry(kind: &'static str, weight: u32, least: u32, most: u32, surface: bool, checks: Checks) -> Entry {
    Entry { kind, event: None, weight, herd: (least, most), category: "monster", surface, medium: Medium::Air, own_caps: None, checks }
}

const FORTRESS: &[Entry] = &[
    entry("minecraft:zombie_pigman", 5, 2, 3, false, Checks::None),
    entry("minecraft:magma_cube", 3, 2, 3, false, Checks::None),
    entry("minecraft:blaze", 10, 2, 3, false, Checks::Walks),
    entry("minecraft:skeleton", 2, 2, 3, false, Checks::Dark),
    entry("minecraft:wither_skeleton", 8, 2, 3, false, Checks::Dark),
];
const MONUMENT: &[Entry] = &[Entry { medium: Medium::Water, own_caps: Some([0, 40]), ..entry("minecraft:guardian", 1, 2, 4, false, Checks::None) }];
const SWAMP_HUT: &[Entry] = &[entry("minecraft:witch", 1, 1, 1, true, Checks::Dark)];
const OUTPOST: &[Entry] = &[
    Entry { category: "pillager", own_caps: Some([7, 7]), ..entry("minecraft:pillager", 1, 1, 2, true, Checks::Walks) },
    Entry { event: Some("minecraft:spawn_as_illager_captain"), category: "pillager", own_caps: Some([1, 1]), ..entry("minecraft:pillager", 1, 1, 1, true, Checks::Walks) },
];

pub(super) fn list(structure: &str) -> Option<&'static [Entry]> {
    match structure {
        "minecraft:fortress" => Some(FORTRESS),
        "minecraft:monument" => Some(MONUMENT),
        "minecraft:swamp_hut" => Some(SWAMP_HUT),
        "minecraft:pillager_outpost" => Some(OUTPOST),
        _ => None,
    }
}

/// The category a structure's own kind counts in, for kinds no natural rule names (pillager, blaze, ...).
pub(in crate::spawning) fn category(kind: &str) -> Option<&'static str> {
    [FORTRESS, MONUMENT, SWAMP_HUT, OUTPOST].iter().flat_map(|list| list.iter()).find(|entry| entry.kind == kind).map(|entry| entry.category)
}

/// An entry of `entries` by their weights, by `roll` from 0 to 1 (BDS's `nextInt(total)`).
pub(super) fn pick(entries: &'static [Entry], roll: f32) -> &'static Entry {
    let total: u32 = entries.iter().map(|entry| entry.weight).sum();
    let mut left = ((roll * total as f32) as u32).min(total.saturating_sub(1));
    for entry in entries {
        if left < entry.weight {
            return entry;
        }
        left -= entry.weight;
    }
    &entries[0]
}

impl Checks {
    /// Whether a spot of `raw` brightness (`stormy`: as a thunderstorm reckons it) passes.
    pub fn pass(self, raw: u8, stormy: u8) -> bool {
        // 0.5 on the light ramp is raw 12 (despawn.rs MONSTER_BRIGHT is the first raw over it).
        let walks = raw <= super::super::despawn::MONSTER_BRIGHT - 1;
        match self {
            Checks::None => true,
            Checks::Walks => walks,
            Checks::Dark => walks && stormy < 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortress_entries_go_by_their_weights() {
        let picked: Vec<&str> = [0.0, 0.17, 0.2, 0.28, 0.3, 0.63, 0.7, 0.72, 0.99].map(|roll| pick(FORTRESS, roll).kind).to_vec();
        let kinds = ["zombie_pigman", "zombie_pigman", "magma_cube", "magma_cube", "blaze", "blaze", "skeleton", "wither_skeleton", "wither_skeleton"];
        assert_eq!(picked, kinds.map(|kind| format!("minecraft:{kind}")));
    }

    #[test]
    fn pillagers_count_as_pillagers_and_the_rest_as_monsters() {
        let kinds = ["minecraft:fortress", "minecraft:monument", "minecraft:swamp_hut", "minecraft:pillager_outpost", "minecraft:trial_chambers"];
        assert_eq!(kinds.map(|kind| list(kind).map_or(0, <[Entry]>::len)), [5, 1, 1, 2, 0]);
        assert_eq!([category("minecraft:pillager"), category("minecraft:witch"), category("minecraft:cow")], [Some("pillager"), Some("monster"), None]);
    }

    #[test]
    fn witches_want_the_dark_and_pillagers_only_the_shade() {
        assert!(Checks::Dark.pass(7, 7) && !Checks::Dark.pass(8, 8) && !Checks::Dark.pass(4, 9));
        assert!(Checks::Walks.pass(12, 12) && !Checks::Walks.pass(13, 13) && Checks::None.pass(15, 15));
    }
}
