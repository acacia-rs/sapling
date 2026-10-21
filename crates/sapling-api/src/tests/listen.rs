use std::sync::{Arc, Mutex};

use crate::testing::{Fake, player};
use crate::{ChatEvent, CommandEvent, Commands, Handler, JoinEvent, ListenerChange, Listeners, Player, PlayerId, PluginBuilder, Priority, Scope, ShardCtx, TickEvent};

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn push(&self, line: impl Into<String>) {
        self.0.lock().unwrap().push(line.into());
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

fn build(listeners: &mut Listeners, f: impl FnOnce(&mut PluginBuilder<'_>)) {
    f(&mut PluginBuilder::new(listeners, &mut Commands::default()));
}

#[test]
fn listeners_run_by_priority_and_a_cancel_skips_all_but_monitors_and_those_asking() {
    let (log, mut listeners) = (Log::default(), Listeners::default());
    let l = [log.clone(), log.clone(), log.clone(), log.clone(), log.clone()];
    let [low, high, lowest, monitor, command] = l;
    build(&mut listeners, |p| {
        p.on::<ChatEvent>(Priority::Low, move |_, _| low.push("low"));
        p.on::<ChatEvent>(Priority::High, move |_, ev| {
            high.push("high");
            ev.message.push('!');
            ev.cancel();
        });
        p.on::<ChatEvent>(Priority::Lowest.receiving_cancelled(), move |_, ev| lowest.push(format!("lowest {}", ev.message)));
        p.on::<ChatEvent>(Priority::Monitor, move |_, ev| {
            monitor.push(format!("monitor {}", ev.is_cancelled()));
            ev.message.clear();
        });
        p.on::<CommandEvent>(Priority::Highest, move |_, _| command.push("command"));
    });
    let mut fake = Fake::with(&["Steve"]);
    let chat = listeners.offer(&mut ShardCtx::new(&mut fake), ChatEvent::new(player(1, "Steve"), "hi".into()));
    assert_eq!(log.take(), ["high", "lowest hi!", "monitor true"]);
    assert!(chat.is_cancelled() && chat.message == "hi!", "what a monitor does is discarded");
}

#[test]
fn scoped_listeners_see_their_world_or_player_until_the_scope_ends() {
    let (log, mut listeners) = (Log::default(), Listeners::default());
    let mut fake = Fake::with(&["Steve", "Alex"]);
    let mut ctx = ShardCtx::new(&mut fake);
    let (arena, alex) = (log.clone(), log.clone());
    ctx.in_world("arena").unwrap().on::<JoinEvent>(Priority::Normal, move |_, ev| arena.push(format!("arena {}", ev.player().name)));
    ctx.for_player(PlayerId(2)).unwrap().on::<JoinEvent>(Priority::Normal, move |_, ev| alex.push(format!("alex {}", ev.player().name)));
    assert!(ctx.in_world("nowhere").is_none() && ctx.for_player(PlayerId(9)).is_none());
    fake.deferred.drain(..).for_each(|change| listeners.apply(change));

    let mut join = |fake: &mut Fake, who: Player| drop(listeners.offer(&mut ShardCtx::new(fake), JoinEvent::new(who)));
    join(&mut fake, player(1, "Steve"));
    join(&mut fake, player(2, "Alex"));
    fake.world = Some("arena".into());
    join(&mut fake, player(1, "Steve"));
    assert_eq!(log.take(), ["alex Alex", "arena Steve"]);

    listeners.apply(ListenerChange::end(Scope::World("arena".into())));
    listeners.apply(ListenerChange::end(Scope::Player(PlayerId(2))));
    let mut join = |fake: &mut Fake, who: Player| drop(listeners.offer(&mut ShardCtx::new(fake), JoinEvent::new(who)));
    join(&mut fake, player(2, "Alex"));
    assert!(log.take().is_empty());
}

#[test]
fn tasks_run_when_due_and_can_be_removed() {
    let (log, mut listeners) = (Log::default(), Listeners::default());
    let (every, later) = (log.clone(), log.clone());
    build(&mut listeners, |p| {
        p.every(2, move |ctx| every.push(format!("every {}", ctx.tick())));
        p.later(3, move |ctx| later.push(format!("later {}", ctx.tick())));
    });
    let mut fake = Fake::default();
    for tick in 0..7 {
        fake.tick = tick;
        listeners.run_tasks(&mut ShardCtx::new(&mut fake));
    }
    assert_eq!(log.take(), ["every 2", "later 3", "every 4", "every 6"]);

    let mut ctx = ShardCtx::new(&mut fake);
    let counted = log.clone();
    let id = ctx.on_shard().every(1, move |_| counted.push("tick"));
    ctx.remove_listener(id);
    fake.deferred.drain(..).for_each(|change| listeners.apply(change));
    fake.tick = 10;
    listeners.run_tasks(&mut ShardCtx::new(&mut fake));
    assert_eq!(log.take(), ["every 10"], "the removed task never ran");
}

#[test]
fn a_handler_is_filed_as_one_listener_per_event() {
    struct Greeter(Log);

    impl Handler for Greeter {
        fn on_join(&mut self, _: &mut ShardCtx<'_>, player: &Player) {
            self.0.push(format!("join {}", player.name));
        }

        fn on_chat(&mut self, _: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
            self.0.push(format!("chat {} {}", player.name, event.message));
            event.cancel();
        }

        fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
            self.0.push(format!("tick {}", ctx.tick()));
        }
    }

    let (log, mut listeners) = (Log::default(), Listeners::default());
    build(&mut listeners, |p| {
        p.handler(Priority::Normal, Box::new(Greeter(log.clone())));
    });
    let mut fake = Fake::with(&["Steve"]);
    listeners.offer(&mut ShardCtx::new(&mut fake), JoinEvent::new(player(1, "Steve")));
    let chat = listeners.offer(&mut ShardCtx::new(&mut fake), ChatEvent::new(player(1, "Steve"), "hi".into()));
    listeners.offer(&mut ShardCtx::new(&mut fake), TickEvent);
    assert!(chat.is_cancelled());
    assert_eq!(log.take(), ["join Steve", "chat Steve hi", "tick 40"]);
}
