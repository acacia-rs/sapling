//! The block entities BDS writes with the structures it generates, field for field as a
//! BDS-generated world holds them (docs/research/structure-block-entities.md).

use acacia_proto::nbt::{List, Str, Value};

const STRING: u8 = 8;

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (Str::from(key), value)).collect())
}

fn empty() -> Value {
    Value::List(List { tag: 0, items: Vec::new() })
}

fn string(text: &str) -> Value {
    Value::String(text.into())
}

fn stack(name: &str, count: i8) -> Value {
    compound(vec![("Count", Value::Byte(count)), ("Damage", Value::Short(0)), ("Name", string(name)), ("WasPickedUp", Value::Byte(0))])
}

/// `fields` between `BlockEntityVersion` and the block's position, in BDS's (sorted) order.
fn entity(id: &str, [x, y, z]: [i32; 3], mut fields: Vec<(&str, Value)>) -> Value {
    fields.push(("BlockEntityVersion", Value::Int(0)));
    fields.extend([("id", string(id)), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z))]);
    fields.sort_by(|a, b| a.0.cmp(b.0));
    compound(fields)
}

/// A monster spawner of `mob` as structures and dungeons leave it.
pub fn generated_spawner(pos: [i32; 3], mob: &str) -> Value {
    let short = |v: i16| Value::Short(v);
    entity("MobSpawner", pos, vec![
        ("Delay", short(20)),
        ("DisplayEntityHeight", Value::Float(1.8)),
        ("DisplayEntityScale", Value::Float(1.0)),
        ("DisplayEntityWidth", Value::Float(0.8)),
        ("EntityIdentifier", string(mob)),
        ("MaxNearbyEntities", short(6)),
        ("MaxSpawnDelay", short(800)),
        ("MinSpawnDelay", short(200)),
        ("RequiredPlayerRange", short(16)),
        ("SpawnCount", short(4)),
        ("SpawnRange", short(4)),
    ])
}

/// The block entity of a block whose loot `table` is rolled with `seed` when first opened or
/// brushed; `block` is its block's name. `None` for a block that keeps no loot.
pub fn generated_loot(pos: [i32; 3], block: &str, table: &str, seed: i32) -> Option<Value> {
    let mut fields = vec![("LootTable", string(table)), ("LootTableSeed", Value::Int(seed))];
    let id = match block {
        "chest" | "trapped_chest" | "barrel" => {
            fields.extend([("Findable", Value::Byte(0)), ("Items", empty())]);
            if block == "barrel" { "Barrel" } else { "Chest" }
        }
        "dispenser" => {
            fields.push(("Items", empty()));
            "Dispenser"
        }
        "suspicious_sand" | "suspicious_gravel" => {
            fields.extend([("brush_count", Value::Int(0)), ("brush_direction", Value::Byte(6)), ("type", string(&format!("minecraft:{block}")))]);
            "BrushableBlock"
        }
        name if name.ends_with("copper_chest") => {
            fields.extend([("Findable", Value::Byte(0)), ("Items", empty())]);
            "Chest"
        }
        _ => return None,
    };
    Some(entity(id, pos, fields))
}

/// A decorated pot with its loot (if any) and sherds (if not plain).
pub fn generated_pot(pos: [i32; 3], table: Option<&str>, seed: i32, sherds: Option<[&str; 4]>) -> Value {
    let mut fields = vec![("animation", Value::Byte(0)), ("item", stack("", 0))];
    if let Some(table) = table {
        fields.extend([("LootTable", string(table)), ("LootTableSeed", Value::Int(seed))]);
    }
    if let Some(sherds) = sherds {
        fields.push(("sherds", Value::List(List { tag: STRING, items: sherds.iter().map(|s| string(s)).collect() })));
    }
    entity("DecoratedPot", pos, fields)
}

/// A trial spawner named after its built-in configs, before it first ticks.
pub fn generated_trial_spawner(pos: [i32; 3], normal: &str, ominous: &str) -> Value {
    entity("TrialSpawner", pos, vec![
        ("cooldown_end_at", Value::Long(0)),
        ("current_mobs", empty()),
        ("next_mob_spawns_at", Value::Long(0)),
        ("normal_config", string(normal)),
        ("ominous_config", string(ominous)),
        ("registered_players", empty()),
        ("required_player_range", Value::Int(14)),
        ("total_mobs_spawned", Value::Int(0)),
    ])
}

/// A vault rewarding `table` for the key item `key`.
// TODO: BDS saves `state_updating_resumes_at` as the level's time at generation; 0 updates at once.
pub fn generated_vault(pos: [i32; 3], table: &str, key: &str) -> Value {
    let config = compound(vec![
        ("activation_range", Value::Float(4.0)),
        ("deactivation_range", Value::Float(4.5)),
        ("key_item", stack(key, 1)),
        ("loot_table", string(table)),
        ("override_loot_table_to_display", string("")),
    ]);
    let data = compound(vec![("items_to_eject", empty()), ("rewarded_players", empty()), ("state_updating_resumes_at", Value::Long(0))]);
    entity("Vault", pos, vec![("config", config), ("data", data)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(value: &Value) -> Vec<String> {
        let Value::Compound(fields) = value else { panic!("a compound") };
        fields.iter().map(|(key, _)| key.to_string()).collect()
    }

    #[test]
    fn generated_entities_keep_bds_fields_in_order() {
        let chest = generated_loot([1, 2, 3], "chest", "loot_tables/chests/simple_dungeon.json", 7).unwrap();
        assert_eq!(keys(&chest), ["BlockEntityVersion", "Findable", "Items", "LootTable", "LootTableSeed", "id", "x", "y", "z"]);
        assert_eq!(chest.get("LootTableSeed"), Some(&Value::Int(7)));
        let gravel = generated_loot([0, 0, 0], "suspicious_gravel", "t", 0).unwrap();
        assert_eq!(gravel.get("id"), Some(&string("BrushableBlock")));
        assert_eq!(gravel.get("type"), Some(&string("minecraft:suspicious_gravel")));
        assert!(generated_loot([0, 0, 0], "stone", "t", 0).is_none());
        let spawner = generated_spawner([0, 0, 0], "minecraft:zombie");
        assert_eq!(spawner.get("EntityIdentifier"), Some(&string("minecraft:zombie")));
        assert_eq!(keys(&spawner)[..2], ["BlockEntityVersion", "Delay"]);
        assert_eq!(keys(&generated_pot([0, 0, 0], None, 0, None)), ["BlockEntityVersion", "animation", "id", "item", "x", "y", "z"]);
    }
}
