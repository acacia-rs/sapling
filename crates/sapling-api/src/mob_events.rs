//! The `/mobevent` switches, kept as plugin state (`ShardCtx::state::<MobEvents>()`) so the
//! shard's spawner reads what the command set.

use std::collections::HashMap;

/// The events `/mobevent` switches, and the entity each answer names.
pub const MOB_EVENTS: [(&str, &str); 3] = [("minecraft:pillager_patrols_event", "pillager"), ("minecraft:wandering_trader_event", "wandering_trader"), ("minecraft:ender_dragon_event", "ender_dragon")];
/// The switch over every event.
pub const ALL_MOB_EVENTS: &str = "events_enabled";

/// Which switches are off; every one starts on.
#[derive(Debug, Default)]
pub struct MobEvents(HashMap<String, bool>);

impl MobEvents {
    /// The switch named `event` (one of [`MOB_EVENTS`] or [`ALL_MOB_EVENTS`]).
    pub fn is_on(&self, event: &str) -> bool {
        self.0.get(event).copied().unwrap_or(true)
    }

    pub fn set(&mut self, event: &str, on: bool) {
        self.0.insert(event.to_owned(), on);
    }

    /// Whether `event` may run: its own switch and the one over all of them on.
    pub fn runs(&self, event: &str) -> bool {
        self.is_on(ALL_MOB_EVENTS) && self.is_on(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_event_runs_while_it_and_the_master_switch_are_on() {
        let mut events = MobEvents::default();
        assert!(events.runs(MOB_EVENTS[0].0));
        events.set(ALL_MOB_EVENTS, false);
        assert!(!events.runs(MOB_EVENTS[0].0) && events.is_on(MOB_EVENTS[0].0));
        events.set(ALL_MOB_EVENTS, true);
        events.set(MOB_EVENTS[0].0, false);
        assert!(!events.runs(MOB_EVENTS[0].0) && events.runs(MOB_EVENTS[1].0));
    }
}
