use crate::PlayerId;

/// What a shard offers its handlers. Implemented by sapling-shard; plugins use [`ShardCtx`].
pub trait Backend {
    fn tick(&self) -> u64;
    fn player_count(&self) -> usize;
    fn kick(&mut self, player: PlayerId, message: &str);
}

/// A handler's access to the shard it runs on, valid only for the call it was passed to.
pub struct ShardCtx<'a> {
    backend: &'a mut dyn Backend,
}

impl<'a> ShardCtx<'a> {
    pub fn new(backend: &'a mut dyn Backend) -> Self {
        Self { backend }
    }

    /// Ticks this shard has run (20 per second).
    pub fn tick(&self) -> u64 {
        self.backend.tick()
    }

    /// Players in the world on this shard; those still loading do not count.
    pub fn player_count(&self) -> usize {
        self.backend.player_count()
    }

    /// Disconnects a player with `message` on their screen. `on_quit` follows on a later tick.
    pub fn kick(&mut self, player: PlayerId, message: &str) {
        self.backend.kick(player, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Handler, Player};

    #[derive(Default)]
    struct Fake {
        kicked: Vec<(PlayerId, String)>,
    }

    impl Backend for Fake {
        fn tick(&self) -> u64 {
            40
        }
        fn player_count(&self) -> usize {
            1
        }
        fn kick(&mut self, player: PlayerId, message: &str) {
            self.kicked.push((player, message.to_owned()));
        }
    }

    struct KickEveryone;

    impl Handler for KickEveryone {
        fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
            ctx.kick(player.id, &format!("full at tick {} with {}", ctx.tick(), ctx.player_count()));
        }
    }

    #[test]
    fn handlers_act_through_the_context_and_default_to_no_ops() {
        let mut backend = Fake::default();
        let player = Player { id: PlayerId(3), name: "Steve".into(), uuid: uuid::Uuid::nil(), xuid: String::new() };
        let mut handler: Box<dyn Handler> = Box::new(KickEveryone);
        handler.on_join(&mut ShardCtx::new(&mut backend), &player);
        handler.on_quit(&mut ShardCtx::new(&mut backend), &player);
        assert_eq!(backend.kicked, [(PlayerId(3), "full at tick 40 with 1".to_owned())]);
    }
}
