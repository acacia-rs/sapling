//! The dragon fight: its start, a kill, a summoning and a fight kept with its world, as BDS ran
//! them (research/captures/bds-dragon-*.txt).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{AddEntity, BossEvent, BossEventType, ChangeDimension, MovePlayer, MovePlayerMode, PlayerAction, PlayerAuthInput, Respawn, ShowCredits};
use acacia_proto::types::{Action, BlockCoordinates, InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, click, input_at};
use sapling_api::{DragonFight, EntityInfo, Item, PlayerId, Slot};
use sapling_world::{END, World};

const ALEX: PlayerId = PlayerId(1);
const THE_END: &str = "default:the_end";
const DRAGON: &str = "minecraft:ender_dragon";

/// A void End with an end stone island up to y 61 round the origin.
fn island() -> World {
    let mut end = World::void_in(END);
    let end_stone = end.block_id("minecraft:end_stone").unwrap();
    end.fill([-12, 50, -12], [12, 61, 12], end_stone);
    end
}

/// Alex on the island beside the pillar, past the dimension change; what was sent meanwhile.
fn into_end(h: &mut Harness) -> Vec<acacia_proto::RawPacket> {
    assert!(h.shard.add_dimension("default", island()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [8.5, 62.0, 0.5])));
    let mut sent = ticks(h, 5);
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    h.receive(1, &PlayerAction { runtime_entity_id: 1, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 });
    sent.extend(ticks(h, 5));
    sent
}

fn ticks(h: &mut Harness, n: usize) -> Vec<acacia_proto::RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn fight(h: &mut Harness) -> Option<DragonFight> {
    let seen = Arc::new(Mutex::new(None));
    let kept = seen.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.dragon_fight());
    h.tick_all();
    seen.lock().unwrap().clone()
}

fn dragon(h: &mut Harness) -> Option<EntityInfo> {
    let seen = Arc::new(Mutex::new(None));
    let kept = seen.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.within(THE_END, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == DRAGON)).flatten());
    h.tick_all();
    seen.lock().unwrap().clone()
}

fn block(h: &mut Harness, at: [i32; 3]) -> String {
    let seen = Arc::new(Mutex::new(String::new()));
    let kept = seen.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.within(THE_END, |ctx| ctx.block(at)).flatten().map_or("?".into(), |block| block.name));
    h.tick_all();
    let name = seen.lock().unwrap().clone();
    name.trim_start_matches("minecraft:").to_owned()
}

fn experience(sent: &[acacia_proto::RawPacket]) -> i32 {
    let worth = |orb: &AddEntity| orb.metadata.iter().find(|item| item.key == Key::ExperienceValue).and_then(|item| if let Value::Default(Plain::Int(value)) = item.value { Some(value) } else { None });
    all::<AddEntity>(sent).iter().filter(|added| added.entity_type == "minecraft:xp_orb").filter_map(worth).sum()
}

fn kill(h: &mut Harness) -> Vec<acacia_proto::RawPacket> {
    let id = dragon(h).expect("a dragon").id;
    h.remote.run(move |ctx| assert_eq!(ctx.within(THE_END, |ctx| ctx.kill_entity(id)), Some(true)));
    ticks(h, 205)
}

#[test]
fn a_player_near_the_island_starts_the_fight_as_on_bds() {
    let mut h = Harness::new();
    let sent = into_end(&mut h);
    let added: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == DRAGON).collect();
    assert_eq!(added.len(), 1);
    assert_eq!([added[0].position.x, added[0].position.y, added[0].position.z], [0.0, 128.0, 0.0]);
    let bars: Vec<(BossEventType, f32, String)> = all::<BossEvent>(&sent).into_iter().map(|bar| (bar.r#type, bar.progress, bar.title)).collect();
    assert!(!bars.is_empty() && bars.iter().all(|bar| *bar == (BossEventType::ShowBar, 1.0, "entity.ender_dragon.name".into())), "{bars:?}");
    assert_eq!([block(&mut h, [0, 61, 0]), block(&mut h, [0, 65, 0]), block(&mut h, [1, 62, 0]), block(&mut h, [1, 64, 0]), block(&mut h, [3, 62, 0])], ["bedrock", "bedrock", "air", "torch", "bedrock"]);
    let fight = fight(&mut h).expect("a fight");
    assert_eq!((fight.health, fight.crystals, fight.previously_killed), (Some(200.0), 0, false));
    assert_eq!(fight.dragon, dragon(&mut h).map(|dragon| dragon.id));
}

#[test]
fn the_first_kill_opens_the_portal_lays_the_egg_and_opens_a_gateway() {
    let mut h = Harness::new();
    into_end(&mut h);
    let sent = kill(&mut h);
    assert_eq!(experience(&sent), 12_000, "BDS's first dragon");
    assert!(dragon(&mut h).is_none());
    assert_eq!([block(&mut h, [1, 62, 0]), block(&mut h, [0, 62, 0]), block(&mut h, [0, 66, 0])], ["end_portal", "bedrock", "dragon_egg"]);
    let fight = fight(&mut h).expect("a fight");
    assert_eq!((fight.dragon, fight.previously_killed), (None, true));
    let gateways = (0..20).filter(|&i| {
        let angle = (i as f32 * 0.157_079_64 + -std::f32::consts::PI) * 2.0;
        block(&mut h, [(angle.cos() * 96.0) as i32, 75, (angle.sin() * 96.0) as i32]) == "end_gateway"
    });
    assert_eq!(gateways.count(), 1);
}

#[test]
fn four_crystals_on_the_portal_summon_a_dragon_worth_500() {
    let mut h = Harness::new();
    into_end(&mut h);
    kill(&mut h);
    for [x, z] in [[0, 3], [-3, 0], [0, -3], [3, 0]] {
        let feet = [x as f32 * 2.0 + 0.5, 62.0, z as f32 * 2.0 + 0.5];
        h.remote.run(move |ctx| {
            ctx.teleport(ALEX, feet);
            assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:end_crystal", 1))));
        });
        ticks(&mut h, 2);
        h.receive(1, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..input_at(feet) });
        ticks(&mut h, 1);
        h.receive(1, &click([x, 62, z], 1, None));
        ticks(&mut h, 1);
    }
    assert_eq!(block(&mut h, [1, 62, 0]), "air", "the portal goes out as the summoning begins");
    let sent = ticks(&mut h, 610);
    assert_eq!(all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == DRAGON).count(), 1);
    let fight = fight(&mut h).expect("a fight");
    assert_eq!((fight.health, fight.crystals), (Some(200.0), 10));
    let sent = kill(&mut h);
    assert_eq!(experience(&sent), 500);
    assert_eq!(block(&mut h, [0, 66, 0]), "dragon_egg", "no second egg, the first still there");
}

/// Alex steps into the lit portal; what was sent until its trip home has begun.
fn into_portal(h: &mut Harness) -> Vec<acacia_proto::RawPacket> {
    h.remote.run(|ctx| ctx.teleport(ALEX, [1.5, 62.0, 0.5]));
    ticks(h, 2)
}

fn ack(h: &mut Harness) {
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    h.receive(1, &PlayerAction { runtime_entity_id: 1, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 });
}

#[test]
fn the_exit_portal_shows_the_credits_once_then_sends_a_player_home() {
    let mut h = Harness::new();
    into_end(&mut h);
    kill(&mut h);
    let sent = into_portal(&mut h);
    assert_eq!(all::<ShowCredits>(&sent).iter().map(|shown| shown.status).collect::<Vec<_>>(), [0]);
    assert!(all::<ChangeDimension>(&sent).is_empty(), "not before the credits are over");
    h.receive(1, &ShowCredits { runtime_entity_id: 1, status: 1 });
    let sent = ticks(&mut h, 3);
    let change = all::<ChangeDimension>(&sent).remove(0);
    assert_eq!((change.dimension, change.position.y, change.respawn, change.loading_screen_id), (0, 32767.0, true, Some(1)), "research/captures/bds-dragon-home.txt");
    assert_eq!(all::<Respawn>(&sent).iter().map(|respawn| respawn.state).collect::<Vec<_>>(), [0]);
    ack(&mut h);
    let sent = ticks(&mut h, 2);
    let risen: Vec<(u8, f32)> = all::<Respawn>(&sent).iter().map(|respawn| (respawn.state, respawn.position.y)).collect();
    assert_eq!(risen, [(0, 32767.0), (1, 65.0 + sapling_data::EYE_HEIGHT)]);
    assert!(all::<MovePlayer>(&sent).iter().any(|moved| moved.mode == MovePlayerMode::Reset));
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [8.5, 62.0, 0.5])));
    ticks(&mut h, 5);
    ack(&mut h);
    ticks(&mut h, 5);
    let sent = into_portal(&mut h);
    assert!(all::<ShowCredits>(&sent).is_empty(), "seen once");
    assert_eq!(all::<ChangeDimension>(&sent).len(), 1);
}

#[test]
fn the_dragon_circles_lands_on_the_portal_and_takes_off_again() {
    let mut h = Harness::new();
    into_end(&mut h);
    // A creative player is no prey, as in research/captures/bds-dragon-watch.txt.
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative));
    let start = dragon(&mut h).expect("a dragon").feet;
    ticks(&mut h, 20);
    let flown = dragon(&mut h).expect("a dragon").feet;
    assert!(flown[1] < start[1] - 2.0, "it dives from its spawn: {flown:?}");
    let sat = (0..4000).find_map(|_| {
        h.tick_all();
        dragon(&mut h).filter(|dragon| dragon.feet[0].hypot(dragon.feet[2]) < 1.0 && (dragon.feet[1] - 66.0).abs() < 1.5)
    });
    assert!(sat.is_some(), "it landed on the pillar's top");
    let left = (0..400).find_map(|_| {
        h.tick_all();
        dragon(&mut h).filter(|dragon| dragon.feet[1] > 70.0)
    });
    assert!(left.is_some(), "and took off");
}

#[test]
fn a_kept_fight_whose_dragon_was_killed_spawns_none() {
    let mut level = common::platform();
    level.level_mut().dragon_fight = Some(sapling_world::DragonFightRecord { killed: true, previously_killed: true, spawned: true, dragon: -1, exit_portal: Some([0, 62, 0]), gateways: vec![1, 2], respawning: false });
    let mut h = Harness::with(common::config(), level);
    into_end(&mut h);
    assert!(dragon(&mut h).is_none());
    assert_eq!(fight(&mut h).map(|fight| fight.previously_killed), Some(true));
}
