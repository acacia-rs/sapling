//! An End gateway that comes back from storage, as BDS's did after a restart
//! (research/captures/bds-d2gate2.txt): shown in its first tick, then relinked, or given a far end
//! when it has no exit.

use crate::common;

use std::sync::Arc;

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use acacia_proto::packets::{BlockEntityData, MovePlayer, MovePlayerMode, PlayerAction};
use bytes::BytesMut;
use common::{Harness, all};
use sapling_api::PlayerId;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, END, MemoryStorage, StoredColumn, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);
const GATEWAY: [i32; 3] = [96, 49, 8];

/// An End stored with end stone at y 40..48 over x 84..116, z -16..16, and a gateway at (96, 49, 8)
/// that has ticked 107 times, with `exit`.
fn stored_end(exit: Option<[i32; 3]>) -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let [end_stone, gateway] = ["minecraft:end_stone", "minecraft:end_gateway"].map(|name| registry.states_of(name).next().unwrap().0);
    let storage = Arc::new(MemoryStorage::default());
    for cx in 5..=7 {
        for cz in -1..=1 {
            let mut chunk = Chunk::empty(cx, cz, Dimension::end(registry.air_id()));
            for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))).filter(|(x, z)| (84..=116).contains(&(cx * 16 + x)) && (-16..=16).contains(&(cz * 16 + z))) {
                (40..=48).for_each(|y| chunk.set(x, y, z, 0, end_stone));
            }
            let mut column = StoredColumn::of(chunk);
            if ChunkPos::of_block(GATEWAY[0], GATEWAY[2]) == ChunkPos::new(cx, cz) {
                column.chunk.set(GATEWAY[0] - cx * 16, GATEWAY[1], GATEWAY[2] - cz * 16, 0, gateway);
                let mut fields = vec![("Age", Value::Int(107)), ("EndGatewayBadPosChecked", Value::Byte(1)), ("id", Value::String("EndGateway".into()))];
                fields.extend(exit.map(|exit| ("ExitPortal", Value::List(List { tag: 3, items: exit.map(Value::Int).into() }))));
                fields.extend([("x", Value::Int(GATEWAY[0])), ("y", Value::Int(GATEWAY[1])), ("z", Value::Int(GATEWAY[2]))]);
                let entity = Value::Compound(fields.into_iter().map(|(key, value)| (nbt::Str::from(key), value)).collect());
                let mut bytes = BytesMut::new();
                nbt::write::<LittleEndian>(&mut bytes, &Nbt { name: String::new(), value: entity });
                column.block_entities = bytes.to_vec();
            }
            storage.store_column(ChunkPos::new(cx, cz), END, &column).unwrap();
        }
    }
    storage
}

fn ack() -> PlayerAction {
    let origin = acacia_proto::types::BlockCoordinates { x: 0, y: 0, z: 0 };
    PlayerAction { runtime_entity_id: 1, action: acacia_proto::types::Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 }
}

/// Alex in the stored End beside the gateway; the gateway's entity each time it is shown, for up to `ticks`.
fn shown_in(exit: Option<[i32; 3]>, ticks: usize, until: impl Fn(&Value) -> bool) -> (Harness, Vec<Value>) {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", World::with_storage_in(stored_end(exit), END)));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [96.5, 49.0, 4.5])));
    let mut shown = Vec::new();
    for tick in 0..ticks {
        if tick == 5 {
            h.receive(1, &ack());
        }
        let sent = h.tick(1);
        shown.extend(all::<BlockEntityData>(&sent).into_iter().filter(|data| [data.position.x, data.position.y, data.position.z] == GATEWAY).map(|data| data.nbt.decode().unwrap().value));
        if tick > 5 && shown.last().is_some_and(&until) {
            break;
        }
    }
    (h, shown)
}

fn exit_of(entity: &Value) -> Option<Vec<i32>> {
    match entity.get("ExitPortal") {
        Some(Value::List(list)) => Some(list.items.iter().filter_map(|c| if let Value::Int(c) = c { Some(*c) } else { None }).collect()),
        _ => None,
    }
}

#[test]
fn a_stored_gateway_is_shown_then_relinked_onto_the_end_stone_round_its_exit() {
    let (mut h, shown) = shown_in(Some([100, 50, 0]), 300, |entity| exit_of(entity) == Some(vec![100, 48, 0]));
    let seen: Vec<(Option<&Value>, Option<Vec<i32>>)> = shown.iter().map(|entity| (entity.get("Age"), exit_of(entity))).collect();
    assert_eq!(seen.first(), Some(&(Some(&Value::Int(108)), Some(vec![100, 50, 0]))), "shown in its first tick: {seen:?}");
    assert_eq!(seen.last().map(|(_, exit)| exit.clone()), Some(Some(vec![100, 48, 0])), "the top end stone nearest the old exit: {seen:?}");
    h.remote.run(|ctx| ctx.teleport(ALEX, [96.5, 49.0, 8.5]));
    let mut sent = Vec::new();
    (0..3).for_each(|_| sent.extend(h.tick(1)));
    let moved: Vec<[f32; 3]> = all::<MovePlayer>(&sent).into_iter().filter(|moved| moved.mode == MovePlayerMode::Teleport).map(|moved| [moved.position.x, moved.position.y - sapling_data::EYE_HEIGHT, moved.position.z]).collect();
    assert_eq!(moved.last(), Some(&[100.5, 49.0, 0.5]), "a block over the new exit");
}

#[test]
fn a_stored_gateway_with_no_exit_gets_a_far_end() {
    let (_, shown) = shown_in(None, 600, |entity| exit_of(entity).is_some());
    let far = shown.last().and_then(exit_of).expect("an exit");
    let reach = far[0].abs().max(far[2].abs());
    assert!(reach > 900, "out along the gateway's way from the origin: {far:?}");
}
