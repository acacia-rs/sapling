//! Running a shard: idle kicks, the chat spam kick, and a panicking shard saving and reporting.

use crate::common;

use std::time::{Duration, Instant};

use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentChat, TextType};
use acacia_proto::types::DisconnectFailReason;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config, input_at, platform};
use sapling_net::Outbound;
use sapling_shard::{ShardConfig, TICK};
use sapling_world::{World, WorldStorage};

fn line(message: &str) -> Text {
    let content = TextContent::Chat(TextContentChat { source_name: "Steve".into(), message: message.into() });
    Text { needs_translation: false, category: TextCategory::Authored, r#type: TextType::Chat, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

/// Ticks `n` times; returns the chat lines conn 1 was sent and every kick, by connection.
fn run(h: &mut Harness, n: usize) -> (usize, Vec<(u64, DisconnectFailReason, String)>) {
    let (mut lines, mut kicks) = (0, Vec::new());
    for _ in 0..n {
        h.shard.tick();
        while let Ok(command) = h.net.sent.try_recv() {
            match command {
                Outbound::Send { conn, packets } if conn.get() == 1 => {
                    let packets: Vec<_> = packets.into_iter().map(|p| acacia_proto::RawPacket::parse(p).unwrap()).collect();
                    lines += all::<Text>(&packets).iter().filter(|text| text.r#type == TextType::Chat).count();
                }
                Outbound::Kick { conn, reason, message } => kicks.push((conn.get(), reason, message)),
                _ => {}
            }
        }
    }
    (lines, kicks)
}

#[test]
fn a_player_who_does_nothing_is_kicked_after_the_idle_timeout() {
    // Alex's clock starts as Steve joins, about ten ticks before the first run.
    let mut h = Harness::with(ShardConfig { idle_timeout: Some(TICK * 30), ..config() }, platform());
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    assert!(run(&mut h, 10).1.is_empty());
    h.receive(1, &input_at([3.5, 65.0, -2.5]));
    h.receive(2, &line("still here"));
    assert!(run(&mut h, 25).1.is_empty(), "moving and chatting are doing something");
    let (_, kicks) = run(&mut h, 10);
    let idle = (DisconnectFailReason::KickedForIdle, "disconnectionScreen.body.idleTooLong".to_owned());
    let mut kicked: Vec<_> = kicks.into_iter().map(|(conn, reason, message)| (conn, (reason, message))).collect();
    kicked.sort_by_key(|(conn, _)| *conn);
    assert_eq!(kicked, [(1, idle.clone()), (2, idle)]);
    assert!(run(&mut h, 30).1.is_empty(), "kicked once");

    let mut never = Harness::new();
    never.enter(1, "Alex");
    assert!(run(&mut never, 100).1.is_empty(), "no timeout by default");
}

#[test]
fn a_chat_flood_is_kicked_for_spam_and_a_steady_talker_is_not() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // A line a second for a minute.
    for n in 0..60 {
        h.receive(2, &line(&format!("line {n}")));
        let (lines, kicks) = run(&mut h, 20);
        assert_eq!((lines, kicks.len()), (1, 0));
    }
    // Three a tick: ten get through, the eleventh is over the limit.
    let mut seen = 0;
    let mut kicks = Vec::new();
    for _ in 0..5 {
        (0..3).for_each(|n| h.receive(1, &line(&format!("flood {n}"))));
        let (lines, kicked) = run(&mut h, 1);
        (seen, kicks) = (seen + lines, [kicks, kicked].concat());
    }
    assert_eq!(seen, 10);
    assert_eq!(kicks, [(1, DisconnectFailReason::Kicked, "disconnect.spam".to_owned())]);
}

#[test]
fn a_shard_that_panics_saves_its_players_and_writes_a_crash_report() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let reports = std::env::temp_dir().join(format!("sapling-crash-{}-{stamp}", std::process::id()));
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(ShardConfig { crash_reports: Some(reports.clone()), ..config() }, World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let Harness { shard, mut net, remote, .. } = h;
    remote.run(|_| panic!("a plugin's bug"));
    let running = shard.spawn();
    let deadline = Instant::now() + Duration::from_secs(20);
    let report = loop {
        assert!(Instant::now() < deadline, "no crash report in {}", reports.display());
        let written = std::fs::read_dir(&reports).into_iter().flatten().flatten().map(|file| file.path()).find(|path| path.extension().is_some_and(|ext| ext == "txt"));
        if let Some(path) = written {
            break std::fs::read_to_string(path).unwrap();
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    running.stop();
    for line in ["message: a plugin's bug", "saved: yes, and its players were disconnected", "  default (dimension 0, "] {
        assert!(report.lines().any(|l| l.starts_with(line)), "{line:?} missing from\n{report}");
    }
    assert!(report.contains("phase Handlers") && report.contains("backtrace:"), "{report}");
    let kicked = std::iter::from_fn(|| net.sent.try_recv().ok()).any(|sent| matches!(sent, Outbound::Kick { conn, ref message, .. } if conn.get() == 1 && message == "disconnect.closed"));
    assert!(kicked, "the player was told the shard closed");
    assert!(storage.load_player(&acacia_auth::login::offline_identity("Steve")).unwrap().is_some(), "the player was saved");
    let _ = std::fs::remove_dir_all(&reports);
}
