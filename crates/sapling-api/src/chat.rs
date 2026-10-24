//! Chat lines told as someone's: an announcement to everyone and a whisper to one player, as
//! `/say` and `/tell` send them; `/me`'s line; and rawtext, as `/tellraw` sends it.

use crate::command::RawText;
use crate::{PlayerId, ShardCtx};

impl ShardCtx<'_> {
    /// Shows every player on this shard `message` as said by `from`, as `/say` does: `[from] message`.
    pub fn announce(&mut self, from: &str, message: &str) {
        self.backend.announce(from, message);
    }

    /// Shows `to` alone `message` as whispered by `from`, as `/tell` does.
    pub fn whisper(&mut self, to: PlayerId, from: &str, message: &str) {
        self.backend.whisper(to, from, message);
    }

    /// Shows every player on this shard `* from message`, as `/me` does.
    pub fn emote(&mut self, from: &str, message: &str) {
        self.backend.emote(from, message);
    }

    /// Shows `to` a line of rawtext, as `/tellraw` does. Selectors and scores in it should be
    /// resolved first ([`RawText::resolve`]): the client shows what is left as written.
    pub fn tell_raw(&mut self, to: PlayerId, text: &RawText) {
        self.backend.tell_raw(to, &text.to_json());
    }
}

/// The chat side of [`crate::Backend`].
pub trait ChatBackend {
    fn announce(&mut self, from: &str, message: &str);
    fn whisper(&mut self, to: PlayerId, from: &str, message: &str);
    fn emote(&mut self, from: &str, message: &str);
    /// `json` is rawtext as BDS writes it, newline and all.
    fn tell_raw(&mut self, to: PlayerId, json: &str);
}

#[cfg(test)]
impl ChatBackend for crate::testing::Fake {
    fn announce(&mut self, from: &str, message: &str) {
        self.log.push(format!("announce {from} {message}"));
    }
    fn whisper(&mut self, to: PlayerId, from: &str, message: &str) {
        self.log.push(format!("whisper {} {from} {message}", to.0));
    }
    fn emote(&mut self, from: &str, message: &str) {
        self.log.push(format!("emote {from} {message}"));
    }
    fn tell_raw(&mut self, to: PlayerId, json: &str) {
        self.log.push(format!("tell raw {} {}", to.0, json.trim_end()));
    }
}
