//! A player's standing with a village from a shot, kept with the world as BDS's `PLAYERS` record
//! (villagers/standing.rs; research/captures/bds-y3ranged.txt).

use crate::common;

use acacia_proto::nbt::{self, LittleEndian, Value};
use common::shooting::{ALEX, draw, face};
use sapling_api::{Block, BlockEntityData, GameMode, Item, Slot};

const BELL: [i32; 3] = [3, 65, 3];
/// The archer's `UniqueID` in its record: bds-f14nvillage.txt's actor's.
const ARCHER: i64 = -4_294_967_295;

/// The players kept in the level's `PLAYERS` records.
fn kept_players(storage: &common::kept::Kept) -> Vec<Value> {
    let level = storage.level().expect("flushed");
    let records = level.villages.iter().filter(|record| record.part == sapling_world::VillagePart::Players);
    let players = records.filter_map(|record| match nbt::read::<LittleEndian>(&mut &record.compound[..]).ok()?.value.get("Players") {
        Some(Value::List(list)) => Some(list.items.clone()),
        _ => None,
    });
    players.flatten().collect()
}

fn standings(storage: &common::kept::Kept) -> Vec<i32> {
    kept_players(storage).iter().filter_map(|player| if let Some(Value::Int(standing)) = player.get("S") { Some(*standing) } else { None }).collect()
}

/// A bell, a held villager 3 before Alex and Alex with a bow, shooting once; the world flushed.
fn shot_once() -> (std::sync::Arc<common::kept::Kept>, common::Harness) {
    let columns = common::stored::floor_holding(Vec::new(), &[]);
    let record = sapling_world::PlayerData { unique_id: Some(ARCHER), ..sapling_world::PlayerData::default() };
    sapling_world::WorldStorage::store_player(&*columns, &acacia_auth::login::offline_identity("Steve"), &record).unwrap();
    let storage = common::kept::Kept::new(columns);
    let mut h = common::kept::opened(&storage);
    common::stored::enter_stored(&mut h, 1);
    let bell = Value::Compound(vec![("id".into(), Value::String("Bell".into())), ("x".into(), Value::Int(BELL[0])), ("y".into(), Value::Int(BELL[1])), ("z".into(), Value::Int(BELL[2]))]);
    h.remote.run(move |ctx| {
        for x in -3..6 {
            for z in -3..9 {
                assert!(ctx.set_block([x, 64, z], &Block::new("stone")));
            }
        }
        assert!(ctx.set_block(BELL, &Block::new("bell")));
        assert!(ctx.set_block_entity(BELL, &BlockEntityData::new(bell)));
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bow", 1))));
        assert!(ctx.set_item(ALEX, Slot::Main(1), Some(&Item::new("minecraft:arrow", 8))));
        let villager = ctx.spawn_entity("minecraft:villager_v2", [0.5, 65.0, 3.5]).expect("spawned");
        assert!(ctx.give_entity_effect(villager, sapling_api::Effect::Slowness, 400, 255));
    });
    face(&mut h, 1, ALEX, [0.5, 65.0, 0.5], 5.0);
    draw(&mut h, 10, 10);
    h.shard.save_worlds();
    (storage, h)
}

#[test]
fn an_arrow_on_a_villager_costs_its_archer_standing_as_a_hit() {
    let (storage, _h) = shot_once();
    assert_eq!(standings(&storage), [-1], "bds-y3ranged.txt: shot twice to death, -1 and then the kill's -2");
}

#[test]
fn a_standing_is_kept_under_the_players_unique_id() {
    let (storage, _h) = shot_once();
    let ids: Vec<Value> = kept_players(&storage).iter().filter_map(|player| player.get("ID").cloned()).collect();
    assert_eq!(ids, [Value::Long(ARCHER)], "bds-f14nvillage.txt: `ID` is the player's ActorUniqueID");
}

#[test]
fn a_new_villagers_first_village_raises_every_kept_standing_by_five() {
    let (storage, mut h) = shot_once();
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:villager_v2", [4.5, 65.0, 5.5]).is_some()));
    (0..40).for_each(|_| drop(h.tick_all()));
    h.shard.save_worlds();
    assert_eq!(standings(&storage), [4], "bds-y3found.txt: a hit's -1, then a villager summoned");
}
