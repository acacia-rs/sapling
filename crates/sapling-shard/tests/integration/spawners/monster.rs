use super::*;

const SPAWNER: [i32; 3] = [3, 65, 3];

fn spawner(fields: Vec<(&'static str, Value)>) -> Value {
    let mut entity = vec![("id", Value::String("MobSpawner".into())), ("EntityIdentifier", Value::String("minecraft:zombie".into()))];
    entity.extend(fields);
    Value::Compound(entity.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

#[test]
fn a_spawner_spawns_its_kind_near_a_player_and_waits_again() {
    let mut h = alex(GameMode::Creative);
    put(&mut h, Block::new("minecraft:mob_spawner"),SPAWNER, spawner(vec![("Delay", Value::Short(0))]));
    run(&mut h, 3);
    let zombies = of_kind(&mut h, "minecraft:zombie");
    assert!((1..=4).contains(&zombies.len()), "{zombies:?}");
    for zombie in &zombies {
        let off = (zombie.feet[0] - 3.5).abs().floor() + (zombie.feet[2] - 3.5).abs().floor() + (zombie.feet[1] - 65.0).abs();
        assert!(off <= 4.0, "{zombie:?}");
    }
    let delay = int(&entity_at(&mut h, SPAWNER), "Delay");
    assert!((195..800).contains(&delay), "{delay}");
}

#[test]
fn no_more_than_max_nearby_entities_of_its_kind_come() {
    let mut h = alex(GameMode::Creative);
    put(&mut h, Block::new("minecraft:mob_spawner"),SPAWNER, spawner(vec![("Delay", Value::Short(0)), ("MinSpawnDelay", Value::Short(1)), ("MaxSpawnDelay", Value::Short(2)), ("SpawnCount", Value::Short(4)), ("MaxNearbyEntities", Value::Short(2))]));
    run(&mut h, 40);
    assert_eq!(of_kind(&mut h, "minecraft:zombie").len(), 2);
}

#[test]
fn a_spawner_waits_while_no_player_is_in_range_and_light_keeps_monsters_away() {
    let mut h = alex(GameMode::Creative);
    put(&mut h, Block::new("minecraft:mob_spawner"),[20, 65, 3], spawner(vec![("Delay", Value::Short(5))]));
    run(&mut h, 10);
    assert_eq!(int(&entity_at(&mut h, [20, 65, 3]), "Delay"), 5, "no player within 16");
    // Glowstone about it: block light wherever the zombies would stand.
    h.remote.run(|ctx| {
        for (x, z) in [(1, 1), (5, 1), (1, 5), (5, 5)] {
            assert!(ctx.set_block([x, 65, z], &Block::new("minecraft:glowstone")));
        }
    });
    put(&mut h, Block::new("minecraft:mob_spawner"),SPAWNER, spawner(vec![("Delay", Value::Short(0))]));
    run(&mut h, 5);
    assert_eq!(of_kind(&mut h, "minecraft:zombie"), [], "lit");
    assert_eq!(int(&entity_at(&mut h, SPAWNER), "Delay"), 0, "a spawn that came to nothing tries again");
}

#[test]
fn a_spawn_egg_sets_the_kind_and_is_spent_in_survival() {
    let mut h = alex(GameMode::Survival);
    h.remote.run(|ctx| assert!(ctx.set_block(SPAWNER, &Block::new("minecraft:mob_spawner"))));
    held(&mut h, "minecraft:skeleton_spawn_egg", 2);
    h.receive(1, &click(SPAWNER, 1, None));
    h.tick_all();
    let entity = entity_at(&mut h, SPAWNER);
    assert_eq!(entity.get("EntityIdentifier"), Some(&Value::String("minecraft:skeleton".into())));
    assert_eq!(int(&entity, "MaxSpawnDelay"), 800);
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1));
    assert_eq!(of_kind(&mut h, "minecraft:skeleton"), [], "nothing hatched");
}
