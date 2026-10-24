//! Chat lines told as someone's: an announcement to everyone and a whisper to one player, as
//! `/say` and `/tell` send them.

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
}

/// The chat side of [`crate::Backend`].
pub trait ChatBackend {
    fn announce(&mut self, from: &str, message: &str);
    fn whisper(&mut self, to: PlayerId, from: &str, message: &str);
}

#[cfg(test)]
impl ChatBackend for crate::testing::Fake {
    fn announce(&mut self, from: &str, message: &str) {
        self.log.push(format!("announce {from} {message}"));
    }
    fn whisper(&mut self, to: PlayerId, from: &str, message: &str) {
        self.log.push(format!("whisper {} {from} {message}", to.0));
    }
}
