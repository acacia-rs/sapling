//! Milestone 1 step 4: bots use a server's commands, forms and HUD. acacia-bot's own trackers
//! (boss bars, scoreboard, forms, titles) say what a client made of the packets.

mod common;

use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::events::TitleKind;
use acacia_bot::forms::{FormKind, FormReply, FormValue};
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{AvailableCommands, BossEventColor, CommandOutput};
use acacia_bot::{Bot, BotConfig, BotEvent, Events};
use common::{GROUND, SPAWN};
use sapling::{
    BossBar, BossBarColour, Command, Form, FormResponse, Handler, Menu, Output, Permission, Player,
    Sender, Server, ShardCtx, Sidebar, Title,
};

const WAIT: Duration = Duration::from_secs(10);

/// The block column a player stands in, as the sidebar shows it.
fn column(feet: [f32; 3]) -> String {
    format!("at {} {}", feet[0].floor(), feet[2].floor())
}

fn sidebar(ctx: &ShardCtx<'_>, player: &Player) -> Sidebar {
    let at = ctx
        .position(player.id)
        .map_or(String::new(), |p| column(p.feet));
    Sidebar::new("§lHub")
        .line(format!("Online: {}", ctx.player_count()))
        .line(at)
}

struct Hub;

impl Handler for Hub {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        if player.name == "Alice" {
            ctx.set_permission(player.id, Permission::Operator);
        }
        ctx.send_title(
            player.id,
            &Title::new("Welcome")
                .subtitle(format!("to the hub, {}", player.name))
                .times(5, 40, 5),
        );
        ctx.set_boss_bar(
            player.id,
            &BossBar::new("sapling hub")
                .progress(0.5)
                .colour(BossBarColour::Green),
        );
    }

    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        for player in ctx.players() {
            let board = sidebar(ctx, &player);
            ctx.set_sidebar(player.id, &board);
        }
    }
}

#[derive(Command)]
#[command(name = "hub", description = "Back to the spawn")]
struct HubCommand;

impl Command for HubCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else {
            return out.error("players only");
        };
        ctx.teleport(player.id, ctx.spawn());
        out.print("Back at the hub");
    }
}

#[derive(Form, Debug)]
#[form(title = "Settings")]
struct Settings {
    nickname: String,
    #[form(label = "View distance", min = 2, max = 16)]
    view: u32,
    #[form(options = ["red", "green", "blue"])]
    colour: usize,
}

/// `/ask menu` and `/ask settings` open a form and broadcast its answer.
#[derive(Command)]
enum Ask {
    Menu,
    Settings,
}

impl Command for Ask {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let Some(player) = sender.player() else {
            return;
        };
        match self {
            Self::Menu => ctx.send_form(
                player.id,
                Menu::new("Games")
                    .body("Pick one")
                    .button("Parkour")
                    .button("Spleef"),
                |ctx, player, response| {
                    ctx.broadcast(&format!("{} picked {response:?}", player.name));
                },
            ),
            Self::Settings => ctx.send_form(
                player.id,
                Settings {
                    nickname: "Al".into(),
                    view: 8,
                    colour: 0,
                },
                |ctx, player, response| match response {
                    FormResponse::Submitted(s) => ctx.broadcast(&format!(
                        "{} set {} {} {}",
                        player.name, s.nickname, s.view, s.colour
                    )),
                    FormResponse::Closed => {
                        ctx.broadcast(&format!("{} closed the settings", player.name))
                    }
                },
            ),
        }
    }
}

#[derive(Command)]
#[command(permission = "operator")]
struct Clear;

impl Command for Clear {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else {
            return;
        };
        ctx.remove_boss_bar(player.id);
        out.print("Cleared");
    }
}

/// Events up to and including the first one `last` accepts.
async fn until(bot: &mut Bot, last: impl Fn(&BotEvent) -> bool) -> Vec<BotEvent> {
    let gathered = tokio::time::timeout(WAIT, async {
        let mut events = Vec::new();
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(event) => {
                    events.push(event);
                    if last(events.last().unwrap()) {
                        return events;
                    }
                }
                None => panic!("the connection is gone"),
            }
        }
    });
    gathered.await.expect("the event arrives in time")
}

fn packet<T: Packet>(event: &BotEvent) -> Option<T> {
    match event {
        BotEvent::Packet(raw) if raw.id == T::ID => {
            Some(raw.decode().expect("the bot decodes what sapling sent"))
        }
        _ => None,
    }
}

/// Runs `line` and returns the (message, success) of its answer.
async fn run(bot: &mut Bot, line: &str) -> (String, bool) {
    bot.client().command(line);
    let events = until(bot, |e| packet::<CommandOutput>(e).is_some()).await;
    let output = packet::<CommandOutput>(events.last().unwrap()).unwrap();
    assert_eq!(
        (output.output.len(), output.output_type.as_str()),
        (1, "alloutput")
    );
    (
        output.output[0].message_id.clone(),
        output.output[0].success,
    )
}

async fn chat(bot: &mut Bot) -> String {
    let events = until(bot, |e| matches!(e, BotEvent::Chat(_))).await;
    let Some(BotEvent::Chat(line)) = events.into_iter().last() else {
        unreachable!()
    };
    line.message
}

/// Joins and returns the bot with the command names its client was told.
async fn join(server: &Server, name: &str) -> (Bot, Vec<String>, Vec<BotEvent>) {
    let subscribe = PacketFilter::none()
        .with(AvailableCommands::ID)
        .with(CommandOutput::ID);
    let config = BotConfig {
        subscribe,
        events: Events::TITLES | Events::CHAT | Events::FORMS,
        ..BotConfig::default()
    };
    let mut bot = common::spawn(common::client(server, name), config).await;
    let events = until(&mut bot, |e| packet::<AvailableCommands>(e).is_some()).await;
    let list = packet::<AvailableCommands>(events.last().unwrap()).unwrap();
    (
        bot,
        list.command_data.into_iter().map(|c| c.name).collect(),
        events,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_uses_commands_forms_and_the_hud() {
    let builder = Server::builder()
        .handler(Hub)
        .command::<HubCommand>()
        .command::<Ask>()
        .command::<Clear>();
    let server = common::start(builder).await;
    let (mut alice, commands, events) = join(&server, "Alice").await;
    assert_eq!(commands, ["hub", "ask", "clear"]);

    // The title on join: subtitle first, then the title that shows it.
    let titles: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let BotEvent::Title(t) = e {
                Some((t.kind, t.text.as_str()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        titles,
        [
            (TitleKind::Subtitle, "to the hub, Alice"),
            (TitleKind::Title, "Welcome")
        ]
    );

    alice.wait_ticks(10).await.expect("connected");
    let bars: Vec<_> = alice
        .state()
        .environment
        .boss_bars
        .iter()
        .map(|(id, bar)| (*id, bar.title.clone(), bar.progress, bar.color))
        .collect();
    assert_eq!(
        bars,
        [(
            alice.client().runtime_entity_id() as i64,
            "sapling hub".to_owned(),
            0.5,
            BossEventColor::Green
        )]
    );
    let spawn = column([SPAWN.0 as f32, GROUND as f32, SPAWN.1 as f32]);
    let board = alice.state().scoreboard.sidebar().expect("a sidebar");
    assert_eq!(
        (
            board.display_name.as_str(),
            alice.state().scoreboard.sidebar_text()
        ),
        ("§lHub", vec!["Online: 1".to_owned(), spawn.clone()])
    );

    // A member gets a shorter list, and what is not on it does not exist for it.
    let (mut bob, commands, _) = join(&server, "Bob").await;
    assert_eq!(commands, ["hub", "ask"]);
    assert_eq!(
        run(&mut bob, "clear").await,
        ("commands.generic.unknown".to_owned(), false)
    );
    assert_eq!(
        run(&mut bob, "ask nobody").await,
        ("commands.generic.syntax".to_owned(), false)
    );

    // Alice walks off and is brought back; the sidebar followed her there and back, line by line.
    alice.controls().expect("physics").forward = 1.0;
    alice.wait_ticks(30).await.expect("connected");
    alice.controls().expect("physics").stop();
    alice.wait_ticks(10).await.expect("connected");
    let away = column(
        alice
            .movement()
            .and_then(|m| m.position())
            .expect("movement started"),
    );
    assert_ne!(away, spawn);
    assert_eq!(
        alice.state().scoreboard.sidebar_text(),
        ["Online: 2".to_owned(), away]
    );
    assert_eq!(
        run(&mut alice, "/hub").await,
        ("Back at the hub".to_owned(), true)
    );
    alice.wait_ticks(10).await.expect("connected");
    let movement = alice.movement().expect("physics");
    assert_eq!(
        (movement.teleports, movement.position().map(column)),
        (1, Some(spawn.clone()))
    );
    // The server took the bot's confirmation of the teleport: it has her at the spawn again.
    assert_eq!(alice.state().scoreboard.sidebar_text()[1], spawn);

    // A menu: the bot presses the second button and the handler gets its index.
    alice.client().command("ask menu");
    let form = alice.wait_form(WAIT).await.expect("the menu arrives");
    let FormKind::Simple {
        content, buttons, ..
    } = &form.kind
    else {
        panic!("{form:?}")
    };
    assert_eq!(
        (
            form.title.as_str(),
            content.as_str(),
            buttons.iter().map(|b| b.text.as_str()).collect::<Vec<_>>()
        ),
        ("Games", "Pick one", vec!["Parkour", "Spleef"])
    );
    alice
        .answer_form(form.id, FormReply::Button(1))
        .await
        .expect("answered");
    assert_eq!(chat(&mut bob).await, "Alice picked Submitted(1)");

    // A derived form: the bot fills it in and the handler gets the struct.
    alice.client().command("ask settings");
    let form = alice.wait_form(WAIT).await.expect("the settings arrive");
    assert_eq!(
        form.defaults(),
        [
            FormValue::Text("Al".into()),
            FormValue::Slider(8.0),
            FormValue::Choice(0)
        ]
    );
    let reply = FormReply::Custom(vec![
        FormValue::Text("Ali".into()),
        FormValue::Slider(12.0),
        FormValue::Choice(2),
    ]);
    alice.answer_form(form.id, reply).await.expect("answered");
    assert_eq!(chat(&mut bob).await, "Alice set Ali 12 2");
    alice.client().command("ask settings");
    let form = alice
        .wait_form(WAIT)
        .await
        .expect("the settings arrive again");
    alice.close_form(form.id).await.expect("closed");
    assert_eq!(chat(&mut bob).await, "Alice closed the settings");

    assert_eq!(run(&mut alice, "clear").await, ("Cleared".to_owned(), true));
    alice.wait_ticks(5).await.expect("connected");
    assert!(alice.state().environment.boss_bars.is_empty());
    for bot in [&alice, &bob] {
        assert_eq!(bot.disconnect_reason(), None);
    }
    alice.disconnect().await;
    bob.disconnect().await;
    server.shutdown().await;
}
