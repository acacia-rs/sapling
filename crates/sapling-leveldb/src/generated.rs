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

/// Either half of a bed of colour `color` (14 red).
pub fn generated_bed(pos: [i32; 3], color: u8) -> Value {
    entity("Bed", pos, vec![("color", Value::Byte(color as i8))])
}

/// A bell no one has rung, never ticked: BDS's `Ticks` counts the ticks it has been loaded
/// (the same two bells saved 24/33 in one oracle run, 21/21 in another).
pub fn generated_bell(pos: [i32; 3]) -> Value {
    entity("Bell", pos, vec![("Direction", Value::Int(0)), ("Ringing", Value::Byte(0)), ("Ticks", Value::Int(0))])
}

/// A jigsaw `/place` kept (`keepJigsaws`) as BDS showed it (research/captures/bds-k1jig.txt):
/// `[name, target, pool, joint, final_state]` and `[selection, placement]` priorities.
pub fn generated_jigsaw(pos: [i32; 3], [name, target, pool, joint, final_state]: [&str; 5], [selection, placement]: [i32; 2]) -> Value {
    let strings = [("final_state", final_state), ("joint", joint), ("name", name), ("target", target), ("target_pool", pool)];
    let mut fields: Vec<_> = strings.into_iter().map(|(key, text)| (key, string(text))).collect();
    fields.extend([("placement_priority", Value::Int(placement)), ("selection_priority", Value::Int(selection))]);
    entity("JigsawBlock", pos, fields)
}

/// A cauldron of water or empty with the potion `potion` in it (-1 none); generation leaves its
/// `PotionType` -1 even with a potion (`SwamplandHut::postProcess` 14836de70 sets the id alone).
pub fn generated_cauldron(pos: [i32; 3], potion: i16) -> Value {
    entity("Cauldron", pos, vec![("Items", empty()), ("PotionId", Value::Short(potion)), ("PotionType", Value::Short(-1))])
}

/// The plant `plant` in its first state, as BDS saved a template's cactus (`age` 0) and a potted
/// sapling (research/captures/bds-potsapling.txt).
fn plant_block(plant: &str) -> Value {
    let states = match plant {
        "minecraft:cactus" => vec![("age", Value::Int(0))],
        sapling if sapling.ends_with("_sapling") => vec![("age_bit", Value::Byte(0))],
        _ => Vec::new(),
    };
    compound(vec![("name", string(plant)), ("states", compound(states)), ("version", Value::Int(crate::states::STATE_VERSION))])
}

/// A flower pot holding the plant `plant` (`minecraft:red_mushroom`), or nothing when it is empty.
pub fn generated_flower_pot(pos: [i32; 3], plant: &str) -> Value {
    entity("FlowerPot", pos, if plant.is_empty() { Vec::new() } else { vec![("PlantBlock", plant_block(plant))] })
}

/// An end gateway leading over `exit`, as BDS's generation leaves one (oracle 20261003-end, which had
/// aged it 23 ticks): `EndGatewayBadPosChecked` 1.
pub fn generated_end_gateway(pos: [i32; 3], exit: [i32; 3]) -> Value {
    let exit = Value::List(List { tag: 3, items: exit.map(Value::Int).into() });
    entity("EndGateway", pos, vec![("Age", Value::Int(0)), ("EndGatewayBadPosChecked", Value::Byte(1)), ("ExitPortal", exit)])
}

/// A chest with no loot holding `items` (sapling-worldgen `BlockEntity::Chest`'s list), as BDS
/// saved a mansion's: `Findable` 0, each item with its slot and, a plant, its block.
pub fn generated_chest(pos: [i32; 3], items: &str) -> Value {
    let item = |spec: &str| {
        let words: Vec<&str> = spec.split(' ').collect();
        let [slot, count, name, more @ ..] = words.as_slice() else { panic!("a chest item: {spec}") };
        let mut fields = vec![("Count", Value::Byte(count.parse().expect("a count"))), ("Damage", Value::Short(0)), ("Name", string(name)), ("Slot", Value::Byte(slot.parse().expect("a slot"))), ("WasPickedUp", Value::Byte(0))];
        if more.contains(&"block") {
            fields.insert(0, ("Block", plant_block(name)));
        }
        let enchants: Vec<Value> = more.iter().filter_map(|word| word.strip_prefix("ench=")?.split_once(':')).map(|(id, level)| compound(vec![("id", Value::Short(id.parse().expect("an id"))), ("lvl", Value::Short(level.parse().expect("a level")))])).collect();
        if !enchants.is_empty() {
            fields.push(("tag", compound(vec![("ench", Value::List(List { tag: 10, items: enchants }))])));
        }
        compound(fields)
    };
    let stacks: Vec<Value> = items.split(';').filter(|spec| !spec.is_empty()).map(item).collect();
    let items = if stacks.is_empty() { empty() } else { Value::List(List { tag: 10, items: stacks }) };
    entity("Chest", pos, vec![("Findable", Value::Byte(0)), ("Items", items)])
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
        assert_eq!(keys(&generated_bed([0, 0, 0], 14)), ["BlockEntityVersion", "color", "id", "x", "y", "z"]);
        assert_eq!(keys(&generated_bell([0, 0, 0]))[..4], ["BlockEntityVersion", "Direction", "Ringing", "Ticks"]);
        assert_eq!(keys(&generated_cauldron([0, 0, 0], -1))[..4], ["BlockEntityVersion", "Items", "PotionId", "PotionType"]);
        assert_eq!(keys(&generated_flower_pot([0, 0, 0], "minecraft:red_mushroom")), ["BlockEntityVersion", "PlantBlock", "id", "x", "y", "z"]);
        assert_eq!(keys(&generated_end_gateway([0, 0, 0], [100, 50, 0])), ["Age", "BlockEntityVersion", "EndGatewayBadPosChecked", "ExitPortal", "id", "x", "y", "z"]);
        let jigsaw = generated_jigsaw([0, 0, 0], ["minecraft:street", "minecraft:street", "minecraft:village/plains/streets", "rollable", "minecraft:air"], [0, 0]);
        let fields = ["BlockEntityVersion", "final_state", "id", "joint", "name", "placement_priority", "selection_priority", "target", "target_pool", "x", "y", "z"];
        assert_eq!(keys(&jigsaw), fields);
        let cactus = generated_flower_pot([0, 0, 0], "minecraft:cactus");
        assert_eq!(cactus.get("PlantBlock").and_then(|plant| plant.get("states")).and_then(|states| states.get("age")), Some(&Value::Int(0)));
    }

    #[test]
    fn a_mansion_chest_keeps_its_items_as_bds_saved_them() {
        let empty_chest = generated_chest([0, 0, 0], "");
        assert_eq!(keys(&empty_chest), ["BlockEntityVersion", "Findable", "Items", "id", "x", "y", "z"]);
        assert_eq!(empty_chest.get("Items"), Some(&empty()));
        let chest = generated_chest([0, 0, 0], "8 1 minecraft:allium block;10 1 minecraft:iron_axe ench=15:1");
        let Some(Value::List(items)) = chest.get("Items") else { panic!("items") };
        assert_eq!(keys(&items.items[0]), ["Block", "Count", "Damage", "Name", "Slot", "WasPickedUp"]);
        assert_eq!(items.items[0].get("Block").and_then(|block| block.get("name")), Some(&string("minecraft:allium")));
        assert_eq!((items.items[1].get("Slot"), keys(&items.items[1]).last().map(String::as_str)), (Some(&Value::Byte(10)), Some("tag")));
    }
}
