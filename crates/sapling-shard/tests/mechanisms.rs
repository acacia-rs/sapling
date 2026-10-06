//! Blocks that move on their own: pressure plates under a player, sand over a gap
//! (harness: common/mod.rs).

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, LevelSoundEvent, MoveEntityDelta, RemoveEntity, UpdateBlock};
use acacia_proto::types::{Action, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, block_actions, hash, input_at};
use sapling_api::{Block, GameMode, PlayerId};

const PLATE: [i32; 3] = [2, 65, 0];

fn plate_state(signal: u8) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find("minecraft:stone_pressure_plate", &format!("redstone_signal={signal}")).expect("a plate state"))
}

/// The plate's states and clicks among `packets`, in order.
fn plate_news(packets: &[RawPacket]) -> (Vec<u32>, Vec<String>) {
    let states = all::<UpdateBlock>(packets).iter().filter(|u| [u.position.x, u.position.y, u.position.z] == PLATE).map(|u| u.block_runtime_id).collect();
    let clicks = all::<LevelSoundEvent>(packets).iter().map(|sound| format!("{:?}", sound.sound_id)).collect();
    (states, clicks)
}

#[test]
fn a_plate_goes_down_under_a_player_and_up_a_check_after_it_left() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.set_block(PLATE, &Block::new("stone_pressure_plate"))));
    h.tick_all();

    // Alex is 0.6 wide; the plate feels from 1/16 inside its cell.
    for x in [1.0, 1.5] {
        h.receive(1, &input_at([x, 65.0, 0.5]));
        assert_eq!(plate_news(h.tick_all().to(2)), (vec![], vec![]), "not reached at {x}");
    }
    h.receive(1, &input_at([2.0, 65.0, 0.5]));
    let (states, clicks) = plate_news(h.tick_all().to(2));
    assert_eq!(states, [plate_state(1)], "down, storing 1");
    assert_eq!(clicks.len(), 1, "with a click: {clicks:?}");
    let held: Vec<_> = (0..45).map(|_| plate_news(h.tick_all().to(2))).filter(|(states, clicks)| !states.is_empty() || !clicks.is_empty()).collect();
    assert!(held.is_empty(), "stays down while stood on: {held:?}");

    h.receive(1, &input_at([1.0, 65.0, 0.5]));
    let after: Vec<_> = (0..21).map(|_| plate_news(h.tick_all().to(2))).filter(|(states, _)| !states.is_empty()).collect();
    assert_eq!(after.len(), 1, "up on the next check: {after:?}");
    assert_eq!((after[0].0.as_slice(), after[0].1.len()), ([plate_state(0)].as_slice(), 1));
}

/// The layer-0 changes among `packets` at `x`, as (y, block).
fn column_changes(packets: &[RawPacket], x: i32) -> Vec<(i32, u32)> {
    all::<UpdateBlock>(packets).iter().filter(|u| u.layer == 0 && u.position.x == x && u.position.z == 0).map(|u| (u.position.y, u.block_runtime_id)).collect()
}

#[test]
fn sand_falls_as_an_entity_and_breaks_up_on_a_torch() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // Two columns of sand held up by stone at y 67, three cells over the platform; a torch
    // stands under the second.
    h.remote.run(|ctx| {
        for x in [3, 4] {
            assert!(ctx.set_block([x, 67, 0], &Block::new("stone")) && ctx.set_block([x, 68, 0], &Block::new("sand")));
        }
        assert!(ctx.set_block([4, 65, 0], &Block::new("torch").with("torch_facing_direction=top")));
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
    });
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, [3, 67, 0]), (Action::PredictBreak, [4, 67, 0])]));
    let sent: Vec<RawPacket> = (0..30).flat_map(|_| h.tick_all().to(2).to_vec()).collect();

    let (sand, air) = (hash("minecraft:sand"), hash("minecraft:air"));
    assert_eq!(column_changes(&sent, 3), [(67, air), (68, air), (65, sand)], "the stone, the sand leaving, the sand landing");
    assert_eq!(column_changes(&sent, 4), [(67, air), (68, air)], "the second never lands");
    let falling: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|entity| entity.entity_type == "minecraft:falling_block").collect();
    assert_eq!(falling.len(), 2);
    let variant = falling[0].metadata.iter().find(|item| item.key == Key::Variant).map(|item| item.value.clone());
    // BDS shows it a tick into its fall, 0.49 above the bottom of its box (research/captures/).
    assert_eq!(variant, Some(Value::Default(Plain::Int(sand as i32))), "it names its block");
    assert!((falling[0].position.y - 68.46).abs() < 1e-4 && (falling[0].velocity.y + 0.0392).abs() < 1e-6, "{:?} {:?}", falling[0].position, falling[0].velocity);
    let heights: Vec<f32> = all::<MoveEntityDelta>(&sent).iter().filter(|moved| moved.runtime_entity_id == falling[0].runtime_id).filter_map(|moved| moved.y).collect();
    assert!(heights.windows(2).all(|pair| pair[1] <= pair[0]) && heights.last() == Some(&65.49), "down to the platform: {heights:?}");
    assert_eq!(all::<RemoveEntity>(&sent).len(), 2, "both entities go");
    let items: Vec<AddItemEntity> = all::<AddItemEntity>(&sent);
    let sand_item = sapling_data::item(&sapling_data::Stack { name: "minecraft:sand", count: 1, damage: 0, block: None, tag: None }, None).unwrap();
    assert!(items.iter().any(|item| item.item.network_id == sand_item.network_id), "the second leaves sand where the torch is");
}
