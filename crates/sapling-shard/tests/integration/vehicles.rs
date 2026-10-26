//! Boats and minecarts: placed from their items, broken back into them, rolled along powered rails,
//! opened, and a TNT minecart's fuse (docs/research/vehicles.md, research/captures/bds-boats.txt).

use crate::common;
use crate::riding::{interact, spawned};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, ContainerOpen, LevelEvent, LevelEventEvent};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, WindowType};
use common::{Harness, all, attack, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick(1)).collect()
}

fn holding(h: &mut Harness, item: &'static str) {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, 1)))));
    h.tick(1);
}

fn added(sent: &[RawPacket], kind: &str) -> Option<AddEntity> {
    all::<AddEntity>(sent).into_iter().find(|entity| entity.entity_type == kind)
}

fn int(entity: &AddEntity, key: Key) -> Option<i32> {
    entity.metadata.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Int(value)) = item.value { Some(value) } else { None })
}

#[test]
fn a_boat_item_places_its_wood_on_a_floor_and_four_bare_hits_break_it_back_into_the_item() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    holding(&mut h, "minecraft:birch_boat");
    h.receive(1, &click([2, 64, 0], 1, None));
    let sent = ticks(&mut h, 2);
    let boat = added(&sent, "minecraft:boat").expect("a boat");
    assert_eq!(int(&boat, Key::Variant), Some(2), "birch");
    assert_eq!(int(&boat, Key::Health), Some(40));
    // Announced at the click point, the floor's top, as BDS's placed boat was (bds-boats.txt, `!place 3 -1 0`).
    assert!((boat.position.y - 65.0).abs() < 1e-4 && (boat.position.x - 2.5).abs() < 1e-4, "{:?}", boat.position);
    h.remote.run(|ctx| assert!(ctx.item(ALEX, Slot::Main(0)).is_none(), "the item was spent"));
    // Each hit takes 10 off and each tick mends 1: the fourth leaves 3, the fifth breaks it.
    let mut sent = Vec::new();
    for _ in 0..4 {
        h.receive(1, &attack(boat.runtime_id));
        sent.extend(ticks(&mut h, 1));
    }
    assert!(all::<AddItemEntity>(&sent).is_empty());
    h.receive(1, &attack(boat.runtime_id));
    assert_eq!(all::<AddItemEntity>(&ticks(&mut h, 2)).len(), 1, "the boat's item");
    h.remote.run(move |ctx| assert!(ctx.entity(sapling_api::EntityId(boat.runtime_id)).is_none()));
    h.tick(1);
}

#[test]
fn a_minecart_goes_only_on_a_rail_at_its_middle() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    holding(&mut h, "minecraft:minecart");
    h.receive(1, &click([2, 64, 0], 1, None));
    assert!(added(&ticks(&mut h, 2), "minecraft:minecart").is_none(), "not on stone");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &Block::new("rail"))));
    h.tick(1);
    h.receive(1, &click([2, 65, 0], 1, None));
    let cart = added(&ticks(&mut h, 2), "minecraft:minecart").expect("on the rail");
    assert!((cart.position.x - 2.5).abs() < 1e-4 && (cart.position.z - 0.5).abs() < 1e-4 && (cart.position.y - 65.35).abs() < 1e-3, "{:?}", cart.position);
}

#[test]
fn a_powered_rail_kicks_a_cart_off_a_wall_and_speeds_it_along() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([-4, 65, 3], &Block::new("stone")));
        for x in -3..=6 {
            assert!(ctx.set_block([x, 64, 3], &Block::new("redstone_block")));
            assert!(ctx.set_block([x, 65, 3], &Block::new("golden_rail").with("rail_data_bit=0,rail_direction=1")));
        }
    });
    ticks(&mut h, 3);
    let cart = spawned(&mut h, &[("minecraft:minecart", [-2.5, 65.0, 3.5])])[0];
    ticks(&mut h, 20);
    h.remote.run(move |ctx| {
        assert!(ctx.block([0, 65, 3]).is_some_and(|rail| rail.properties.contains("rail_data_bit=1")), "powered");
        let at = ctx.entity(cart).expect("the cart").feet;
        assert!(at[0] > 0.0 && (at[2] - 3.5).abs() < 1e-3, "rolled east along the rail: {at:?}");
    });
    h.tick(1);
}

#[test]
fn a_chest_minecart_opens_its_own_screen() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cart = spawned(&mut h, &[("minecraft:chest_minecart", [2.5, 65.0, 0.5])])[0];
    h.receive(1, &interact(cart.0));
    let sent = ticks(&mut h, 2);
    let opened = all::<ContainerOpen>(&sent);
    assert!(opened.len() == 1 && opened[0].window_type == WindowType::MinecartChest && opened[0].runtime_entity_id == cart.0 as i64, "{opened:?}");
}

#[test]
fn flint_and_steel_primes_a_tnt_minecart_which_bursts_eighty_ticks_on() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cart = spawned(&mut h, &[("minecraft:tnt_minecart", [3.5, 65.0, 0.5])])[0];
    holding(&mut h, "minecraft:flint_and_steel");
    h.receive(1, &interact(cart.0));
    let burst = (1..=90).find(|_| all::<LevelEvent>(&h.tick(1)).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion));
    assert!(burst.is_some_and(|tick| (79..=81).contains(&tick)), "{burst:?}");
    h.remote.run(move |ctx| assert!(ctx.entity(cart).is_none()));
    h.tick(1);
}
