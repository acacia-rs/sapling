mod blocks;
mod commands;
mod forms;
mod items;

use crate::testing::{Fake, player};
use crate::{BossBar, BossBarColour, ChatEvent, CommandEvent, GameMode, Handler, Permission, Player, PlayerId, Priority, ShardCtx, Sidebar, Title};

struct Hub;

impl Handler for Hub {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let names: Vec<String> = ctx.players().map(|p| p.name).collect();
        let at = ctx.position(player.id).map(|p| p.feet);
        ctx.broadcast(&format!("{} joined {names:?} at {at:?} on tick {} ({} online)", player.name, ctx.tick(), ctx.player_count()));
        ctx.send_message(player.id, "welcome");
        ctx.set_permission(player.id, Permission::Operator);
        ctx.set_game_mode(player.id, GameMode::Creative);
        ctx.teleport(player.id, ctx.spawn());
        ctx.set_sidebar(player.id, &Sidebar::new("Hub").line("one").line("two"));
        ctx.remove_sidebar(player.id);
        ctx.set_boss_bar(player.id, &BossBar::new("Boss").progress(7.0).colour(BossBarColour::Red));
        ctx.remove_boss_bar(player.id);
        ctx.send_title(player.id, &Title::new("Hi").subtitle("there").times(1, 2, 3));
        ctx.send_actionbar(player.id, "bar");
        ctx.send_toast(player.id, "Toast", "is ready");
        ctx.kick(player.id, "bye");
    }
}

#[test]
fn handlers_act_through_the_context_and_default_to_no_ops() {
    let mut backend = Fake::with(&["Steve"]);
    let steve = player(1, "Steve");
    let mut handler: Box<dyn Handler> = Box::new(Hub);
    handler.on_join(&mut ShardCtx::new(&mut backend), &steve);
    handler.on_quit(&mut ShardCtx::new(&mut backend), &steve);
    handler.on_tick(&mut ShardCtx::new(&mut backend));
    let (mut chat, mut command) = (ChatEvent::new("hi".into()), CommandEvent::new("tp  home".into()));
    handler.on_chat(&mut ShardCtx::new(&mut backend), &steve, &mut chat);
    handler.on_command(&mut ShardCtx::new(&mut backend), &steve, &mut command);
    assert!(!chat.is_cancelled() && !command.is_cancelled() && chat.message == "hi");
    assert_eq!((command.name(), command.line()), ("tp", "tp  home"));
    let expected = [
        r#"broadcast Steve joined ["Steve"] at Some([1.0, 2.0, 3.0]) on tick 40 (1 online)"#,
        "message 1 welcome",
        "permission 1 Operator",
        "game mode 1 Creative",
        "teleport 1 [0.5, 65.0, 0.5]",
        r#"sidebar 1 Some(("Hub", ["one", "two"]))"#,
        "sidebar 1 None",
        r#"boss bar 1 Some(("Boss", 1.0, Red))"#,
        "boss bar 1 None",
        r#"title 1 Hi Some("there")"#,
        "actionbar 1 bar",
        "toast 1 Toast is ready",
        "kick 1 bye",
    ];
    assert_eq!(backend.log, expected);
    assert!(ShardCtx::new(&mut backend).position(PlayerId(9)).is_none());
}

#[test]
fn state_is_kept_per_type_between_calls() {
    #[derive(Default)]
    struct Visits(u32);
    let mut backend = Fake::default();
    ShardCtx::new(&mut backend).state::<Visits>().0 += 2;
    ShardCtx::new(&mut backend).state::<Vec<String>>().push("other".into());
    let mut ctx = ShardCtx::new(&mut backend);
    assert_eq!((ctx.state::<Visits>().0, ctx.state::<Vec<String>>().len(), *ctx.state::<u32>()), (2, 1, 0));
}

#[test]
fn worlds_are_named_and_within_puts_the_focus_back() {
    let mut backend = Fake::with(&["Steve"]);
    let mut ctx = ShardCtx::new(&mut backend);
    assert_eq!((ctx.worlds(), ctx.world(), ctx.world_of(PlayerId(1))), (vec!["arena".to_owned(), "hub".to_owned()], "hub".to_owned(), Some("hub".to_owned())));
    assert_eq!(ctx.within("arena", |ctx| (ctx.world(), ctx.players_in("hub").len())), Some(("arena".to_owned(), 1)));
    assert_eq!((ctx.world(), ctx.within("nowhere", |_| ())), ("hub".to_owned(), None));
    assert!(ctx.transfer(PlayerId(1), "arena") && !ctx.transfer(PlayerId(1), "nowhere"));
    assert_eq!(ctx.block_in("nowhere", [0, 1, 0]), None);
    ctx.broadcast_in("arena", "go");
    ctx.broadcast_all("hello");
    assert!(ctx.create_world("a1", "arena") && ctx.reset_world("a1") && ctx.remove_world("a1"));
    let expected = ["transfer 1 arena", "transfer 1 nowhere", "broadcast in arena go", "broadcast all hello", "create a1 from arena", "reset a1", "remove a1"];
    assert_eq!(backend.log, expected);
}

#[test]
fn events_cancel_and_priorities_order() {
    let mut chat = ChatEvent::new("hi".into());
    chat.cancel();
    assert!(chat.is_cancelled());
    let mut order = [Priority::Monitor, Priority::Low, Priority::Highest, Priority::Normal, Priority::Lowest, Priority::High];
    order.sort();
    assert_eq!(order, [Priority::Highest, Priority::High, Priority::Normal, Priority::Low, Priority::Lowest, Priority::Monitor]);
    assert_eq!(Priority::default(), Priority::Normal);
    assert!(BossBar::new("x").progress(f32::NAN).progress == 0.0 && Permission::Member < Permission::Operator);
    assert_eq!(Title::new("t").times(1, 2, 3).stay, 2);
}
