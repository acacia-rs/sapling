use super::*;

const VAULT: [i32; 3] = [2, 65, 0];

/// Alex in survival beside an inactive vault.
fn beside_a_vault() -> Harness {
    let mut h = alex_on(GameMode::Survival, "minecraft:glowstone", false);
    put(&mut h, Block::new("minecraft:vault").with("vault_state=inactive"), VAULT, compound([("id", Value::String("Vault".into()))]));
    h
}

fn state(h: &mut Harness) -> String {
    block(h, VAULT).properties.split(',').find_map(|pair| pair.strip_prefix("vault_state=")).unwrap_or_default().to_owned()
}

fn rewarded(entity: &Value) -> Vec<Value> {
    match entity.get("data").and_then(|data| data.get("rewarded_players")) {
        Some(Value::List(list)) => list.items.clone(),
        _ => Vec::new(),
    }
}

#[test]
fn a_key_unlocks_the_vault_once_per_player_and_it_ejects_its_reward() {
    let mut h = beside_a_vault();
    run(&mut h, 25);
    assert_eq!(state(&mut h), "active", "a player within 4 not yet rewarded");
    assert!(entity_at(&mut h, VAULT).get("data").and_then(|data| data.get("display_item")).is_some(), "it shows what it holds");
    held(&mut h, "minecraft:trial_key", 2);
    h.receive(1, &click(VAULT, 1, None));
    let sent = h.tick_all();
    assert_eq!(state(&mut h), "unlocking");
    assert!(sounds(&sent).contains(&"vault.insert_item".to_owned()), "{:?}", sounds(&sent));
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1));
    assert_eq!(rewarded(&entity_at(&mut h, VAULT)).len(), 1);
    let mut states = Vec::new();
    for _ in 0..400 {
        h.tick_all();
        let now = state(&mut h);
        if states.last() != Some(&now) {
            states.push(now);
        }
    }
    assert_eq!(states, ["unlocking", "ejecting", "inactive"], "it ejects, then sleeps: its one player was rewarded");
    let lying = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.is_some() && (entity.feet[0] - 2.5).abs() < 3.0).count());
    assert!(lying >= 1, "the reward lies about the vault");
    // Rewarded, Alex no longer wakes it, and a second key is not taken.
    h.receive(1, &click(VAULT, 1, None));
    run(&mut h, 30);
    assert_eq!(state(&mut h), "inactive");
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1));
}

#[test]
fn the_wrong_item_is_refused_aloud_and_kept() {
    let mut h = beside_a_vault();
    run(&mut h, 25);
    held(&mut h, "minecraft:ominous_trial_key", 1);
    h.receive(1, &click(VAULT, 1, None));
    let sent = h.tick_all();
    assert!(sounds(&sent).contains(&"vault.insert_item_fail".to_owned()), "{:?}", sounds(&sent));
    assert_eq!((state(&mut h), ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count))), ("active".to_owned(), Some(1)));
}

#[test]
fn a_vault_sleeps_when_no_one_is_near() {
    let mut h = beside_a_vault();
    run(&mut h, 25);
    h.remote.run(|ctx| ctx.teleport(ALEX, [12.5, 65.0, 0.5]));
    run(&mut h, 45);
    assert_eq!(state(&mut h), "inactive");
}
