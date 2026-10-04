//! Chat: a client's line is cleaned, shown to the handlers and sent to everyone in the world
//! under the sender's verified name, whatever name the packet claims.

use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentChat, TextContentRaw, TextType};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{ChatOutcome, Handler, Player, ShardCtx};

use crate::components::{Identity, Outgoing, PacketQueues, Stage, in_world};
use crate::handlers::Ctx;

/// The vanilla chat box takes 512 characters.
const MAX_CHARS: usize = 512;
/// Lines one player may send in a tick; more are dropped.
const MAX_LINES_PER_TICK: usize = 3;
const FORMAT: char = '§';

/// A chat line without control characters and formatting codes, cut to length; `None` if nothing is left.
fn clean(message: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut chars = message.chars();
    while let Some(c) = chars.next() {
        if c == FORMAT {
            chars.next();
        } else if !c.is_control() {
            cleaned.push(c);
        }
    }
    let cut: String = cleaned.trim().chars().take(MAX_CHARS).collect();
    (!cut.is_empty()).then_some(cut)
}

fn text(r#type: TextType, category: TextCategory, content: TextContent, xuid: &str) -> Bytes {
    let packet = Text { needs_translation: false, category, r#type, content, xuid: xuid.to_owned(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None };
    sapling_net::encode(&packet)
}

/// A player's line, as BDS relays one: the name ends with a format reset.
fn chat_packet(from: &Player, message: String) -> Bytes {
    let content = TextContentChat { source_name: format!("{}{FORMAT}r", from.name), message };
    text(TextType::Chat, TextCategory::Authored, TextContent::Chat(content), &from.xuid)
}

/// A line without a sender.
pub(crate) fn raw_packet(message: &str) -> Bytes {
    text(TextType::Raw, TextCategory::MessageOnly, TextContent::Raw(TextContentRaw { message: message.to_owned() }), "")
}

pub(crate) fn send_to_all(world: &mut World, packet: &Bytes) {
    for player in in_world(world) {
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.0.push(packet.clone());
        }
    }
}

/// What the handlers make of `message`: the text to send, or `None` once one cancels.
fn decide(world: &mut World, handlers: &mut [Box<dyn Handler>], from: &Player, mut message: String) -> Option<String> {
    for handler in handlers.iter_mut() {
        match handler.on_chat(&mut ShardCtx::new(&mut Ctx { world }), from, &message) {
            ChatOutcome::Allow => {}
            ChatOutcome::Cancel => return None,
            ChatOutcome::Replace(replacement) => message = replacement,
        }
    }
    Some(message)
}

/// Runs this tick's chat lines through the handlers and sends what they let through.
pub(crate) fn dispatch(world: &mut World, handlers: &mut [Box<dyn Handler>]) {
    let lines = world.resource_mut::<PacketQueues>().take::<Text>();
    let mut sent: Vec<Entity> = Vec::new();
    for (entity, packet) in lines {
        let TextContent::Chat(chat) = packet.content else { continue };
        let Some(from) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        let in_world = world.get::<Stage>(entity) == Some(&Stage::InWorld);
        let Some(message) = clean(&chat.message).filter(|_| in_world) else { continue };
        sent.push(entity);
        if sent.iter().filter(|e| **e == entity).count() > MAX_LINES_PER_TICK {
            continue;
        }
        if let Some(message) = decide(world, handlers, &from, message) {
            send_to_all(world, &chat_packet(&from, message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_lose_control_characters_formatting_and_excess_length() {
        assert_eq!(clean("  hi\tthere\u{0}\n ").as_deref(), Some("hithere"));
        assert_eq!(clean("§cred §lbold").as_deref(), Some("red bold"));
        assert_eq!((clean(" \n\t"), clean("§a"), clean("")), (None, None, None));
        assert_eq!(clean(&"é".repeat(600)).unwrap().chars().count(), 512);
        assert_eq!(clean("grüße, 世界 :)").as_deref(), Some("grüße, 世界 :)"));
    }
}
