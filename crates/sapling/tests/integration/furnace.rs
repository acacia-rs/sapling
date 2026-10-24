//! A blast furnace on a real connection: a bot places it, puts raw iron and coal in, and takes
//! the ingots out once they are smelted.

use crate::common;

use acacia_bot::BotConfig;
use acacia_bot::client::PacketFilter;
use acacia_bot::interact::Face;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::ContainerSetData;
use common::packet;
use sapling::{Block, Handler, Item, Player, ShardCtx, Slot};

/// A ground block two east of the spawn, which the server makes dirt with air above it.
const BASE: [i32; 3] = [common::SPAWN.0 + 2, common::GROUND, common::SPAWN.1];
const FURNACE: [i32; 3] = [BASE[0], BASE[1] + 1, BASE[2]];
const BESIDE: [i32; 3] = [BASE[0], BASE[1], BASE[2] + 1];
const HOPPER: [i32; 3] = [BESIDE[0], BESIDE[1] + 1, BESIDE[2]];

struct Kit;

impl Handler for Kit {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        for [x, y, z] in [BASE, BESIDE] {
            assert!(ctx.set_block([x, y, z], &Block::new("dirt")));
            (1..=2).for_each(|up| assert!(ctx.set_block([x, y + up, z], &Block::air())));
        }
        let kit = [Item::new("minecraft:blast_furnace", 1), Item::new("minecraft:raw_iron", 2), Item::new("minecraft:coal", 3), Item::new("minecraft:hopper", 1)];
        for (slot, item) in kit.into_iter().enumerate().map(|(slot, item)| (slot as u8, item)) {
            assert!(ctx.set_item(player.id, Slot::Main(slot), Some(&item)));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_blast_furnace_smelts_what_a_bot_puts_in_on_one_piece_of_coal() {
    let server = common::start(crate::common::builder().handler(Kit)).await;
    let watching = PacketFilter::none().with(ContainerSetData::ID);
    let mut bot = common::spawn(common::client(&server, "Alice"), BotConfig { subscribe: watching, ..BotConfig::default() }).await;
    bot.wait_ticks(20).await.expect("connected");

    bot.place_block(BASE, Face::Up).await.expect("the hand holds a blast furnace");
    bot.wait_ticks(5).await.expect("connected");
    let smelted = bot.smelt(FURNACE, "minecraft:raw_iron", 2, Some(("minecraft:coal", 3))).await.expect("the iron smelts");
    assert_eq!(smelted, 2);
    let held = |name: &str| bot.state().items.id(name).map_or(0, |id| bot.state().inventory.count_of(id));
    assert_eq!((held("minecraft:iron_ingot"), held("minecraft:raw_iron"), held("minecraft:coal")), (2, 0, 0));

    // The screen was told of the fire as it was lit: a piece of coal burns for 800 ticks here.
    let lit = loop {
        let told = packet::<ContainerSetData>(&mut bot).await;
        if told.property == 2 && told.value > 0 {
            break told.value;
        }
    };
    assert_eq!(lit, 800);

    // One piece of coal did for both; the other two are still in it.
    let again = bot.open_container_at(FURNACE).await.expect("a furnace opens again");
    assert_eq!(again.slots.iter().map(|slot| slot.count).collect::<Vec<_>>(), [0, 2, 0]);
    bot.close_container().await.expect("the server confirms a close");

    // A hopper, an item that names no block, is placed by its name and opens with five slots.
    bot.select_hotbar(3).expect("a hotbar slot");
    bot.place_block(BESIDE, Face::Up).await.expect("the hand holds a hopper");
    bot.wait_ticks(5).await.expect("connected");
    assert_eq!(bot.open_container_at(HOPPER).await.expect("a hopper opens").slots.len(), 5);
    bot.close_container().await.expect("the server confirms a close");

    assert_eq!(bot.disconnect_reason(), None);
    bot.disconnect().await;
    server.shutdown().await;
}
