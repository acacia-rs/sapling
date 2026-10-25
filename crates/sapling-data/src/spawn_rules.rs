//! Natural spawning's rules: each condition of BDS's vanilla `spawn_rules` (`data/spawn-rules.tsv`),
//! how far off each kind despawns (`data/despawns.tsv`) and the biome tags they test (biomes.rs).
//! README.md "Spawn rules".

mod biomes;

use std::sync::LazyLock;

use biomes::Filter;
pub use biomes::{biome_has_tag, biome_id};

/// Where a spot is: under open sky or below something, and in what.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medium {
    Air,
    Water,
    /// Air over lava, where a strider stands.
    Lava,
}

/// What a spawn rule tests of the spot a mob would appear at.
#[derive(Clone, Copy, Debug)]
pub struct Spot<'a> {
    pub biome: u32,
    /// Nothing that blocks movement above it.
    pub surface: bool,
    pub medium: Medium,
    /// Its raw brightness with the sky dimmed by the time and the weather, and what an
    /// `adjust_for_weather` rule reads: in a thunderstorm, as if dimmed by 10 (`canSpawnInConditions`).
    pub brightness: u8,
    pub weathered: u8,
    /// 0 peaceful to 3 hard.
    pub difficulty: u8,
    pub y: i32,
    /// From the nearest player.
    pub distance: f32,
    /// The block under it, namespaced.
    pub below: &'a str,
}

/// One condition of a kind's spawn rules.
#[derive(Clone, Debug)]
pub struct SpawnRule {
    pub kind: &'static str,
    /// Its `population_control`: `monster`, `animal`, `water_animal`, `ambient`.
    pub category: &'static str,
    surface: bool,
    underground: bool,
    underwater: bool,
    lava: bool,
    pub weight: u32,
    /// The least and most of a herd.
    pub herd: (u32, u32),
    /// Least and most brightness, and whether the weather's dimming counts.
    light: Option<(u8, u8, bool)>,
    difficulty: (u8, u8),
    height: Option<(i32, i32)>,
    /// From the nearest player, least and most.
    distance: (f32, f32),
    /// How many of its kind may be about already, on the surface and underground.
    pub density: (Option<u32>, Option<u32>),
    on: Vec<&'static str>,
    not_on: Vec<&'static str>,
    /// Kinds it may appear as instead, by weight.
    pub permute: Vec<(u32, &'static str)>,
    biomes: Filter,
}

const DIFFICULTIES: [&str; 4] = ["peaceful", "easy", "normal", "hard"];
/// A rule's distances from the nearest player where it names none (BDS's spawn rule parser).
const DISTANCE: (f32, f32) = (24.0, 128.0);
/// At BDS's default chunk tick range of 4, no mob spawns farther from a player than this.
const TICK_RANGE_FAR: f32 = 44.0;

fn range<T: std::str::FromStr>(text: &str) -> Option<(T, T)> {
    let (least, most) = text.split_once('~')?;
    Some((least.parse().ok()?, most.parse().ok()?))
}

fn parse(row: &'static str) -> Option<SpawnRule> {
    let [kind, category, places, weight, herd, fields, biomes] = row.split('\t').collect::<Vec<_>>().try_into().ok()?;
    let has = |place: &str| places.split(',').any(|own| own == place);
    let (least, most) = herd.split_once('-')?;
    let mut rule = SpawnRule {
        kind,
        category,
        surface: has("surface"),
        underground: has("underground"),
        underwater: has("underwater"),
        lava: has("lava"),
        weight: weight.parse().ok()?,
        herd: (least.parse().ok()?, most.parse().ok()?),
        light: None,
        difficulty: (0, 3),
        height: None,
        distance: DISTANCE,
        density: (None, None),
        on: Vec::new(),
        not_on: Vec::new(),
        permute: Vec::new(),
        biomes: Filter::parse(biomes),
    };
    for field in fields.split(' ').filter(|field| !field.is_empty()) {
        let (key, value) = field.split_once('=').unwrap_or((field, ""));
        match key {
            "light" => rule.light = range(value.trim_end_matches('w')).map(|(least, most)| (least, most, value.ends_with('w'))),
            "difficulty" => {
                let (least, most) = value.split_once('~')?;
                let level = |name: &str| DIFFICULTIES.iter().position(|own| *own == name).map(|level| level as u8);
                rule.difficulty = (level(least)?, level(most)?);
            }
            "height" => rule.height = range(value),
            "distance" => rule.distance = range(value)?,
            "density" => {
                let (surface, underground) = value.split_once(':')?;
                rule.density = (surface.parse().ok(), underground.parse().ok());
            }
            "on" => rule.on = value.split(',').collect(),
            "not_on" => rule.not_on = value.split(',').collect(),
            "permute" => rule.permute = value.split(',').filter_map(|entry| entry.split_once(':')).filter_map(|(weight, kind)| Some((weight.parse().ok()?, kind))).collect(),
            _ => {}
        }
    }
    Some(rule)
}

static RULES: LazyLock<Vec<SpawnRule>> = LazyLock::new(|| include_str!("../data/spawn-rules.tsv").lines().filter_map(parse).collect());

/// Every natural spawn condition of the vanilla packs, by kind.
pub fn spawn_rules() -> &'static [SpawnRule] {
    &RULES
}

impl SpawnRule {
    /// Whether a mob of this rule may appear at `spot`.
    pub fn admits(&self, spot: &Spot) -> bool {
        let placed = if spot.surface { self.surface } else { self.underground };
        let medium = match spot.medium {
            Medium::Air => !self.underwater && !self.lava,
            Medium::Water => self.underwater,
            Medium::Lava => self.lava,
        };
        let (near, far) = self.distance;
        let lit = self.light.is_none_or(|(least, most, weather)| (least..=most).contains(&if weather { spot.weathered } else { spot.brightness }));
        placed
            && medium
            && lit
            && (self.difficulty.0..=self.difficulty.1).contains(&spot.difficulty)
            && self.height.is_none_or(|(least, most)| (least..=most).contains(&spot.y))
            && (near..=far.min(TICK_RANGE_FAR)).contains(&spot.distance)
            && (self.on.is_empty() || self.on.contains(&spot.below))
            && !self.not_on.contains(&spot.below)
            && self.biomes.admits(spot.biome)
    }

    /// How many of its kind may be about already where `spot` is; `None` for no limit.
    pub fn density_at(&self, surface: bool) -> Option<u32> {
        if surface { self.density.0 } else { self.density.1 }
    }

    /// The kind a mob of this rule appears as, by `roll` from 0 to 1.
    pub fn permuted(&self, roll: f32) -> &'static str {
        let total: u32 = self.permute.iter().map(|(weight, _)| weight).sum();
        let mut left = roll * total as f32;
        for &(weight, kind) in &self.permute {
            if left < weight as f32 {
                return kind;
            }
            left -= weight as f32;
        }
        self.kind
    }
}

/// The `population_control` of `kind`'s spawn rules; `None` for a kind without any.
pub fn spawn_category(kind: &str) -> Option<&'static str> {
    spawn_rules().iter().find(|rule| rule.kind == kind).map(|rule| rule.category)
}

/// From how far off a mob of `kind` may despawn and from how far it always does (its
/// `minecraft:despawn`'s `despawn_from_distance`); `None` for a kind that never despawns so.
pub fn despawn_distances(kind: &str) -> Option<(f32, f32)> {
    let row = include_str!("../data/despawns.tsv").lines().filter_map(|row| row.split_once('\t')).find(|(name, _)| *name == kind)?;
    let (near, far) = row.1.split_once('-')?;
    Some((near.parse().ok()?, far.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAINS: u32 = 1;
    const HELL: u32 = 8;

    fn rule(kind: &str, nth: usize) -> &'static SpawnRule {
        spawn_rules().iter().filter(|rule| rule.kind == kind).nth(nth).unwrap_or_else(|| panic!("{kind} {nth}"))
    }

    fn dark_grass() -> Spot<'static> {
        Spot { biome: PLAINS, surface: true, medium: Medium::Air, brightness: 0, weathered: 0, difficulty: 2, y: 64, distance: 30.0, below: "minecraft:grass_block" }
    }

    #[test]
    fn every_row_reads() {
        assert_eq!(spawn_rules().len(), include_str!("../data/spawn-rules.tsv").lines().count());
        assert!(spawn_rules().iter().all(|rule| rule.weight > 0 && rule.herd.0 <= rule.herd.1));
    }

    #[test]
    fn a_zombie_wants_the_dark_and_a_difficulty() {
        let zombie = rule("minecraft:zombie", 1);
        assert_eq!((zombie.category, zombie.weight, zombie.herd), ("monster", 100, (2, 4)));
        assert!(zombie.admits(&dark_grass()));
        assert!(zombie.admits(&Spot { surface: false, below: "minecraft:stone", ..dark_grass() }));
        assert!(!zombie.admits(&Spot { brightness: 8, weathered: 8, ..dark_grass() }));
        // A thunderstorm dims the sky enough for it.
        assert!(zombie.admits(&Spot { brightness: 10, weathered: 7, ..dark_grass() }));
        // 24 from the nearest player at least, and within the 44 of a tick range of 4.
        assert!(!zombie.admits(&Spot { distance: 20.0, ..dark_grass() }) && !zombie.admits(&Spot { distance: 50.0, ..dark_grass() }));
        assert!(!zombie.admits(&Spot { difficulty: 0, ..dark_grass() }));
        assert!(!zombie.admits(&Spot { medium: Medium::Water, ..dark_grass() }));
        assert!(!zombie.admits(&Spot { biome: HELL, ..dark_grass() }));
        assert_eq!((zombie.permuted(0.0), zombie.permuted(0.97)), ("minecraft:zombie", "minecraft:zombie_villager_v2"));
    }

    #[test]
    fn a_cow_wants_lit_grass_under_the_sky() {
        let cow = rule("minecraft:cow", 0);
        let lit = Spot { brightness: 15, weathered: 12, difficulty: 0, ..dark_grass() };
        assert!(cow.admits(&lit));
        assert!(!cow.admits(&Spot { below: "minecraft:stone", ..lit }));
        assert!(!cow.admits(&Spot { surface: false, ..lit }));
        assert!(!cow.admits(&Spot { brightness: 6, ..lit }));
        assert_eq!(cow.permuted(0.5), "minecraft:cow");
    }

    #[test]
    fn nether_and_water_rules_test_their_medium_and_place() {
        let strider = rule("minecraft:strider", 0);
        let lava = Spot { biome: HELL, surface: false, medium: Medium::Lava, below: "minecraft:lava", ..dark_grass() };
        assert!(strider.admits(&lava) && !strider.admits(&Spot { medium: Medium::Air, ..lava }));
        let cod = rule("minecraft:cod", 0);
        let sea = Spot { biome: 0, medium: Medium::Water, y: 50, distance: 20.0, below: "minecraft:water", ..dark_grass() };
        assert!(cod.admits(&sea));
        assert!(!cod.admits(&Spot { distance: 40.0, ..sea }) && !cod.admits(&Spot { y: 70, ..sea }));
        assert_eq!((cod.density_at(true), cod.density_at(false)), (Some(20), None));
    }

    #[test]
    fn spots_on_prevented_blocks_are_refused() {
        let piglin = rule("minecraft:zombie_pigman", 0);
        let wastes = Spot { biome: HELL, surface: false, below: "minecraft:netherrack", ..dark_grass() };
        assert!(piglin.admits(&wastes));
        assert!(!piglin.admits(&Spot { below: "minecraft:shroomlight", ..wastes }));
    }

    #[test]
    fn despawning_reads_each_kinds_distances() {
        assert_eq!(despawn_distances("minecraft:zombie"), Some((32.0, 128.0)));
        assert_eq!(despawn_distances("minecraft:cod"), Some((32.0, 40.0)));
        assert_eq!(despawn_distances("minecraft:villager_v2"), None);
    }
}
