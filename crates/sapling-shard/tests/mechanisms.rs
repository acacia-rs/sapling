//! Blocks that move on their own: pressure plates under a player, sand over a gap
//! (harness: common/mod.rs).

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    AddEntity, AddItemEntity, LevelSoundEvent, MoveEntityDelta, RemoveEntity, UpdateBlock, UpdateBlockSynced, UpdateBlockSyncedTransitionType as Transition,
};
use acacia_proto::types::{Action, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, block_actions, block_changes, click, hash, input_at};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

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

#[test]
fn sand_placed_over_a_gap_falls_two_ticks_later_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let sand = Item::new("minecraft:sand", 8);
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&sand)));
        assert!(ctx.set_block([2, 67, 0], &Block::new("stone")));
    });
    h.tick_all();
    // On the east face of the stone: sand at (3, 67, 0), over three cells of air.
    h.receive(1, &click([2, 67, 0], 5, Some("minecraft:sand")));
    let falls = |sent: &[RawPacket]| all::<AddEntity>(sent).iter().filter(|entity| entity.entity_type == "minecraft:falling_block").count();
    let placed = h.tick_all();
    assert_eq!((column_changes(placed.to(2), 3), falls(placed.to(2))), (vec![(67, hash("minecraft:sand"))], 0), "placed, still");
    assert_eq!(falls(h.tick_all().to(2)), 0);
    assert_eq!(falls(h.tick_all().to(2)), 1, "falling on the second tick after");
}

#[test]
fn sand_falling_into_water_splashes_once_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([6, 65, 0], &Block::new("water")) && ctx.set_block([6, 67, 0], &Block::new("stone")) && ctx.set_block([6, 68, 0], &Block::new("sand")));
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
    });
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, [6, 67, 0])]));
    let sent: Vec<RawPacket> = (0..30).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let splashes: Vec<LevelSoundEvent> = all::<LevelSoundEvent>(&sent).into_iter().filter(|sound| sound.sound_id == "splash").collect();
    assert_eq!(splashes.len(), 1, "{splashes:?}");
    let splash = &splashes[0];
    // Heard from the middle of its box; louder the faster it went in (docs/research/splash-sound.md).
    assert!(splash.entity_type == "minecraft:falling_block" && splash.extra_data > 0 && (splash.position.x, splash.position.z) == (6.5, 0.5), "{splash:?}");
}

#[test]
fn concrete_powder_sets_where_water_touches_it_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // Pools in the platform walled with stone: two deep at x -6, one deep at x -2.
    h.remote.run(|ctx| {
        for (x, deep) in [(-6, 2), (-2, 1)] {
            for y in 62..=64 {
                for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
                    assert!(ctx.set_block([x + dx, y, dz], &Block::new("stone")));
                }
            }
            assert!(ctx.set_block([x, 64 - deep, 0], &Block::new("stone")));
            for y in 65 - deep..=64 {
                assert!(ctx.set_block([x, y, 0], &Block::new("water")));
            }
            assert!(ctx.update_block([x, 68, 0], &Block::new("red_concrete_powder")));
        }
        assert!(ctx.update_block([6, 65, 0], &Block::new("water")) && ctx.update_block([7, 65, 0], &Block::new("red_concrete_powder")));
    });
    let sent: Vec<RawPacket> = (0..30).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let (powder, concrete, water, stone, air) =
        (hash("minecraft:red_concrete_powder"), hash("minecraft:red_concrete"), hash("minecraft:water"), hash("minecraft:stone"), hash("minecraft:air"));
    assert_eq!(column_changes(&sent, 7), [(65, concrete)], "beside water it is put as concrete");
    let falls = [(68, powder), (68, air)];
    let deep = [[(62, stone), (63, water), (64, water)].as_slice(), &falls, &[(63, concrete)]].concat();
    assert_eq!(column_changes(&sent, -6), deep, "in water touching water it sets there, falling");
    let shallow = [[(63, stone), (64, water)].as_slice(), &falls, &[(64, powder)]].concat();
    assert_eq!(column_changes(&sent, -2), shallow, "in a pool one deep it lands as powder");
    let synced: Vec<_> = all::<UpdateBlockSynced>(&sent).iter().filter(|u| u.transition_type == Transition::Destroy).map(|u| u.position.x).collect();
    assert_eq!(synced, [-2], "only a landing names the falling block");
}

/// The layer-0 changes among `packets` at `x`, as (y, block).
fn column_changes(packets: &[RawPacket], x: i32) -> Vec<(i32, u32)> {
    block_changes(packets).into_iter().filter(|([at_x, _, z], _)| (*at_x, *z) == (x, 0)).map(|([_, y, _], block)| (y, block)).collect()
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
    // As BDS: the cell a falling block leaves and the one it lands in name it.
    let synced: Vec<_> = all::<UpdateBlockSynced>(&sent).iter().map(|u| (u.position.x, u.position.y, u.transition_type.clone())).collect();
    assert_eq!(synced, [(3, 68, Transition::Create), (4, 68, Transition::Create), (3, 65, Transition::Destroy)]);
    let falling: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|entity| entity.entity_type == "minecraft:falling_block").collect();
    assert_eq!(falling.len(), 2);
    let variant = falling[0].metadata.iter().find(|item| item.key == Key::Variant).map(|item| item.value.clone());
    // BDS shows it a tick into its fall, 0.49 above the bottom of its box (research/captures/).
    assert_eq!(variant, Some(Value::Default(Plain::Int(sand as i32))), "it names its block");
    let keys: Vec<Key> = falling[0].metadata.iter().take(3).map(|item| item.key.clone()).collect();
    assert_eq!(keys, [Key::Flags, Key::Health, Key::Variant], "BDS's order");
    assert!((falling[0].position.y - 68.46).abs() < 1e-4 && (falling[0].velocity.y + 0.0392).abs() < 1e-6, "{:?} {:?}", falling[0].position, falling[0].velocity);
    let heights: Vec<f32> = all::<MoveEntityDelta>(&sent).iter().filter(|moved| moved.runtime_entity_id == falling[0].runtime_id).filter_map(|moved| moved.y).collect();
    assert!(heights.windows(2).all(|pair| pair[1] <= pair[0]) && heights.last() == Some(&65.49), "down to the platform: {heights:?}");
    assert_eq!(all::<RemoveEntity>(&sent).len(), 2, "both entities go");
    let items: Vec<AddItemEntity> = all::<AddItemEntity>(&sent);
    let sand_item = sapling_data::item(&sapling_data::Stack { name: "minecraft:sand", count: 1, damage: 0, block: None, tag: None }, None).unwrap();
    let left = items.iter().find(|item| item.item.network_id == sand_item.network_id).expect("the second leaves sand where the torch is");
    // On BDS the item's box starts where the falling block's wire position was: 65.49, plus 0.125.
    assert!((left.position.y - 65.615).abs() < 1e-4 && left.velocity.y == 0.2, "{:?} {:?}", left.position, left.velocity);
}
