//! Handler order and the events a handler can stop or change.

/// When a handler runs among the others: `Highest` first, `Lowest` last, `Monitor` after all of
/// them. Within a priority, handlers run in the order they were added.
///
/// For an event that can be cancelled, a cancel stops the handlers that have not run yet, except
/// `Monitor` ones: they always see the event as it ended, and what they do to it is discarded.
/// An event that starts cancelled (a break in adventure mode) was cancelled by no handler, so all
/// of them see it and any may `allow()` it.
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

/// A chat line on its way to everyone. Handlers may rewrite [`ChatEvent::message`] or cancel it;
/// a handler that formats chat itself cancels and uses [`crate::ShardCtx::broadcast`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatEvent {
    /// Already cleaned of control characters and cut to length.
    pub message: String,
    cancelled: bool,
}

/// A command line a player sent, before it is looked up and run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEvent {
    line: String,
    cancelled: bool,
}

impl ChatEvent {
    pub fn new(message: String) -> Self {
        Self { message, cancelled: false }
    }
}

impl CommandEvent {
    pub fn new(line: String) -> Self {
        Self { line, cancelled: false }
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

/// An event handlers can stop; what a shard needs to run one through its handlers.
pub trait Cancellable: Clone {
    fn is_cancelled(&self) -> bool;

    /// Whether a handler cancelled it, which is what skips the handlers after it. An event that
    /// only started cancelled (see [`crate::BreakEvent`]) is still shown to all of them.
    fn is_stopped(&self) -> bool {
        self.is_cancelled()
    }
}

macro_rules! cancellable {
    ($($event:ty),*) => {$(
        impl $event {
            /// Stops the event: nothing happens, and only `Monitor` handlers still see it.
            pub fn cancel(&mut self) {
                self.cancelled = true;
            }

            pub fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }

        impl Cancellable for $event {
            fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }
    )*};
}
cancellable!(ChatEvent, CommandEvent);
