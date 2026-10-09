//! A chest on a real connection: one bot places it and fills it, a second takes from it while
//! the first looks on, and what is left spills when the chest is broken.

mod common;

use acacia_bot::client::PacketFilter;
use acacia_bot::interact::Face;
use acacia_bot::items::SlotRef;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::AddItemEntity;
use acacia_bot::{Bot, BotConfig};
use common::packet;
use sapling::{Block, Handler, Item, Player, Server, ShardCtx, Slot};

/// A ground block two east of the spawn, which the server makes dirt with air above it.
const BASE: [i32; 3] = [common::SPAWN.0 + 2, common::GROUND, common::SPAWN.1];
const CHEST: [i32; 3] = [BASE[0], BASE[1] + 1, BASE[2]];
/// The ground block south of it: a chest on it stands beside the first.
const BESIDE: [i32; 3] = [BASE[0], BASE[1], BASE[2] + 1];

/// Gives Alice a chest, diamonds and sticks.
struct Kit;

impl Handler for Kit {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        if player.name == "Alice" {
            for [x, y, z] in [BASE, BESIDE] {
                assert!(ctx.set_block([x, y, z], &Block::new("dirt")));
                (1..=2).for_each(|up| assert!(ctx.set_block([x, y + up, z], &Block::air())));
            }
            for (slot, item) in [
                (0, Item::new("minecraft:chest", 2)),
                (1, Item::new("minecraft:diamond", 3)),
                (2, Item::new("minecraft:stick", 8)),
            ] {
                assert!(ctx.set_item(player.id, Slot::Main(slot), Some(&item)));
            }
        }
    }
}

fn count(bot: &Bot, slot: SlotRef) -> u16 {
    slot.stack(bot.state()).map_or(0, |stack| stack.count)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_chest_keeps_what_one_bot_puts_in_for_the_next_and_spills_when_broken() {
    let server = common::start(Server::builder().handler(Kit)).await;
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    let watching = PacketFilter::none().with(AddItemEntity::ID);
    let mut bob = common::spawn(
        common::client(&server, "Bob"),
        BotConfig {
            subscribe: watching,
            ..BotConfig::default()
        },
    )
    .await;
    alice.wait_ticks(20).await.expect("connected");

    alice
        .place_block(BASE, Face::Up)
        .await
        .expect("the hand holds a chest");
    alice.wait_ticks(5).await.expect("connected");
    let opened = alice.open_container_at(CHEST).await.expect("a chest opens");
    assert_eq!(
        (
            opened.slots.len(),
            opened.slots.iter().filter(|slot| !slot.is_empty()).count()
        ),
        (27, 0)
    );
    alice.put_in_container(1, 4).await.expect("diamonds go in");
    alice.put_in_container(2, 5).await.expect("sticks go in");
    assert_eq!(
        (
            count(&alice, SlotRef::Main(1)),
            count(&alice, SlotRef::Container(4))
        ),
        (0, 3)
    );

    // Bob finds them there and takes the diamonds; Alice, still looking, sees them go.
    let found = bob
        .open_container_at(CHEST)
        .await
        .expect("a chest opens for a second player");
    assert_eq!((found.slots[4].count, found.slots[5].count), (3, 8));
    let went_to = bob.move_to_inventory(4).await.expect("diamonds come out");
    assert_eq!(count(&bob, went_to[0]), 3);
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(
        (
            count(&alice, SlotRef::Container(4)),
            count(&alice, SlotRef::Container(5))
        ),
        (0, 8)
    );
    alice
        .close_container()
        .await
        .expect("the server confirms a close");
    bob.close_container()
        .await
        .expect("the server confirms a close");

    // A second chest beside the first makes one of 54 slots; the one that stood there leads.
    alice
        .place_block(BESIDE, Face::Up)
        .await
        .expect("the hand holds another chest");
    alice.wait_ticks(5).await.expect("connected");
    let double = alice
        .open_container_at(CHEST)
        .await
        .expect("a double chest opens");
    assert_eq!((double.slots.len(), double.slots[5].count), (54, 8));
    alice
        .close_container()
        .await
        .expect("the server confirms a close");

    // Broken by hand, the chest leaves its sticks and itself.
    alice
        .break_block(CHEST)
        .await
        .expect("a chest breaks by hand");
    let mut left = [
        packet::<AddItemEntity>(&mut bob).await.item.count,
        packet::<AddItemEntity>(&mut bob).await.item.count,
    ];
    left.sort_unstable();
    assert_eq!(left, [1, 8]);

    for bot in [&mut alice, &mut bob] {
        assert_eq!(bot.disconnect_reason(), None);
        bot.disconnect().await;
    }
    server.shutdown().await;
}
