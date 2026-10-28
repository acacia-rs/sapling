use super::*;

const SPAWNER: [i32; 3] = [3, 65, 3];
const REWARDS: &[&str] = &["minecraft:trial_key", "minecraft:cooked_chicken", "minecraft:bread", "minecraft:baked_potato", "minecraft:potion"];

/// Alex in survival over a lit floor, so only the spawner's zombies come, by a zombie trial
/// spawner waiting for players.
fn chamber() -> Harness {
    let mut h = alex_on(GameMode::Survival, "minecraft:glowstone", true);
    let entity = compound([("id", Value::String("TrialSpawner".into())), ("normal_config", Value::String("minecraft:trial_chamber/melee/zombie/normal".into()))]);
    put(&mut h, Block::new("minecraft:trial_spawner").with("ominous=0,trial_spawner_state=1"), SPAWNER, entity);
    h
}

fn state(h: &mut Harness) -> String {
    block(h, SPAWNER).properties.split(',').find_map(|pair| pair.strip_prefix("trial_spawner_state=")).unwrap_or_default().to_owned()
}

#[test]
fn a_wave_comes_for_the_player_three_at_once_and_six_in_all() {
    let mut h = chamber();
    run(&mut h, 25);
    assert_eq!(state(&mut h), "2", "the player was taken on within a scan");
    run(&mut h, 120);
    let zombies = of_kind(&mut h, "minecraft:zombie");
    assert_eq!(zombies.len(), 3, "three at once for one player: {zombies:?}");
    assert_eq!(int(&entity_at(&mut h, SPAWNER), "total_mobs_spawned"), 3);
}

#[test]
fn a_wave_beaten_ejects_a_reward_and_cools_down() {
    let mut h = chamber();
    let mut killed = 0;
    for _ in 0..600 {
        killed += ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:zombie").filter(|zombie| ctx.kill_entity(zombie.id)).count());
        if state(&mut h) == "5" {
            break;
        }
    }
    assert_eq!((killed, state(&mut h)), (6, "5".to_owned()), "six zombies, then the reward and the cooldown");
    let rewards = ask(&mut h, |ctx| ctx.entities().into_iter().filter_map(|entity| entity.item).filter(|item| REWARDS.contains(&item.name.as_str())).count());
    assert!(rewards >= 1, "one roll of the key or the consumables");
    let entity = entity_at(&mut h, SPAWNER);
    assert!(int(&entity, "cooldown_end_at") > 30_000);
    assert!(matches!(entity.get("selected_loot_table"), Some(Value::String(table)) if table.starts_with("loot_tables/spawners/trial_chamber/")));
}

#[test]
fn creative_players_and_a_spawner_without_a_mob_wake_nothing() {
    let mut h = chamber();
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    run(&mut h, 60);
    assert_eq!((state(&mut h), of_kind(&mut h, "minecraft:zombie").len()), ("1".to_owned(), 0));
    put(&mut h, Block::new("minecraft:trial_spawner"), [-3, 65, 3], compound([("id", Value::String("TrialSpawner".into()))]));
    run(&mut h, 5);
    assert_eq!(block(&mut h, [-3, 65, 3]).properties.contains("trial_spawner_state=0"), true, "no mob, so inactive");
}

#[test]
fn a_spawn_egg_gives_a_bare_spawner_its_mob() {
    let mut h = alex_on(GameMode::Creative, "minecraft:glowstone", true);
    put(&mut h, Block::new("minecraft:trial_spawner"), SPAWNER, compound([("id", Value::String("TrialSpawner".into()))]));
    held(&mut h, "minecraft:husk_spawn_egg", 1);
    h.receive(1, &click(SPAWNER, 1, None));
    run(&mut h, 3);
    assert_eq!(state(&mut h), "1", "waiting for players now");
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1), "kept in creative");
}

#[test]
fn bad_omen_turns_to_trial_omen_and_the_spawner_ominous() {
    let mut h = chamber();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, sapling_api::Effect::BadOmen, 6000, 1, true)));
    let sent: Vec<_> = (0..25).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let told: Vec<_> = common::all::<acacia_proto::packets::MobEffect>(&sent).into_iter().map(|effect| (effect.effect_id, effect.duration)).collect();
    assert_eq!(told.iter().filter(|(id, _)| *id != 28).collect::<Vec<_>>(), [&(31, 36000)], "trial omen for 18000 a level: {told:?}");
    assert!(block(&mut h, SPAWNER).properties.contains("ominous=1"), "{:?}", block(&mut h, SPAWNER));
}
