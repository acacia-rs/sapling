//! What a trial spawner spawns and how many: BDS's built-in configs (`TrialSpawnerConfigRegistry`,
//! in the server, not in the packs), named in the entity's `normal_config` and `ominous_config`, or
//! written there whole.

use acacia_proto::nbt::Value;

use super::super::fields::Fields;

const PREFIX: &str = "minecraft:trial_chamber/";
const KEY: &str = "loot_tables/spawners/trial_chamber/key.json";
const CONSUMABLES: &str = "loot_tables/spawners/trial_chamber/consumables.json";
const OMINOUS_KEY: &str = "loot_tables/spawners/ominous/trial_chamber/key.json";
const OMINOUS_CONSUMABLES: &str = "loot_tables/spawners/ominous/trial_chamber/consumables.json";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Config {
    pub total: f32,
    pub simultaneous: f32,
    pub total_per_player: f32,
    pub simultaneous_per_player: f32,
    pub ticks_between: i64,
    pub spawn_range: i32,
    pub cooldown: i64,
    /// Kinds by weight; a baby where the flag says.
    pub potentials: Vec<(String, u32, bool)>,
    /// Loot tables a reward is rolled from, by weight.
    pub loot: Vec<(String, u32)>,
}

impl Config {
    /// `TrialSpawnerConfig`'s constructor: nothing to spawn yet.
    fn base(ominous: bool) -> Self {
        let loot = if ominous { [(OMINOUS_KEY, 3), (OMINOUS_CONSUMABLES, 7)] } else { [(KEY, 1), (CONSUMABLES, 1)] };
        Self {
            total: 6.0,
            simultaneous: 2.0,
            total_per_player: 2.0,
            simultaneous_per_player: 1.0,
            ticks_between: 20,
            spawn_range: 4,
            cooldown: 36_000,
            potentials: Vec::new(),
            loot: loot.map(|(table, weight)| (table.to_owned(), weight)).to_vec(),
        }
    }

    /// The most mobs a wave of `players` spawns, and that may be about at once.
    pub fn caps(&self, players: usize) -> (f32, f32) {
        let more = players.saturating_sub(1) as f32;
        (self.total + self.total_per_player * more, self.simultaneous + self.simultaneous_per_player * more)
    }
}

/// `(kind, baby)` and the numbers (total, at once, total per player, at once per player, ticks
/// between) each built-in config has, normal then ominous; `None` keeps the default.
type Numbers = [Option<f32>; 5];
const BUILT_IN: &[(&str, &str, bool, Numbers, Numbers)] = &[
    ("breeze", "minecraft:breeze", false, [Some(2.0), Some(1.0), Some(1.0), Some(0.5), None], [Some(4.0), Some(2.0), Some(1.0), Some(0.5), None]),
    ("melee/husk", "minecraft:husk", false, THREE, THREE),
    ("melee/spider", "minecraft:spider", false, THREE, TWELVE),
    ("melee/zombie", "minecraft:zombie", false, THREE, THREE),
    ("ranged/poison_skeleton", "minecraft:bogged", false, THREE, THREE),
    ("ranged/skeleton", "minecraft:skeleton", false, THREE, THREE),
    ("ranged/stray", "minecraft:stray", false, THREE, THREE),
    ("slow_ranged/poison_skeleton", "minecraft:bogged", false, SLOW, SLOW),
    ("slow_ranged/skeleton", "minecraft:skeleton", false, SLOW, SLOW),
    ("slow_ranged/stray", "minecraft:stray", false, SLOW, SLOW),
    ("small_melee/baby_zombie", "minecraft:zombie", true, HALF, HALF),
    ("small_melee/cave_spider", "minecraft:cave_spider", false, THREE, TWELVE),
    ("small_melee/silverfish", "minecraft:silverfish", false, THREE, TWELVE),
    ("small_melee/slime", "minecraft:slime", false, THREE, TWELVE),
];
const THREE: Numbers = [None, Some(3.0), None, Some(0.5), None];
const TWELVE: Numbers = [Some(12.0), Some(4.0), None, Some(0.5), None];
const SLOW: Numbers = [None, Some(4.0), None, Some(2.0), Some(160.0)];
const HALF: Numbers = [None, None, None, Some(0.5), None];

fn built_in(key: &str) -> Option<Config> {
    let (path, variant) = key.strip_prefix(PREFIX)?.rsplit_once('/')?;
    let ominous = match variant {
        "normal" => false,
        "ominous" => true,
        _ => return None,
    };
    let &(_, kind, baby, normal, charged) = BUILT_IN.iter().find(|(name, ..)| *name == path)?;
    let numbers = if ominous { charged } else { normal };
    let mut config = Config::base(ominous);
    let [total, simultaneous, total_per_player, simultaneous_per_player, ticks] = numbers;
    config.total = total.unwrap_or(config.total);
    config.simultaneous = simultaneous.unwrap_or(config.simultaneous);
    config.total_per_player = total_per_player.unwrap_or(config.total_per_player);
    config.simultaneous_per_player = simultaneous_per_player.unwrap_or(config.simultaneous_per_player);
    config.ticks_between = ticks.map_or(config.ticks_between, |ticks| ticks as i64);
    config.potentials = vec![(kind.to_owned(), 1, baby)];
    Some(config)
}

/// A config written whole (BDS's inline form): its numbers, its `spawn_potentials` and its
/// `loot_tables_to_eject`, each field the default where missing.
// TODO: [U] the inline potentials' keys; read here as a monster spawner's `SpawnData`.
fn inline(fields: &Fields, ominous: bool) -> Config {
    let mut config = Config::base(ominous);
    config.total = fields.float("total_mobs", config.total);
    config.simultaneous = fields.float("simultaneous_mobs", config.simultaneous);
    config.total_per_player = fields.float("total_mobs_added_per_player", config.total_per_player);
    config.simultaneous_per_player = fields.float("simultaneous_mobs_added_per_player", config.simultaneous_per_player);
    config.ticks_between = fields.int("ticks_between_spawn", config.ticks_between);
    config.spawn_range = fields.int("spawn_range", config.spawn_range.into()) as i32;
    config.cooldown = fields.int("target_cooldown_length", config.cooldown);
    config.potentials = fields.list("spawn_potentials").iter().filter_map(|data| Some((spawn_kind(data)?, weight(data), matches!(data.get("IsBaby"), Some(Value::Byte(1)))))).collect();
    let loot: Vec<(String, u32)> = fields.list("loot_tables_to_eject").iter().filter_map(|entry| Some((name_of(entry)?, weight(entry)))).collect();
    if !loot.is_empty() {
        config.loot = loot;
    }
    config
}

fn weight(data: &Value) -> u32 {
    match data.get("Weight").or_else(|| data.get("weight")) {
        Some(Value::Int(weight)) => (*weight).max(0) as u32,
        _ => 1,
    }
}

fn name_of(entry: &Value) -> Option<String> {
    ["name", "loot_table", "data"].iter().find_map(|key| if let Some(Value::String(name)) = entry.get(key) { Some(name.to_string()) } else { None })
}

/// The kind a `SpawnData` names: `TypeId`, else `entity`'s `id`.
pub(super) fn spawn_kind(data: &Value) -> Option<String> {
    let named = match data.get("TypeId") {
        Some(Value::String(kind)) => Some(kind.to_string()),
        _ => match data.get("entity").and_then(|entity| entity.get("id")) {
            Some(Value::String(kind)) => Some(kind.to_string()),
            _ => None,
        },
    };
    named.filter(|kind| !kind.is_empty())
}

/// The config of a spawner's entity that applies, ominous or not: the one it names or holds,
/// else the ominous one's normal counterpart, else the default.
pub(super) fn of(fields: &Fields, ominous: bool) -> Config {
    let read = |key: &str| match fields.get(key) {
        Some(Value::String(name)) => built_in(name),
        Some(Value::Compound(_)) => Some(inline(&fields.inner(key), ominous)),
        _ => None,
    };
    let normal = || read("normal_config").map(|config| Config { loot: Config::base(ominous).loot, ..config });
    let found = if ominous { read("ominous_config").or_else(normal) } else { read("normal_config") };
    found.unwrap_or_else(|| Config::base(ominous))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(normal: &str) -> Fields {
        Fields(vec![("normal_config".into(), Value::String(normal.into())), ("ominous_config".into(), Value::String(normal.replace("/normal", "/ominous").into()))])
    }

    #[test]
    fn built_in_configs_scale_with_players() {
        let fields = named("minecraft:trial_chamber/breeze/normal");
        let breeze = of(&fields, false);
        assert_eq!((breeze.potentials[0].0.as_str(), breeze.caps(1), breeze.caps(3)), ("minecraft:breeze", (2.0, 1.0), (4.0, 2.0)));
        assert_eq!(of(&fields, true).caps(1), (4.0, 2.0));
        assert_eq!(of(&fields, true).loot[0], (OMINOUS_KEY.to_owned(), 3));
        let slow = of(&named("minecraft:trial_chamber/slow_ranged/stray/normal"), false);
        assert_eq!((slow.ticks_between, slow.caps(2)), (160, (8.0, 6.0)));
        assert!(of(&named("minecraft:trial_chamber/small_melee/baby_zombie/normal"), false).potentials[0].2);
    }

    #[test]
    fn a_spawner_without_a_config_has_the_default_and_nothing_to_spawn() {
        let config = of(&Fields(Vec::new()), false);
        assert_eq!((config.caps(1), config.cooldown, config.potentials.len()), ((6.0, 2.0), 36_000, 0));
    }
}
