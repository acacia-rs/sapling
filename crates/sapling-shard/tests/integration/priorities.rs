//! Handler order, cancelling and monitors (harness: common/mod.rs).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{CommandOutput, CommandRequest, Text, TextCategory, TextContent, TextContentChat, TextType};
use acacia_proto::types::CommandOrigin;
use common::{Harness, all};
use sapling_api::{ChatEvent, CommandEvent, Handler, Player, Priority, ShardCtx};

type Log = Arc<Mutex<Vec<String>>>;

/// Logs what it is shown; cancels lines containing `cancels`, and appends its name to chat.
struct Named {
    name: &'static str,
    cancels: &'static str,
    log: Log,
}

impl Named {
    fn note(&self, what: String) {
        self.log.lock().unwrap().push(format!("{} {what}", self.name));
    }
}

impl Handler for Named {
    fn on_join(&mut self, _: &mut ShardCtx<'_>, player: &Player) {
        self.note(format!("join {}", player.name));
    }

    fn on_chat(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut ChatEvent) {
        self.note(format!("chat {:?}{}", event.message, if event.is_cancelled() { " cancelled" } else { "" }));
        if event.message.contains(self.cancels) {
            event.cancel();
        }
        event.message.push_str(&format!(" +{}", self.name));
    }

    fn on_command(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut CommandEvent) {
        self.note(format!("command {:?} ({}){}", event.line(), event.name(), if event.is_cancelled() { " cancelled" } else { "" }));
        if event.line().contains(self.cancels) {
            event.cancel();
        }
    }
}

const NEVER: &str = "\u{0}";

/// Handlers added out of order: a monitor, then low to high, with two at normal priority.
fn harness() -> (Harness, Log) {
    let mut h = Harness::new();
    let log = Log::default();
    let handlers = [
        (Priority::Monitor, "monitor", "bye"),
        (Priority::Low, "low", NEVER),
        (Priority::Normal, "first", NEVER),
        (Priority::Highest, "highest", NEVER),
        (Priority::Normal, "second", NEVER),
        (Priority::Lowest, "lowest", NEVER),
        (Priority::High, "high", "stop"),
    ];
    for (priority, name, cancels) in handlers {
        h.shard.add_handler_at(priority, Box::new(Named { name, cancels, log: log.clone() }));
    }
    h.enter(1, "Alex");
    (h, log)
}

fn taken(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
}

fn line(message: &str) -> Text {
    Text {
        needs_translation: false,
        category: TextCategory::Authored,
        r#type: TextType::Chat,
        content: TextContent::Chat(TextContentChat { source_name: "Alex".into(), message: message.into() }),
        xuid: String::new(),
        platform_chat_id: String::new(),
        has_filtered_message: false,
        filtered_message: None,
    }
}

fn command(line: &str) -> CommandRequest {
    let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid::default(), request_id: String::new(), player_entity_id: 0 };
    CommandRequest { command: line.into(), origin, internal: false, version: "latest".into() }
}

fn chat(packets: &[Text]) -> Vec<&str> {
    packets.iter().filter_map(|t| if let TextContent::Chat(c) = &t.content { Some(c.message.as_str()) } else { None }).collect()
}

#[test]
fn handlers_run_from_highest_to_lowest_then_monitors() {
    let (_, log) = harness();
    let order = ["highest", "high", "first", "second", "low", "lowest", "monitor"];
    assert_eq!(taken(&log), order.map(|name| format!("{name} join Alex")));
}

#[test]
fn a_cancel_stops_later_handlers_but_not_monitors() {
    let (mut h, log) = harness();
    taken(&log);
    h.receive(1, &line("stop it"));
    assert!(chat(&all::<Text>(&h.tick(1))).is_empty());
    assert_eq!(taken(&log), [r#"highest chat "stop it""#, r#"high chat "stop it +highest""#, r#"monitor chat "stop it +highest +high" cancelled"#]);

    // Uncancelled, every handler sees what the ones before it made of the line.
    h.receive(1, &line("hello"));
    assert_eq!(chat(&all::<Text>(&h.tick(1))), ["hello +highest +high +first +second +low +lowest"]);
    let seen = taken(&log);
    assert_eq!(seen.len(), 7);
    assert_eq!(seen[6], r#"monitor chat "hello +highest +high +first +second +low +lowest""#);
}

#[test]
fn a_monitor_changes_nothing() {
    let (mut h, log) = harness();
    taken(&log);
    // The monitor cancels "bye" and appends its name; neither reaches the outcome.
    h.receive(1, &line("bye"));
    assert_eq!(chat(&all::<Text>(&h.tick(1))), ["bye +highest +high +first +second +low +lowest"]);
    assert_eq!(taken(&log).len(), 7);
}

#[test]
fn commands_pass_the_handlers_before_they_run() {
    let (mut h, log) = harness();
    taken(&log);
    h.receive(1, &command("/stop now"));
    assert!(all::<CommandOutput>(&h.tick(1)).is_empty());
    assert_eq!(taken(&log), [r#"highest command "stop now" (stop)"#, r#"high command "stop now" (stop)"#, r#"monitor command "stop now" (stop) cancelled"#]);

    h.receive(1, &command("/bye"));
    let outputs = all::<CommandOutput>(&h.tick(1));
    assert_eq!((outputs.len(), outputs[0].output[0].message_id.as_str()), (1, "commands.generic.unknown"));
    assert_eq!(taken(&log).len(), 7);
}
