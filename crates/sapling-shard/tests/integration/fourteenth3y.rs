//! Group Y of the fourteenth parity brief: a chest pair's put (research/captures/bds-u3chest.txt), a
//! chest removed while open, a killed cart's drop (bds-ffburn.txt), and what pistons tell of what
//! they move (bds-u3carry.txt, bds-ffpush.txt).

use std::sync::{Arc, Mutex};

use crate::common;
use crate::riding::spawned;

use acacia_proto::Packet;
use acacia_proto::packets::{AddItemEntity, BlockEntityData, BlockEvent, ContainerClose, ContainerOpen, LevelSoundEvent, MoveEntityDelta, UpdateSubchunkBlocks};
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const HERE: [i32; 3] = [1, 65, 0];
const THERE: [i32; 3] = [2, 65, 0];

fn alex(items: Vec<Item>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        for (slot, item) in items.iter().enumerate() {
            assert!(ctx.set_item(PlayerId(1), Slot::Main(slot as u8), Some(item)));
        }
    });
    h.tick_all();
    h
}

fn put(h: &mut Harness, blocks: &'static [([i32; 3], &'static str, &'static str)]) {
    h.remote.run(move |ctx| {
        for (pos, name, properties) in blocks {
            assert!(ctx.set_block(*pos, &Block::new(*name).with(*properties)), "{pos:?} {name}");
        }
    });
    h.tick_all();
}

fn at(position: &acacia_proto::types::BlockCoordinates) -> [i32; 3] {
    [position.x, position.y, position.z]
}

#[test]
fn a_chest_put_beside_another_is_heard_as_its_default_state_and_shown_paired_a_tick_on() {
    // bds-u3chest.txt: `place` names the north-facing chest; the next tick one UpdateSubchunkBlocks
    // of lead, other, other, lead and the four entities in that order.
    let mut h = alex(vec![Item::new("minecraft:chest", 2)]);
    h.receive(1, &click([HERE[0], 64, HERE[2]], 1, Some("minecraft:chest")));
    h.tick_all();
    h.receive(1, &click([THERE[0], 64, THERE[2]], 1, Some("minecraft:chest")));
    let placed = h.tick_all().to(1).to_vec();
    let registry = sapling_world::BlockRegistry::vanilla();
    let north = registry.find("minecraft:chest", "minecraft:cardinal_direction=north").expect("a chest state");
    let heard: Vec<i32> = all::<LevelSoundEvent>(&placed).iter().filter(|sound| sound.sound_id == "place").map(|sound| sound.extra_data).collect();
    assert_eq!(heard, [registry.network_hash(north) as i32]);
    let next = h.tick_all().to(1).to_vec();
    let cells: Vec<[i32; 3]> = all::<UpdateSubchunkBlocks>(&next).iter().flat_map(|batch| batch.blocks.iter().map(|block| at(&block.position))).collect();
    let [lead, other] = [cells[0], cells[1]];
    assert!([lead, other] == [HERE, THERE] || [lead, other] == [THERE, HERE], "{cells:?}");
    assert_eq!(cells, [lead, other, other, lead]);
    let entities: Vec<[i32; 3]> = all::<BlockEntityData>(&next).iter().map(|data| at(&data.position)).collect();
    assert_eq!(entities, [lead, other, other, lead]);
}

#[test]
fn a_chest_removed_while_open_shuts_its_lid_either_side_of_the_close() {
    // bds-u3chest.txt: BlockEvent 0, ContainerClose (server), BlockEvent 0; no sound.
    let mut h = alex(Vec::new());
    put(&mut h, &[(HERE, "chest", "minecraft:cardinal_direction=north")]);
    h.receive(1, &click(HERE, 1, None));
    assert_eq!(all::<ContainerOpen>(h.tick_all().to(1)).len(), 1);
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_block(HERE, &Block::air())));
    let sent: Vec<_> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let told: Vec<u32> = sent.iter().map(|packet| packet.id).filter(|id| [BlockEvent::ID, ContainerClose::ID].contains(id)).collect();
    assert_eq!(told, [BlockEvent::ID, ContainerClose::ID, BlockEvent::ID]);
    assert!(all::<BlockEvent>(&sent).iter().all(|lid| lid.data == 0 && at(&lid.position) == HERE));
    assert!(all::<ContainerClose>(&sent)[0].server);
    assert!(!all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id.contains("chest")));
}

#[test]
fn a_killed_cart_drops_its_item_at_its_wire_point() {
    // bds-ffburn.txt: a cart on the floor at -60 left its item at -59.925 on the wire.
    let mut h = alex(Vec::new());
    put(&mut h, &[(HERE, "rail", "")]);
    let cart = spawned(&mut h, &[("minecraft:minecart", [1.5, 65.0, 0.5])])[0];
    (0..5).for_each(|_| _ = h.tick_all());
    let feet = common::asking::ask(&mut h, move |ctx| ctx.entity(cart).map_or(f32::NAN, |info| info.feet[1]));
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cart)));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let dropped = all::<AddItemEntity>(&sent).first().map(|item| item.position.y).expect("its item");
    assert!((dropped - feet - 0.475).abs() < 1e-3, "{feet} -> {dropped}");
}

#[test]
fn a_piston_tells_what_it_moves_as_its_own_and_keeps_a_carpet_on_its_arm() {
    // bds-u3carry.txt: every piston-caused move has force_move_local_entity; bds-ffpush.txt: a carpet
    // on a pushed stone stayed on the arm.
    let mut h = alex(Vec::new());
    put(&mut h, &[([0, 65, 0], "piston", "facing_direction=4"), ([1, 65, 0], "stone", ""), ([0, 65, 4], "piston", "facing_direction=4"), ([1, 65, 4], "stone", ""), ([1, 66, 4], "white_carpet", "")]);
    let stand = spawned(&mut h, &[("minecraft:armor_stand", [1.5, 66.0, 0.5])])[0];
    let before: Vec<MoveEntityDelta> = (0..3).flat_map(|_| all::<MoveEntityDelta>(h.tick_all().to(1))).collect();
    assert!(before.iter().all(|moved| !moved.force_move_local_entity));
    put(&mut h, &[([0, 66, 0], "redstone_block", ""), ([0, 66, 4], "redstone_block", "")]);
    let moved: Vec<MoveEntityDelta> = (0..3).flat_map(|_| all::<MoveEntityDelta>(h.tick_all().to(1))).filter(|moved| moved.runtime_entity_id == stand.0).collect();
    assert!(moved.first().is_some_and(|first| first.force_move_local_entity), "{moved:?}");
    (0..10).for_each(|_| _ = h.tick_all());
    let carpet = common::asking::ask(&mut h, |ctx| ctx.block([1, 66, 4]).map(|block| block.name.clone()));
    assert_eq!(carpet.as_deref(), Some("minecraft:white_carpet"));
}
