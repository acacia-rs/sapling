use crate::{Player, PlayerId, Position};

/// What a shard offers its handlers. Implemented by sapling-shard; plugins use [`ShardCtx`].
pub trait Backend {
    fn tick(&self) -> u64;
    fn player_count(&self) -> usize;
    fn players(&self) -> Vec<Player>;
    fn position(&self, player: PlayerId) -> Option<Position>;
    fn kick(&mut self, player: PlayerId, message: &str);
    fn broadcast(&mut self, message: &str);
    fn send_message(&mut self, player: PlayerId, message: &str);
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

    /// The players [`ShardCtx::player_count`] counts.
    pub fn players(&self) -> impl Iterator<Item = Player> + use<> {
        self.backend.players().into_iter()
    }

    /// Where a player in the world is; `None` once they left.
    pub fn position(&self, player: PlayerId) -> Option<Position> {
        self.backend.position(player)
    }

    /// Disconnects a player with `message` on their screen. `on_quit` follows on a later tick.
    pub fn kick(&mut self, player: PlayerId, message: &str) {
        self.backend.kick(player, message);
    }

    /// Shows `message` in the chat of every player in the world. `§` codes format it.
    pub fn broadcast(&mut self, message: &str) {
        self.backend.broadcast(message);
    }

    /// Shows `message` in one player's chat.
    pub fn send_message(&mut self, player: PlayerId, message: &str) {
        self.backend.send_message(player, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatOutcome, Handler};

    #[derive(Default)]
    struct Fake {
        kicked: Vec<(PlayerId, String)>,
        chat: Vec<(Option<PlayerId>, String)>,
    }

    fn steve() -> Player {
        Player { id: PlayerId(3), name: "Steve".into(), uuid: uuid::Uuid::nil(), xuid: String::new() }
    }

    impl Backend for Fake {
        fn tick(&self) -> u64 {
            40
        }
        fn player_count(&self) -> usize {
            1
        }
        fn players(&self) -> Vec<Player> {
            vec![steve()]
        }
        fn position(&self, player: PlayerId) -> Option<Position> {
            (player == PlayerId(3)).then_some(Position { feet: [1.0, 2.0, 3.0], yaw: 90.0, pitch: 0.0 })
        }
        fn kick(&mut self, player: PlayerId, message: &str) {
            self.kicked.push((player, message.to_owned()));
        }
        fn broadcast(&mut self, message: &str) {
            self.chat.push((None, message.to_owned()));
        }
        fn send_message(&mut self, player: PlayerId, message: &str) {
            self.chat.push((Some(player), message.to_owned()));
        }
    }

    struct KickEveryone;

    impl Handler for KickEveryone {
        fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
            ctx.kick(player.id, &format!("full at tick {} with {}", ctx.tick(), ctx.player_count()));
        }
    }

    struct Greeter;

    impl Handler for Greeter {
        fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
            let names: Vec<String> = ctx.players().map(|p| p.name).collect();
            let at = ctx.position(player.id).map(|p| p.feet);
            ctx.broadcast(&format!("{} joined {names:?} at {at:?}", player.name));
            ctx.send_message(player.id, "welcome");
        }
    }

    #[test]
    fn handlers_act_through_the_context_and_default_to_no_ops() {
        let mut backend = Fake::default();
        let mut handler: Box<dyn Handler> = Box::new(KickEveryone);
        handler.on_join(&mut ShardCtx::new(&mut backend), &steve());
        handler.on_quit(&mut ShardCtx::new(&mut backend), &steve());
        assert_eq!(handler.on_chat(&mut ShardCtx::new(&mut backend), &steve(), "hi"), ChatOutcome::Allow);
        assert_eq!(backend.kicked, [(PlayerId(3), "full at tick 40 with 1".to_owned())]);
    }

    #[test]
    fn the_context_lists_players_and_sends_messages() {
        let mut backend = Fake::default();
        Greeter.on_join(&mut ShardCtx::new(&mut backend), &steve());
        let expected = [(None, r#"Steve joined ["Steve"] at Some([1.0, 2.0, 3.0])"#.to_owned()), (Some(PlayerId(3)), "welcome".to_owned())];
        assert_eq!(backend.chat, expected);
    }
}
