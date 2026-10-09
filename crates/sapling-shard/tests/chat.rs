//! Chat and the handler-side view of players (harness: common/mod.rs).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    Text, TextCategory, TextContent, TextContentChat, TextContentRaw, TextType,
};
use common::{Harness, all, ids, input_at};
use sapling_api::{ChatEvent, Handler, Player, ShardCtx};

/// A chat line as a client sends it, claiming to be `claimed`.
fn line(claimed: &str, message: &str) -> Text {
    Text {
        needs_translation: false,
        category: TextCategory::Authored,
        r#type: TextType::Chat,
        content: TextContent::Chat(TextContentChat {
            source_name: claimed.into(),
            message: message.into(),
        }),
        xuid: "2535400000000000".into(),
        platform_chat_id: String::new(),
        has_filtered_message: false,
        filtered_message: None,
    }
}

fn chat(packet: &Text) -> (&str, &str) {
    match &packet.content {
        TextContent::Chat(chat) => (chat.source_name.as_str(), chat.message.as_str()),
        other => panic!("not chat: {other:?}"),
    }
}

fn raw(packet: &Text) -> &str {
    match &packet.content {
        TextContent::Raw(raw) => raw.message.as_str(),
        other => panic!("not raw: {other:?}"),
    }
}

struct Moderator;

impl Handler for Moderator {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.broadcast(&format!("{} joined", player.name));
    }

    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
        match event.message.as_str() {
            "ping" => {
                ctx.send_message(player.id, "pong");
                event.message = "PING".into();
            }
            "where" => {
                let names: Vec<String> = ctx.players().map(|p| p.name).collect();
                let at = ctx.position(player.id).map(|p| (p.feet, p.yaw));
                event.message = format!("{names:?} {at:?}");
            }
            "colour" => event.message = "§cred\n".into(),
            secret if secret.contains("secret") => event.cancel(),
            _ => {}
        }
    }
}

/// Runs after [`Moderator`]: sees what it let through.
struct Shout;

impl Handler for Shout {
    fn on_chat(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut ChatEvent) {
        assert!(
            !event.message.contains("secret"),
            "a cancelled line reached the next handler"
        );
        if event.message == "PING" {
            event.message = "PING!".into();
        }
    }
}

fn two_players() -> Harness {
    let mut h = Harness::new();
    h.shard.add_handler(Box::new(Moderator));
    h.shard.add_handler(Box::new(Shout));
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

#[test]
fn chat_reaches_everyone_under_the_verified_name() {
    let mut h = two_players();
    h.receive(1, &line("Notch", " hello §cworld\n"));
    let sent = h.tick_all();
    for conn in [1, 2] {
        let texts = all::<Text>(sent.to(conn));
        assert_eq!(
            (ids(sent.to(conn)), chat(&texts[0])),
            (vec![Text::ID], ("Alex§r", "hello world"))
        );
        assert_eq!(
            (
                texts[0].xuid.as_str(),
                texts[0].r#type,
                texts[0].category,
                texts[0].needs_translation
            ),
            ("", TextType::Chat, TextCategory::Authored, false)
        );
    }

    // Only chat is taken from clients, only from players in the world, and never empty.
    let system = Text {
        r#type: TextType::Raw,
        content: TextContent::Raw(TextContentRaw {
            message: "Server closing".into(),
        }),
        ..line("", "")
    };
    h.receive(1, &system);
    h.receive(1, &line("Alex", " §a\t"));
    h.join(3, "Early");
    h.receive(3, &line("Early", "let me in"));
    let sent = h.tick_all();
    assert!(sent.to(1).is_empty() && sent.to(2).is_empty());

    // Three lines a tick per player; the fourth is dropped.
    (1..=4).for_each(|n| h.receive(2, &line("Steve", &format!("line {n}"))));
    let texts = all::<Text>(h.tick_all().to(1));
    assert_eq!(
        texts.iter().map(|t| chat(t).1).collect::<Vec<_>>(),
        ["line 1", "line 2", "line 3"]
    );
    assert_eq!(chat(&texts[0]).0, "Steve§r");
}

#[test]
fn handlers_cancel_and_replace_chat() {
    let mut h = two_players();
    h.receive(1, &line("Alex", "the secret plan"));
    let sent = h.tick_all();
    assert!(sent.to(1).is_empty() && sent.to(2).is_empty());

    h.receive(2, &line("Steve", "ping"));
    let sent = h.tick_all();
    let (alex, steve) = (all::<Text>(sent.to(1)), all::<Text>(sent.to(2)));
    assert_eq!((alex.len(), chat(&alex[0])), (1, ("Steve§r", "PING!")));
    assert_eq!(
        (steve.len(), raw(&steve[0]), chat(&steve[1])),
        (2, "pong", ("Steve§r", "PING!"))
    );
    assert_eq!(
        (steve[0].r#type, steve[0].category),
        (TextType::Raw, TextCategory::MessageOnly)
    );

    // What a handler writes into the message is cleaned like a client's line.
    h.receive(2, &line("Steve", "colour"));
    assert_eq!(chat(&all::<Text>(h.tick_all().to(1))[0]).1, "red");
}

#[test]
fn handlers_see_the_players_and_where_they_are() {
    let mut h = Harness::new();
    h.shard.add_handler(Box::new(Moderator));
    assert_eq!(
        raw(&all::<Text>(h.enter(1, "Alex").to(1))[0]),
        "Alex joined"
    );
    let sent = h.enter(2, "Steve");
    assert_eq!(
        (
            raw(&all::<Text>(sent.to(1))[0]),
            raw(&all::<Text>(sent.to(2))[0])
        ),
        ("Steve joined", "Steve joined")
    );

    let mut input = input_at([3.5, 65.0, -2.5]);
    input.yaw = 45.0;
    h.receive(2, &input);
    h.receive(2, &line("Steve", "where"));
    let texts = all::<Text>(h.tick_all().to(1));
    assert_eq!(
        chat(&texts[0]).1,
        r#"["Alex", "Steve"] Some(([3.5, 65.0, -2.5], 45.0))"#
    );

    // A player who left is neither listed nor found.
    h.leave(2);
    h.tick_all();
    h.receive(1, &line("Alex", "where"));
    let texts = all::<Text>(h.tick_all().to(1));
    assert_eq!(
        chat(&texts[0]).1,
        r#"["Alex"] Some(([0.5, 65.0, 0.5], 0.0))"#
    );
}
