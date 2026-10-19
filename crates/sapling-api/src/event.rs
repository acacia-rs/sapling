//! Listener order and the events of the game; block and item events are in block.rs and item.rs.

use crate::{Event, Player};

/// When a listener runs among the others: `Highest` first, `Lowest` last, `Monitor` after all of
/// them. Within a priority, listeners run in the order they were added.
///
/// For an event that can be cancelled, a cancel stops the listeners that have not run yet, except
/// `Monitor` ones and those added [`Priority::receiving_cancelled`]: `Monitor` ones always see the
/// event as it ended, and what they do to it is discarded. An event that starts cancelled (a break
/// in adventure mode) was cancelled by no listener, so all of them see it and any may `allow()` it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    Highest,
    High,
    #[default]
    Normal,
    Low,
    Lowest,
    Monitor,
}

/// The player's client has loaded the world and left its loading screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinEvent {
    player: Player,
}

/// A player who joined has left the server, for any reason. They are already gone from the shard:
/// the ctx no longer counts them and cannot reach them. It comes from the shard they were on last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuitEvent {
    player: Player,
}

/// A player was moved from one world to another ([`crate::ShardCtx::transfer`], or a world that
/// was removed). It comes from the shard of `to`, with the player already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldChangeEvent {
    player: Player,
    from: String,
    to: String,
}

/// A transfer [`crate::ShardCtx::transfer`] accepted could not be made: the shard of world `to`
/// refused or did not answer. The player is where they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFailedEvent {
    player: Player,
    to: String,
}

/// Once per tick, after the events and tasks of the tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickEvent;

impl Event for TickEvent {}

/// A chat line on its way to everyone. Listeners may rewrite [`ChatEvent::message`] or cancel it;
/// one that formats chat itself cancels and uses [`crate::ShardCtx::broadcast`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatEvent {
    player: Player,
    /// Already cleaned of control characters and cut to length.
    pub message: String,
    cancelled: bool,
}

/// A command line a player sent, registered or not, before it is looked up and run. Cancelled,
/// it is not run and gets no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEvent {
    player: Player,
    line: String,
    cancelled: bool,
}

impl JoinEvent {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

impl QuitEvent {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

impl WorldChangeEvent {
    pub fn new(player: Player, from: String, to: String) -> Self {
        Self { player, from, to }
    }

    /// The world they left; it may be on another shard.
    pub fn from(&self) -> &str {
        &self.from
    }

    pub fn to(&self) -> &str {
        &self.to
    }
}

impl TransferFailedEvent {
    pub fn new(player: Player, to: String) -> Self {
        Self { player, to }
    }

    /// The world they were to be moved to.
    pub fn to(&self) -> &str {
        &self.to
    }
}

impl ChatEvent {
    pub fn new(player: Player, message: String) -> Self {
        Self { player, message, cancelled: false }
    }
}

impl CommandEvent {
    pub fn new(player: Player, line: String) -> Self {
        Self { player, line, cancelled: false }
    }

    /// The line without its leading `/`.
    pub fn line(&self) -> &str {
        &self.line
    }

    /// The command's name: the first word of the line.
    pub fn name(&self) -> &str {
        self.line.split_whitespace().next().unwrap_or("")
    }
}

/// `player()` for events about a player, and [`Event`] for those that cannot be stopped.
macro_rules! player_event {
    ($($event:ty),*) => {$(
        impl $event {
            /// The player it is about.
            pub fn player(&self) -> &Player {
                &self.player
            }
        }
    )*};
}
player_event!(JoinEvent, QuitEvent, WorldChangeEvent, TransferFailedEvent, ChatEvent, CommandEvent, crate::BreakEvent, crate::PlaceEvent, crate::InteractEvent, crate::DropEvent);

macro_rules! notice {
    ($($event:ty),*) => {$(
        impl Event for $event {
            fn player(&self) -> Option<&Player> {
                Some(&self.player)
            }
        }
    )*};
}
notice!(JoinEvent, QuitEvent, WorldChangeEvent, TransferFailedEvent);

macro_rules! cancellable {
    ($($event:ty),*) => {$(
        impl $event {
            /// Stops the event: nothing happens, and only `Monitor` listeners still see it.
            pub fn cancel(&mut self) {
                self.cancelled = true;
            }

            pub fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }

        impl Event for $event {
            fn player(&self) -> Option<&Player> {
                Some(&self.player)
            }

            fn is_stopped(&self) -> bool {
                self.cancelled
            }
        }
    )*};
}
cancellable!(ChatEvent, CommandEvent, crate::DropEvent);
