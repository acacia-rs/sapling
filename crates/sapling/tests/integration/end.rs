//! A bot goes to the End and back by `/execute in`, and fills a ring of end portal frames and
//! walks into the portal it makes, arriving on the obsidian platform (research/captures/bds-end-*.txt).

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{ChangeDimension, CommandOutput, LevelChunk, MovePlayer, PlayerAction};
use acacia_bot::{Bot, BotConfig};
use common::{GROUND, SPAWN};
use sapling::{END, Server, World};
use travel::{Operators, arrive, block, run};

/// A void End with end stone round the arrival place, its top at y 48.
fn end() -> World {
    let mut end = World::void_in(END);
    let end_stone = end.block_id("minecraft:end_stone").unwrap();
    end.fill([88, 40, -12], [112, 48, 12], end_stone);
    end
}

async fn bot(server: &Server) -> Bot {
    let subscribed = PacketFilter::none().with(MovePlayer::ID).with(ChangeDimension::ID).with(PlayerAction::ID).with(CommandOutput::ID).with(LevelChunk::ID);
    let mut alice = common::spawn(common::client(server, "Alice"), BotConfig { subscribe: subscribed, ..BotConfig::default() }).await;
    alice.wait_ticks(20).await.expect("connected");
    alice
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_goes_to_the_end_by_command_and_back() {
    let server = common::start(Server::builder().end(end()).handler(Operators).vanilla_commands()).await;
    let mut alice = bot(&server).await;
    alice.client().command("execute in the_end run tp @s 100.5 49 0.5");
    let (change, placed) = arrive(&mut alice, END).await;
    assert_eq!(([change.position.x, change.position.y, change.position.z], change.loading_screen_id), ([100.5, 49.0, 0.5], Some(0)));
    assert_eq!((placed, block(&alice, [100, 48, 0])), ([100, 49, 0], ("minecraft:end_stone", END)));
    assert_eq!(block(&alice, [100, 49, 0]).0, "minecraft:air", "no platform for a command's trip");

    alice.client().command(&format!("execute in overworld run tp @s {}.5 {} {}.5", SPAWN.0, GROUND + 1, SPAWN.1));
    let (back, placed) = arrive(&mut alice, 0).await;
    assert_eq!((back.loading_screen_id, placed), (Some(1), [SPAWN.0, GROUND + 1, SPAWN.1]));
    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_fills_a_ring_of_frames_and_walks_into_the_portal_onto_the_platform() {
    let server = common::start(Server::builder().end(end()).handler(Operators).vanilla_commands()).await;
    let mut alice = bot(&server).await;
    // The ring round the 3 x 3 from (x, y, z), one eye short, and the bot south of it facing in.
    let (x, y, z) = (SPAWN.0 + 3, GROUND + 1, SPAWN.1 + 3);
    let mut lines = vec![format!("fill {} {y} {} {} {} {} air", x - 1, z - 1, x + 3, y + 2, z + 6), "gamemode creative".into()];
    let sides: [(i32, i32, i32, i32, &str); 4] = [(0, -1, 1, 0, "south"), (0, 3, 1, 0, "north"), (-1, 0, 0, 1, "east"), (3, 0, 0, 1, "west")];
    for (ox, oz, dx, dz, facing) in sides {
        for i in 0..3 {
            let eye = !(oz == 3 && i == 1);
            lines.push(format!("setblock {} {y} {} end_portal_frame [\"minecraft:cardinal_direction\"=\"{facing}\",\"end_portal_eye_bit\"={eye}]", x + ox + dx * i, z + oz + dz * i));
        }
    }
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 ender_eye".into(), format!("tp @s {}.5 {y} {}.5", x + 1, z + 5)]);
    for line in &lines {
        assert!(run(&mut alice, line).await.1, "{line}");
    }
    alice.wait_ticks(10).await.expect("connected");
    alice.use_item_on_block([x + 1, y, z + 3], acacia_bot::interact::Face::Up).await.expect("the eye goes in");
    alice.wait_ticks(5).await.expect("connected");
    assert_eq!(block(&alice, [x + 1, y, z + 1]).0, "minecraft:end_portal");

    // North, over the filled frame, into the portal.
    let controls = alice.controls().expect("physics");
    (controls.forward, controls.yaw, controls.jump) = (1.0, 180.0, true);
    let (change, placed) = arrive(&mut alice, END).await;
    alice.controls().expect("physics").stop();
    assert_eq!([change.position.x, change.position.y, change.position.z], [100.0, 50.0, 0.0]);
    assert_eq!(placed, [100, 50, 0]);
    alice.wait_ticks(10).await.expect("connected");
    assert_eq!((block(&alice, [100, 49, 0]), block(&alice, [98, 49, -2]).0, block(&alice, [100, 51, 0]).0), (("minecraft:obsidian", END), "minecraft:obsidian", "minecraft:air"));
    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}
