//! The eye of ender thrown, as BDS throws it (research/captures/bds-eye-throw.txt, bds-eye-dims.txt).

use crate::common;

use std::sync::Arc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, EntityEvent, EntityEventEventId, InventoryTransaction, LevelEvent, LevelEventEvent, LevelSoundEvent, MoveEntityDelta, PlayerAuthInput, RemoveEntity};
use acacia_proto::types::{InputData, TransactionTransactionData, TransactionUseItemActionType};
use common::{Harness, all, input_at};
use sapling_api::{GameMode, Item, PlayerId, Slot};
use sapling_world::{ChunkPos, MemoryStorage, StorageError, StoredColumn, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);
/// BDS's stronghold for seed 20261018 thrown from the origin.
const STRONGHOLD: [i32; 3] = [-1454, 32, -2542];
const FEET: [f32; 3] = [0.5, 151.0, 0.5];

/// A void overworld whose generator would put a stronghold at `STRONGHOLD`.
struct Strongholds;

impl WorldStorage for Strongholds {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        MemoryStorage::default().load_column(pos, dimension)
    }
    fn store_column(&self, _: ChunkPos, _: i32, _: &StoredColumn) -> Result<(), StorageError> {
        Ok(())
    }
    fn locate_stronghold(&self, _: i32, _: i32) -> Option<[i32; 3]> {
        Some(STRONGHOLD)
    }
}

fn click_air() -> InventoryTransaction {
    let mut click = common::click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut click.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    click
}

/// Alex at `FEET` in a world of `level`, two eyes in hand, in `mode`.
fn ready(level: World, mode: GameMode) -> Harness {
    let mut h = Harness::with(common::config(), level);
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // The columns come from storage's threads.
    (0..10).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, mode);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:ender_eye", 2))));
        ctx.teleport(ALEX, FEET);
    });
    (0..3).for_each(|_| drop(h.tick_all()));
    h.receive(1, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..input_at(FEET) });
    h.tick_all();
    h
}

fn assert_eyes_held(h: &mut Harness, count: u8) {
    h.remote.run(move |ctx| assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|item| item.count), Some(count)));
    h.tick_all();
}

/// Throws, then the eye's 81 ticks of flight, as Steve is told them.
fn throw(h: &mut Harness) -> (Vec<RawPacket>, Vec<RawPacket>) {
    h.receive(1, &click_air());
    let thrown = h.tick_all().to(2).to_vec();
    let flight = (0..82).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    (thrown, flight)
}

#[test]
fn a_survival_eye_flies_bds_path_toward_the_stronghold_and_is_spent() {
    let mut h = ready(World::with_storage(Arc::new(Strongholds)), GameMode::Survival);
    let (thrown, flight) = throw(&mut h);
    assert!(all::<EntityEvent>(&thrown).iter().any(|event| event.event_id == EntityEventEventId::ArmSwing));
    let eye = all::<AddEntity>(&thrown).into_iter().find(|added| added.entity_type == "minecraft:eye_of_ender_signal").expect("an eye");
    assert_eq!([eye.position.x, eye.position.y, eye.position.z], [1.0, 153.12001, 1.0], "half a block past the eyes");
    let sound = all::<LevelSoundEvent>(&thrown).into_iter().find(|sound| sound.sound_id == "throw").expect("the throw heard");
    assert_eq!(([sound.position.x, sound.position.y, sound.position.z], sound.entity_type.as_str()), ([0.5, 152.32, 0.65], "minecraft:player"));
    assert_eyes_held(&mut h, 1);

    let path: Vec<[f32; 3]> = all::<MoveEntityDelta>(&flight).iter().filter(|delta| delta.runtime_entity_id == eye.runtime_id).map(|d| [d.x, d.y, d.z].map(Option::unwrap_or_default)).collect();
    // bds-eye-throw.txt: the first move BDS told, toward the stronghold's side of the origin.
    let first = path[0];
    assert!((first[0] - 0.985_103_1).abs() < 1e-5 && (first[1] - 153.135_01).abs() < 1e-5 && (first[2] - 0.973_96).abs() < 1e-5, "{first:?}");
    let ([dx, dz], [sx, sz]) = ([first[0] - 1.0, first[2] - 1.0], [STRONGHOLD[0] as f32 - 1.0, STRONGHOLD[2] as f32 - 1.0]);
    assert!((dx * sz - dz * sx).abs() < 1e-3 * dx.hypot(dz) * sx.hypot(sz) && dx * sx > 0.0, "flies at the stronghold");
    let ended = [-4.959_399_7, 161.121_86, -9.415_638];
    let broke = all::<LevelEvent>(&flight).into_iter().filter(|event| event.event == LevelEventEvent::ParticleEyeDespawn).map(|e| [e.position.x, e.position.y, e.position.z]);
    let dropped = all::<AddItemEntity>(&flight).into_iter().map(|item| [item.position.x, item.position.y - 0.125, item.position.z]);
    let end: Vec<[f32; 3]> = broke.chain(dropped).collect();
    assert_eq!(end.len(), 1, "it breaks or drops, once");
    assert!((0..3).all(|axis| (end[0][axis] - ended[axis]).abs() < 1e-4), "where BDS's ended: {:?}", end[0]);
    assert!(all::<RemoveEntity>(&flight).iter().any(|gone| gone.entity_id_self == eye.unique_id));
}

#[test]
fn a_creative_eye_is_not_spent() {
    let mut h = ready(World::with_storage(Arc::new(Strongholds)), GameMode::Creative);
    let (thrown, _) = throw(&mut h);
    assert_eq!(all::<AddEntity>(&thrown).len(), 1);
    assert_eyes_held(&mut h, 2);
}

#[test]
fn no_eye_is_thrown_where_no_stronghold_is_located() {
    // bds-eye-dims.txt: in the Nether and the End nothing happens and nothing is spent.
    let mut h = ready(common::platform(), GameMode::Survival);
    let (thrown, flight) = throw(&mut h);
    assert!(all::<AddEntity>(&thrown).is_empty() && all::<AddEntity>(&flight).is_empty());
    assert!(!all::<LevelSoundEvent>(&thrown).iter().any(|sound| sound.sound_id == "throw"));
    assert_eyes_held(&mut h, 2);
}
