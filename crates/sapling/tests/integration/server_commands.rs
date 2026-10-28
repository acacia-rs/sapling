//! `/list`, `/save`, `/permission` and `/stop` on a server whose world is a folder on disk:
//! BDS's answers (docs/research/server-leftovers.md) and what the backup hold lists.

use std::path::{Path, PathBuf};

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::{Bot, BotConfig};
use sapling::{AccessLists, World};
use travel::{Operators, next};

/// Every line of the answer to `line`: key, parameters, success.
async fn answer(bot: &mut Bot, line: &str) -> Vec<(String, Vec<String>, bool)> {
    bot.client().command(line);
    let output: CommandOutput = next(bot).await;
    output.output.into_iter().map(|line| (line.message_id, line.parameters, line.success)).collect()
}

async fn key(bot: &mut Bot, line: &str) -> (String, bool) {
    let first = answer(bot, line).await.remove(0);
    (first.0, first.2)
}

/// The fixture world copied to a folder of its own, so that saving writes there.
fn copied_fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("sapling-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    copy(&common::fixture(), &root.join("world"));
    root
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().map(Result::unwrap) {
        match entry.file_type().unwrap().is_dir() {
            true => copy(&entry.path(), &to.join(entry.file_name())),
            false => drop(std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap()),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_commands_answer_as_bds_answers() {
    let root = copied_fixture("server-commands");
    std::fs::write(root.join("permissions.json"), r#"[{ "permission": "operator", "xuid": "2535413803071674" }]"#).unwrap();
    let opened = World::open_folder(&root.join("world")).expect("the copy opens");
    let builder = common::builder().handler(Operators).vanilla_commands().access(AccessLists::open(&root).unwrap()).max_players(10);
    let server = builder.bind("127.0.0.1:0".parse().unwrap()).online_mode(false).world(opened.world, opened.spawn).start().await.expect("starts");
    let config = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config).await;
    alice.wait_ticks(5).await.expect("connected");

    let list = answer(&mut alice, "list").await;
    assert_eq!(list, [("commands.players.list".into(), vec!["1".into(), "10".into()], true), ("commands.players.list.names".into(), vec!["Alice".into()], true)]);

    assert_eq!(key(&mut alice, "save query").await, ("commands.save-on.notDone".into(), false));
    assert_eq!(key(&mut alice, "save resume").await, ("commands.save-on.notDone".into(), false));
    assert_eq!(key(&mut alice, "save hold").await, ("commands.save.start".into(), true));
    assert_eq!(key(&mut alice, "save hold").await, ("commands.generic.running".into(), false));
    let mut ready = None;
    for _ in 0..50 {
        let lines = answer(&mut alice, "save query").await;
        if lines[0].0 == "commands.save-all.success" {
            ready = Some(lines);
            break;
        }
        alice.wait_ticks(2).await.expect("connected");
    }
    let ready = ready.expect("the hold is ready in time");
    let files = &ready[1].1[0];
    assert!(files.contains("world/level.dat:") && files.contains("world/db/CURRENT:") && !files.contains("LOCK"), "{files}");
    assert_eq!(key(&mut alice, "save resume").await, ("commands.save-on.success".into(), true));
    assert_eq!(key(&mut alice, "save resume").await, ("commands.save-on.notDone".into(), false));

    let listed = key(&mut alice, "permission list").await.0;
    assert_eq!(listed, "###* {\"command\":\"ops\",\"result\":[\"2535413803071674\"]}\n *###");
    let reloaded: Vec<String> = answer(&mut alice, "ops reload").await.into_iter().map(|line| line.0).collect();
    assert_eq!(reloaded, ["commands.ops.reloaded", "commands.permissions.reloaded"]);

    assert!(!server.is_stop_requested());
    assert_eq!(key(&mut alice, "stop").await, ("commands.stop.start".into(), true));
    tokio::time::timeout(travel::WAIT, server.stop_requested()).await.expect("the stop is asked for");
    alice.disconnect().await;
    server.shutdown().await;
    let _ = std::fs::remove_dir_all(&root);
}
