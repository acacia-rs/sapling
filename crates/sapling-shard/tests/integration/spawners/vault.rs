use super::*;

use acacia_proto::RawPacket;
use acacia_proto::packets::{BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateSubchunkBlocks};

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
fn an_active_vault_shows_its_players_and_item_with_its_block_each_second_and_unlocks_heard_after() {
    // research/captures/bds-x2ran.txt: block and entity (`connected_players`, `display_item`) each second;
    // on a key, the block, its entity, then `vault.insert_item` at the cell's corner.
    use acacia_proto::RawPacket;
    use acacia_proto::packets::{BlockEntityData, LevelSoundEvent};
    let mut h = beside_a_vault();
    run(&mut h, 25);
    let sent: Vec<RawPacket> = (0..21).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let shown: Vec<Value> = crate::common::all::<BlockEntityData>(&sent).into_iter().map(|data| data.nbt.decode().unwrap().value).collect();
    let players = |entity: &Value| matches!(entity.get("connected_players"), Some(Value::List(list)) if !list.items.is_empty());
    let item = |entity: &Value| entity.get("display_item").and_then(|item| item.get("Name")).is_some_and(|name| *name != Value::String("".into()));
    assert!(!shown.is_empty() && shown.iter().all(|entity| players(entity) && item(entity)), "{shown:?}");
    assert!(!crate::common::block_changes(&sent).is_empty(), "its block shown again");
    held(&mut h, "minecraft:trial_key", 2);
    h.receive(1, &click(VAULT, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let at = |found: &dyn Fn(&[RawPacket]) -> bool| sent.iter().position(|packet| found(std::slice::from_ref(packet)));
    let (block, entity) = (at(&|one| !crate::common::block_changes(one).is_empty()), at(&|one| !crate::common::all::<BlockEntityData>(one).is_empty()));
    let sound = crate::common::all::<LevelSoundEvent>(&sent).into_iter().find(|sound| sound.sound_id == "vault.insert_item").expect("heard");
    let heard = at(&|one| crate::common::all::<LevelSoundEvent>(one).iter().any(|sound| sound.sound_id == "vault.insert_item"));
    assert!(block.is_some() && block < entity && entity < heard, "{block:?} {entity:?} {heard:?}");
    assert_eq!([sound.position.x, sound.position.y, sound.position.z], VAULT.map(|c| c as f32));
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

/// research/captures/bds-placing2.txt: BDS woke a vault put down by hand beside its placer.
#[test]
fn a_vault_put_down_by_hand_wakes_for_its_placer() {
    let mut h = alex_on(GameMode::Survival, "minecraft:glowstone", false);
    held(&mut h, "minecraft:vault", 1);
    h.receive(1, &click([VAULT[0], VAULT[1] - 1, VAULT[2]], 1, None));
    run(&mut h, 25);
    assert_eq!(state(&mut h), "active");
}

/// Alex's packets in the tick `event` is told and in the next.
fn around(h: &mut Harness, event: LevelEventEvent) -> (Vec<RawPacket>, Vec<RawPacket>) {
    for _ in 0..400 {
        let sent = h.tick_all().to(1).to_vec();
        if common::all::<LevelEvent>(&sent).iter().any(|told| told.event == event) {
            return (sent, h.tick_all().to(1).to_vec());
        }
    }
    panic!("no {event:?}");
}

fn shown_at_vault(sent: &[RawPacket]) -> Vec<u32> {
    common::block_changes(sent).into_iter().filter(|(at, _)| *at == VAULT).map(|(_, id)| id).collect()
}

fn hash(h: &mut Harness) -> u32 {
    let now = block(h, VAULT);
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(&now.name, &now.properties).expect("a state"))
}

/// The vault's block listed twice in one UpdateSubchunkBlocks, then its entity twice; where the last entity went.
fn listed_twice_then_entity_twice(sent: &[RawPacket], id: u32) -> usize {
    let one = std::slice::from_ref::<RawPacket>;
    let listing = sent.iter().position(|packet| !common::all::<UpdateSubchunkBlocks>(one(packet)).is_empty()).expect("listed");
    assert_eq!(shown_at_vault(one(&sent[listing])), [id, id]);
    assert_eq!(shown_at_vault(sent), [id, id], "listed only there");
    let at_vault = |packet: &RawPacket| common::all::<BlockEntityData>(one(packet)).iter().any(|data| [data.position.x, data.position.y, data.position.z] == VAULT);
    let entities: Vec<usize> = sent.iter().enumerate().filter(|(_, packet)| at_vault(packet)).map(|(i, _)| i).collect();
    assert!(entities.len() == 2 && entities.iter().all(|&i| i > listing), "{entities:?} after {listing}");
    entities[1]
}

#[test]
fn a_vault_going_active_is_told_at_once_and_shown_the_next_tick() {
    // research/captures/bds-x2ran.txt: AnimationVaultActivate, then a tick on its block listed twice and its entity twice.
    let mut h = beside_a_vault();
    let (told, next) = around(&mut h, LevelEventEvent::AnimationVaultActivate);
    let active = hash(&mut h);
    assert_eq!(state(&mut h), "active");
    assert!(!shown_at_vault(&told).contains(&active), "not shown active with its animation");
    listed_twice_then_entity_twice(&next, active);
}

#[test]
fn an_unlocked_vault_is_shown_again_the_next_tick() {
    // research/captures/bds-x2ran.txt: the key's tick shows block, entity and insert_item; the next its block and entity again.
    let mut h = beside_a_vault();
    run(&mut h, 25);
    held(&mut h, "minecraft:trial_key", 2);
    h.receive(1, &click(VAULT, 1, None));
    h.tick_all();
    let next = h.tick_all().to(1).to_vec();
    let unlocking = hash(&mut h);
    assert_eq!(state(&mut h), "unlocking");
    assert_eq!(shown_at_vault(&next), [unlocking]);
    assert!(common::all::<BlockEntityData>(&next).iter().any(|data| [data.position.x, data.position.y, data.position.z] == VAULT));
}

#[test]
fn an_emptied_vault_closes_its_shutter_a_tick_after_its_animation() {
    // research/captures/bds-x2ran.txt: ejecting to inactive told by AnimationVaultDeactivate; a tick on the block
    // listed twice, the entity twice, then vault.close_shutter.
    let mut h = beside_a_vault();
    run(&mut h, 25);
    held(&mut h, "minecraft:trial_key", 2);
    h.receive(1, &click(VAULT, 1, None));
    let (told, next) = around(&mut h, LevelEventEvent::AnimationVaultDeactivate);
    let inactive = hash(&mut h);
    assert_eq!(state(&mut h), "inactive");
    let heard = |sent: &[RawPacket]| sent.iter().position(|packet| common::all::<LevelSoundEvent>(std::slice::from_ref(packet)).iter().any(|sound| sound.sound_id == "vault.close_shutter"));
    assert_eq!(heard(&told), None, "not with its animation");
    let shown = listed_twice_then_entity_twice(&next, inactive);
    assert!(heard(&next).is_some_and(|at| at > shown), "{:?} after {shown}", heard(&next));
}

#[test]
fn a_vault_sleeps_when_no_one_is_near() {
    let mut h = beside_a_vault();
    run(&mut h, 25);
    h.remote.run(|ctx| ctx.teleport(ALEX, [12.5, 65.0, 0.5]));
    run(&mut h, 45);
    assert_eq!(state(&mut h), "inactive");
}
