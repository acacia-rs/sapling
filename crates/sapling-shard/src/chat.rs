//! Chat: a client's line is cleaned, shown to the handlers and sent to everyone in the world
//! under the sender's verified name, whatever name the packet claims.

use std::collections::HashMap;

use acacia_proto::packets::{
    Text, TextCategory, TextContent, TextContentAnnouncement, TextContentChat, TextContentJson, TextContentJsonWhisper, TextContentRaw, TextContentTranslation, TextContentWhisper, TextType,
};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{ChatEvent, Player, PlayerId};

use crate::components::{Conn, InLayer, Net, Outgoing, PacketQueues, PerTick, in_world, in_world_identity};
use crate::plugins;
use crate::worlds;

/// The vanilla chat box takes 512 characters.
const MAX_CHARS: usize = 512;
/// Lines one player may send in a tick; more are dropped.
const MAX_LINES_PER_TICK: usize = 3;
const FORMAT: char = '§';
/// Java's spam rule, BDS's not being known: a line adds 20, a tick takes 1 away, and a player
/// over 200 (a line a second, or ten at once) is kicked.
const SPAM_PER_LINE: u32 = 20;
const SPAM_LIMIT: u32 = 200;
const SPAM: &str = "disconnect.spam";

/// Each player's spam count, by [`SPAM_PER_LINE`].
#[derive(Resource, Default)]
struct Spam(HashMap<Entity, u32>);

/// Counts a line of `from`'s; false once it is over the limit, when it is also kicked.
fn count_spam(world: &mut World, from: Entity) -> bool {
    let mut spam = world.resource_mut::<Spam>();
    let count = spam.0.entry(from).or_default();
    let (was, now) = (*count, count.saturating_add(SPAM_PER_LINE));
    *count = now;
    if now <= SPAM_LIMIT {
        return true;
    }
    if was <= SPAM_LIMIT && let Some(conn) = world.get::<Conn>(from).map(|conn| conn.0) {
        world.resource::<Net>().0.kick(conn, SPAM);
    }
    false
}

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

/// A line of the client's own text (`{"rawtext":[...]}`), in each player's language. BDS ends
/// the JSON with a newline.
pub(crate) fn json_packet(parts: Vec<serde_json::Value>) -> Bytes {
    let message = format!("{}\n", serde_json::json!({ "rawtext": parts }));
    text(TextType::Json, TextCategory::MessageOnly, TextContent::Json(TextContentJson { message }), "")
}

/// The line of translation key `key` with `parameters`, for the client to put into its own
/// language: how BDS says who died of what (research/captures/bds-duel.txt).
pub(crate) fn translated_packet(key: &str, parameters: &[String]) -> Bytes {
    let content = TextContent::Translation(TextContentTranslation { message: key.to_owned(), parameters: parameters.to_vec() });
    let packet = Text { needs_translation: true, category: TextCategory::Parameters, r#type: TextType::Translation, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None };
    sapling_net::encode(&packet)
}

/// The part of a [`json_packet`] that is translation key `key` with `parameters`; BDS leaves
/// `with` null when there are none.
pub(crate) fn translation(key: &str, parameters: &[String]) -> serde_json::Value {
    let with = (!parameters.is_empty()).then(|| parameters.iter().map(|p| serde_json::json!({ "text": p })).collect::<Vec<_>>());
    serde_json::json!({ "translate": key, "with": { "rawtext": with } })
}

/// To everyone in a world of this shard.
pub(crate) fn send_to_all(world: &mut World, packet: &Bytes) {
    for player in in_world(world) {
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.0.push(packet.clone());
        }
    }
}

/// To everyone in one world.
pub(crate) fn send_to_layer(world: &mut World, layer: Entity, packet: &Bytes) {
    for player in in_world(world) {
        if world.get::<InLayer>(player).is_some_and(|in_layer| in_layer.0 == layer) {
            world.get_mut::<Outgoing>(player).expect("players have an outbox").0.push(packet.clone());
        }
    }
}

/// `/say`'s and `/tell`'s lines as BDS sent them (research/captures/bds-feedback.txt).
impl sapling_api::ChatBackend for crate::handlers::Ctx<'_> {
    fn announce(&mut self, from: &str, message: &str) {
        let content = TextContentAnnouncement { source_name: from.to_owned(), message: format!("[{from}] {message}") };
        send_to_all(self.world, &text(TextType::Announcement, TextCategory::Authored, TextContent::Announcement(content), ""));
    }

    fn whisper(&mut self, to: PlayerId, from: &str, message: &str) {
        let content = TextContentWhisper { source_name: from.to_owned(), message: message.to_owned() };
        let packet = text(TextType::Whisper, TextCategory::Authored, TextContent::Whisper(content), "");
        if let Some(mut out) = self.out(to) {
            out.0.push(packet);
        }
    }

    /// `/me`'s line is a Chat with no sender (research/captures/bds-messaging.txt).
    fn emote(&mut self, from: &str, message: &str) {
        let content = TextContentChat { source_name: String::new(), message: format!("* {from} {message}") };
        send_to_all(self.world, &text(TextType::Chat, TextCategory::Authored, TextContent::Chat(content), ""));
    }

    fn tell_raw(&mut self, to: PlayerId, json: &str) {
        let content = TextContent::JsonWhisper(TextContentJsonWhisper { message: json.to_owned() });
        let packet = text(TextType::JsonWhisper, TextCategory::MessageOnly, content, "");
        if let Some(mut out) = self.out(to) {
            out.0.push(packet);
        }
    }
}

/// Runs this tick's chat lines through the handlers and sends what they let through.
pub(crate) fn dispatch(world: &mut World) {
    world.get_resource_or_insert_with(Spam::default).0.retain(|_, count| {
        *count -= 1;
        *count > 0
    });
    let lines = world.resource_mut::<PacketQueues>().take::<Text>();
    let mut limit = PerTick::new(MAX_LINES_PER_TICK);
    for (entity, packet) in lines {
        let TextContent::Chat(chat) = packet.content else { continue };
        let Some(from) = in_world_identity(world, entity) else { continue };
        let Some(message) = clean(&chat.message) else { continue };
        if !limit.allow(entity) || !count_spam(world, entity) {
            continue;
        }
        worlds::focus_on(world, entity);
        let event = plugins::offer(world,ChatEvent::new(from.clone(), message));
        // A handler may leave anything in the message; what goes out is clean again.
        let Some(message) = clean(&event.message).filter(|_| !event.is_cancelled()) else { continue };
        // The line stays in the sender's world, wherever a handler has moved them by now.
        if let Some(layer) = world.get::<InLayer>(entity).map(|in_layer| in_layer.0) {
            send_to_layer(world, layer, &chat_packet(&from, message));
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
