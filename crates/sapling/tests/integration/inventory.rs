//! Inventories on a real connection: a bot moves, swaps and throws what a handler gave it, as a
//! vanilla client does (screen opened around every request), while a second bot watches.

use crate::common;

use acacia_bot::client::PacketFilter;
use acacia_bot::items::SlotRef;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{AddItemEntity, MobEquipment, TakeItemEntity};
use acacia_bot::{ActionError, Bot, BotConfig};
use acacia_bot::interact::Face;
use common::packet;
use sapling::{Block, DropEvent, Handler, Item, Player, ShardCtx, Slot};

/// A ground block two east of the spawn, which the server makes dirt with air above it.
const BASE: [i32; 3] = [common::SPAWN.0 + 2, common::GROUND, common::SPAWN.1];

/// Gives Alice stone and diamonds, and lets nobody throw diamonds away.
struct Kit;

impl Handler for Kit {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        if player.name == "Alice" {
            assert!(ctx.set_block(BASE, &Block::new("dirt")));
            (1..=2).for_each(|up| assert!(ctx.set_block([BASE[0], BASE[1] + up, BASE[2]], &Block::air())));
            assert!(ctx.set_item(player.id, Slot::Main(0), Some(&Item::new("minecraft:stone", 20))));
            assert!(ctx.set_item(player.id, Slot::Main(1), Some(&Item::new("minecraft:diamond", 3))));
        }
    }

    fn on_drop(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut DropEvent) {
        if event.item().name == "minecraft:diamond" {
            event.cancel();
        }
    }
}

/// How many items the bot's own copy of its inventory has in a slot.
fn count(bot: &Bot, slot: u8) -> u16 {
    SlotRef::Main(slot).stack(bot.state()).map_or(0, |stack| stack.count)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_moves_swaps_and_throws_its_items_and_takes_them_back() {
    let server = common::start(crate::common::builder().handler(Kit)).await;
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    let watching = PacketFilter::none().with(AddItemEntity::ID).with(TakeItemEntity::ID).with(MobEquipment::ID);
    let mut bob = common::spawn(common::client(&server, "Bob"), BotConfig { subscribe: watching, ..BotConfig::default() }).await;
    alice.wait_ticks(20).await.expect("connected");
    assert_eq!((count(&alice, 0), count(&alice, 1), count(&bob, 0)), (20, 3, 0), "what the handler gave reaches the client");

    alice.move_item(SlotRef::Main(0), SlotRef::Main(10), 5).await.expect("a part of a stack moves");
    alice.swap_items(SlotRef::Main(1), SlotRef::Main(10)).await.expect("two stacks trade places");
    assert_eq!((count(&alice, 0), count(&alice, 1), count(&alice, 10)), (15, 5, 3));
    let kept = alice.drop_item(SlotRef::Main(10), 1).await;
    assert!(matches!(kept, Err(ActionError::Rejected(_))), "{kept:?}");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(count(&alice, 10), 3, "the handler keeps the diamonds, and the client is shown them");

    // Bob sees what Alice takes in hand, and the stone she throws.
    alice.select_hotbar(1).expect("a hotbar slot");
    let held: MobEquipment = packet(&mut bob).await;
    assert_eq!((held.runtime_entity_id, held.item.count, held.selected_slot), (alice.client().runtime_entity_id(), 5, 1));
    alice.drop_item(SlotRef::Main(0), 15).await.expect("stone may be thrown");
    let lying: AddItemEntity = packet(&mut bob).await;
    assert_eq!((lying.item.count, count(&alice, 0)), (15, 0));

    // Once it can be taken, Alice walks over it: it joins the stone she holds.
    alice.wait_ticks(45).await.expect("connected");
    alice.controls().expect("physics").forward = 1.0;
    alice.wait_ticks(12).await.expect("connected");
    alice.controls().expect("physics").stop();
    alice.wait_ticks(10).await.expect("connected");
    let taken: TakeItemEntity = packet(&mut bob).await;
    assert_eq!((taken.runtime_entity_id, u64::from(taken.target)), (lying.runtime_entity_id, alice.client().runtime_entity_id()));
    assert_eq!((count(&alice, 1), count(&alice, 10)), (20, 3));

    // Survival: a stone goes from the hand into the world and off the stack, and breaking it by
    // hand takes stone's seven and a half seconds, which the server holds the bot to.
    alice.place_block(BASE, Face::Up).await.expect("the hand holds stone");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(count(&alice, 1), 19);
    let started = std::time::Instant::now();
    alice.break_block([BASE[0], BASE[1] + 1, BASE[2]]).await.expect("a break that took its time is taken");
    let took = crate::common::clock::tick_ms(started);
    assert!(took > 7000, "{took} ms");

    for bot in [&mut alice, &mut bob] {
        assert_eq!(bot.disconnect_reason(), None);
        bot.disconnect().await;
    }
    server.shutdown().await;
}
