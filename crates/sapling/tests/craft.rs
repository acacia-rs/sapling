//! Crafting on a real connection: a bot turns logs into planks and sticks in its own grid, by the
//! recipe book and by hand, then a pickaxe at a crafting table it placed.

mod common;

use acacia_bot::interact::Face;
use acacia_bot::workstation::CraftMode;
use acacia_bot::{Bot, BotConfig};
use sapling::{Block, Handler, Item, Player, Server, ShardCtx, Slot};

/// A ground block two east of the spawn, which the server makes dirt with air above it.
const BASE: [i32; 3] = [common::SPAWN.0 + 2, common::GROUND, common::SPAWN.1];
const TABLE: [i32; 3] = [BASE[0], BASE[1] + 1, BASE[2]];
const BESIDE: [i32; 3] = [BASE[0], BASE[1], BASE[2] + 1];
const CUTTER: [i32; 3] = [BESIDE[0], BESIDE[1] + 1, BESIDE[2]];

/// Gives Alice two logs and a crafting table.
struct Kit;

impl Handler for Kit {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        for [x, y, z] in [BASE, BESIDE] {
            assert!(ctx.set_block([x, y, z], &Block::new("dirt")));
            (1..=2).for_each(|up| assert!(ctx.set_block([x, y + up, z], &Block::air())));
        }
        let kit = [("minecraft:oak_log", 2), ("minecraft:crafting_table", 1), ("minecraft:stonecutter_block", 1), ("minecraft:stone", 4)];
        for (slot, (item, count)) in kit.into_iter().enumerate() {
            assert!(ctx.set_item(player.id, Slot::Main(slot as u8), Some(&Item::new(item, count))));
        }
    }
}

/// How many of an item the bot's own copy of its inventory holds.
fn held(bot: &Bot, name: &str) -> u32 {
    let state = bot.state();
    state.inventory.main.iter().filter(|stack| !stack.is_empty() && state.items.name(stack.network_id) == Some(name)).map(|stack| u32::from(stack.count)).sum()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_crafts_in_its_own_grid_and_at_a_table() {
    let server = common::start(Server::builder().handler(Kit)).await;
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    alice.wait_ticks(20).await.expect("connected");

    assert_eq!(alice.craft("minecraft:oak_planks", 8, None).await.expect("planks from logs"), 8);
    assert_eq!(alice.craft("minecraft:stick", 4, None).await.expect("sticks by the recipe book"), 4);
    assert_eq!((held(&alice, "minecraft:oak_log"), held(&alice, "minecraft:oak_planks"), held(&alice, "minecraft:stick")), (0, 6, 4));
    let pickaxe = alice.craft("minecraft:wooden_pickaxe", 1, None).await;
    assert!(pickaxe.is_err(), "a pickaxe does not fit the 2x2 grid: {pickaxe:?}");

    alice.select_hotbar(1).expect("a hotbar slot");
    alice.place_block(BASE, Face::Up).await.expect("the hand holds a crafting table");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(alice.craft("minecraft:wooden_pickaxe", 1, Some(TABLE)).await.expect("a pickaxe at the table"), 1);
    assert_eq!(alice.craft_with("minecraft:stick", 4, Some(TABLE), CraftMode::Grid).await.expect("sticks laid out by hand"), 4);
    assert_eq!((held(&alice, "minecraft:oak_planks"), held(&alice, "minecraft:stick"), held(&alice, "minecraft:wooden_pickaxe")), (1, 6, 1));

    alice.select_hotbar(2).expect("a hotbar slot");
    alice.place_block(BESIDE, Face::Up).await.expect("the hand holds a stonecutter");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(alice.stonecut(CUTTER, "minecraft:stone_bricks", 2).await.expect("stone cut to bricks"), 2);
    assert_eq!((held(&alice, "minecraft:stone"), held(&alice, "minecraft:stone_bricks")), (2, 2));

    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}
