//! What a world keeps besides its columns, and how a world folder reads and writes it.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use acacia_proto::nbt::Value;
use sapling_data::{GameRules, RuleValue};
use sapling_leveldb::{DragonFightRecord, LevelDat, LevelDbWorld, PortalRecord, ScoreboardRecord, TickingAreaRecord};

/// What a world keeps besides its columns; storage saves it when the world closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    /// The spawn block; y 32767 leaves the height to the surface, as vanilla saves it.
    pub spawn: Option<[i32; 3]>,
    /// The time of day counter: a day is 24000 long and the count runs on past it, while the
    /// `doDayLightCycle` rule is on.
    pub time: i64,
    pub rules: GameRules,
    /// The lit Nether portals of every dimension, by which a portal finds the one it leads to.
    pub portals: Vec<PortalRecord>,
    /// The world seed (`RandomSeed`): BDS seeds the draws of its portal making with it.
    pub seed: i64,
    /// The End's dragon fight, once one began (`TheEnd`).
    pub dragon_fight: Option<DragonFightRecord>,
    /// The scoreboard as storage holds it: what was read until the shard takes it, and what the
    /// shard hands back to be saved.
    pub scoreboard: Option<ScoreboardRecord>,
    /// Structure templates saved in the world (`.mcstructure` bytes) by namespaced name; shared,
    /// as every tick copies the level into the realm's other dimensions.
    pub structures: Arc<BTreeMap<String, Vec<u8>>>,
    /// Those of the folder's `structures` directory: read, never written back.
    pub folder_structures: Arc<BTreeMap<String, Vec<u8>>>,
    /// The ticking areas of every dimension.
    pub ticking_areas: Vec<TickingAreaRecord>,
    /// `IsHardcore`: one life, after which a player only watches.
    pub hardcore: bool,
}

impl Default for Level {
    /// Noon, since a world without storage has nothing to say when it is.
    fn default() -> Self {
        Self {
            spawn: None,
            time: 6000,
            rules: GameRules::default(),
            portals: Vec::new(),
            seed: 0,
            dragon_fight: None,
            scoreboard: None,
            structures: Arc::default(),
            folder_structures: Arc::default(),
            ticking_areas: Vec::new(),
            hardcore: false,
        }
    }
}

/// The rules `level.dat` sets: each under its name in lower case, a byte for a switch.
pub(crate) fn read_rules(dat: &LevelDat) -> GameRules {
    let mut rules = GameRules::default();
    let names: Vec<&str> = rules.iter().map(|(name, _)| name).collect();
    for name in names {
        let value = match dat.root.get(&name.to_ascii_lowercase()) {
            Some(Value::Byte(on)) => RuleValue::Bool(*on != 0),
            Some(Value::Int(n)) => RuleValue::Int(*n),
            _ => continue,
        };
        rules.set(name, value);
    }
    rules
}

/// Writes the rules `level.dat` held already and those that are not BDS's default.
pub(crate) fn write_rules(dat: &mut LevelDat, rules: &GameRules) {
    let defaults = GameRules::default();
    for (name, value) in rules.iter() {
        let key = name.to_ascii_lowercase();
        if dat.root.get(&key).is_none() && defaults.get(name) == Some((name, value)) {
            continue;
        }
        dat.set(&key, match value {
            RuleValue::Bool(on) => Value::Byte(i8::from(on)),
            RuleValue::Int(n) => Value::Int(n),
        });
    }
}

/// The templates of the folder's `structures` directory, named by their path below it
/// (`structures/house.mcstructure` is `mystructure:house`, `structures/mine/house.mcstructure`
/// `mine:house`, as BDS names a pack's), then those its database keeps.
pub(crate) fn read_structures(world: &LevelDbWorld) -> [BTreeMap<String, Vec<u8>>; 2] {
    let mut folder = BTreeMap::new();
    folder_structures(&world.folder().join("structures"), &mut folder);
    let saved = world.load_structures().unwrap_or_else(|error| {
        tracing::warn!(%error, "the saved structures are unreadable");
        Vec::new()
    });
    [folder, saved.into_iter().collect()]
}

fn folder_structures(root: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let Some(relative) = path.strip_prefix(root).ok().filter(|_| path.extension().is_some_and(|e| e == "mcstructure")) else { continue };
            let parts: Vec<String> = relative.with_extension("").iter().map(|part| part.to_string_lossy().into_owned()).collect();
            let name = match parts.split_first() {
                Some((namespace, rest)) if !rest.is_empty() => format!("{namespace}:{}", rest.join("/")),
                _ => sapling_leveldb::structure_name(&parts.join("/")),
            };
            match std::fs::read(&path) {
                Ok(bytes) => drop(into.insert(name, bytes)),
                Err(error) => tracing::warn!(path = %path.display(), %error, "skipping an unreadable structure"),
            }
        }
    }
}
