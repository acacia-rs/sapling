//! A hub, in a vanilla world folder or, without one, on a stone platform in the void: a title and
//! a boss bar on join, a sidebar with who is online and where you stand, `/hub`, `/menu`,
//! `/settings` and, for operators, vanilla's commands (`/gamemode`, `/tp`, `/setblock` and the
//! rest: `sapling::vanilla`). Only operators in creative mode change blocks.
//! A second world, `arena`, is a stone platform on a second shard, where anyone may dig and
//! build: `/world arena` and `/world hub` (or the menu) move you between them.
//! `cargo run -p sapling --example hub -- [port] [world folder] [--offline] [--no-cache] [--op=Name] [--nethernet] [--lan]`
//! (default port 19132, online mode). `--nethernet` also hosts over NetherNet direct connect;
//! `--lan` adds the LAN list to that. `RUST_LOG=debug` shows refused logins and ended connections.
//! Lines typed on stdin run as the server's commands.
use std::collections::HashMap;
use std::path::Path;

use sapling::{
    BossBar, BossBarColour, BreakEvent, ChatEvent, Command, Form, FormResponse, GameMode, Handler, InteractEvent, Menu, Output, Permission, PlaceEvent, Player, PlayerId, Sender,
    Lan, NetherNet, Server, ShardCtx, Sidebar, Title,
};

const NAME: &str = "sapling hub";
const HUB: &str = "hub";
const ARENA: &str = "arena";
/// In the working directory; clients pin the key in it per server address.
const HOST_KEY_FILE: &str = "nethernet-host.key";
const COLOURS: [BossBarColour; 4] = [BossBarColour::Purple, BossBarColour::Green, BossBarColour::Blue, BossBarColour::Red];
/// Ticks between sidebar updates.
const SIDEBAR_EVERY: u64 = 10;

/// What a player chose in `/settings`.
#[derive(Form, Clone)]
#[form(title = "Settings")]
struct Settings {
    #[form(note = "These last until you leave.", label = "Show the sidebar")]
    sidebar: bool,
    #[form(label = "Boss bar colour", options = ["Purple", "Green", "Blue", "Red"])]
    colour: usize,
    #[form(label = "Boss bar fill (%)", min = 0, max = 100, step = 10)]
    fill: u32,
    #[form(label = "Greet me with", placeholder = "a title of your own", max_len = 32)]
    greeting: String,
}

impl Default for Settings {
    fn default() -> Self {
        // Green and half full: neither is what a client shows when it misreads ShowBar.
        Self { sidebar: true, colour: 1, fill: 50, greeting: String::new() }
    }
}

/// The settings of everyone online, shared by the handler, the commands and the forms.
#[derive(Default)]
struct Chosen(HashMap<PlayerId, Settings>);

fn settings(ctx: &mut ShardCtx<'_>, player: PlayerId) -> Settings {
    ctx.state::<Chosen>().0.entry(player).or_default().clone()
}

fn show_boss_bar(ctx: &mut ShardCtx<'_>, player: PlayerId, settings: &Settings) {
    ctx.set_boss_bar(player, &BossBar::new(NAME).progress(settings.fill as f32 / 100.0).colour(COLOURS[settings.colour]));
}

fn show_sidebar(ctx: &mut ShardCtx<'_>, player: PlayerId) {
    if !settings(ctx, player).sidebar {
        return ctx.remove_sidebar(player);
    }
    let Some([x, y, z]) = ctx.position(player).map(|p| p.feet.map(|c| c.floor() as i32)) else { return };
    let world = ctx.world_of(player).unwrap_or_default();
    let here = ctx.players_in(&world).len();
    let sidebar = Sidebar::new(format!("§l§a{NAME}")).line(format!("World: §b{world}")).line(format!("Here: §e{here}")).line("").line(format!("§7{x} {y} {z}"));
    ctx.set_sidebar(player, &sidebar);
}

/// Sends a player to another world; they hear at once if that cannot be.
fn change_world(ctx: &mut ShardCtx<'_>, player: PlayerId, world: &str) -> bool {
    let moving = ctx.transfer(player, world);
    if !moving {
        ctx.send_message(player, &format!("§cYou cannot go to {world} from here"));
    }
    moving
}

fn to_spawn(ctx: &mut ShardCtx<'_>, player: PlayerId) {
    ctx.teleport(player, ctx.spawn());
    ctx.send_actionbar(player, "§aBack at the spawn");
}

/// The protection: in the arena anyone digs and builds; in the hub only operators in creative
/// mode change a block.
fn may_build(ctx: &ShardCtx<'_>, player: &Player) -> bool {
    ctx.world() == ARENA || ctx.permission(player.id) == Some(Permission::Operator) && ctx.game_mode(player.id) == Some(GameMode::Creative)
}

struct Hub {
    operators: Vec<String>,
}

impl Handler for Hub {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, xuid = %player.xuid, uuid = %player.uuid, online = ctx.player_count(), "joined");
        ctx.broadcast_all(&format!("§e{} joined the game", player.name));
        if self.operators.contains(&player.name) {
            ctx.set_permission(player.id, Permission::Operator);
        }
        ctx.send_title(player.id, &Title::new("§aWelcome").subtitle(format!("to the {NAME}, {}", player.name)).times(10, 60, 20));
        show_boss_bar(ctx, player.id, &Settings::default());
        show_sidebar(ctx, player.id);
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, online = ctx.player_count(), "left");
        ctx.state::<Chosen>().0.remove(&player.id);
        ctx.broadcast_all(&format!("§e{} left the game", player.name));
    }

    /// A move is not a leave: each world hears where the player went or came from.
    fn on_world_change(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, from: &str, to: &str) {
        tracing::info!(name = %player.name, from, to, "changed world");
        ctx.broadcast_in(from, &format!("§7{} went to {to}", player.name));
        ctx.broadcast_in(to, &format!("§7{} came from {from}", player.name));
        ctx.send_title(player.id, &Title::new(format!("§b{to}")).times(5, 30, 10));
        // Settings are kept per shard: on another one the player starts with the defaults.
        let chosen = settings(ctx, player.id);
        show_boss_bar(ctx, player.id, &chosen);
        show_sidebar(ctx, player.id);
    }

    fn on_transfer_failed(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, to: &str) {
        ctx.send_message(player.id, &format!("§c{to} is not taking players right now"));
    }

    fn on_chat(&mut self, _: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
        tracing::info!(name = %player.name, message = %event.message, "chat");
    }

    fn on_break(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut BreakEvent) {
        if may_build(ctx, player) { event.allow() } else { event.cancel() }
    }

    fn on_place(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut PlaceEvent) {
        if may_build(ctx, player) { event.allow() } else { event.cancel() }
    }

    fn on_interact(&mut self, _: &mut ShardCtx<'_>, player: &Player, event: &mut InteractEvent) {
        tracing::debug!(name = %player.name, block = %event.block().name, at = ?event.pos(), "clicked a block");
    }

    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        if ctx.tick() % SIDEBAR_EVERY == 0 {
            ctx.players().for_each(|player| show_sidebar(ctx, player.id));
        }
    }
}

#[derive(Command)]
#[command(name = "hub", aliases = ["lobby", "spawn"], description = "Takes you back to the spawn")]
struct HubCommand;

impl Command for HubCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else { return out.error("Only a player can go to the hub") };
        to_spawn(ctx, player.id);
        out.print("Back at the spawn");
    }
}

#[derive(Command)]
#[command(name = "menu", description = "Opens the hub menu")]
struct MenuCommand;

impl Command for MenuCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else { return out.error("Only a player can open the menu") };
        let other = if ctx.world_of(player.id).as_deref() == Some(ARENA) { HUB } else { ARENA };
        let menu = Menu::new(NAME).body("What would you like?").button("Back to the spawn").button("Greet everyone").button("A toast").button("Settings").button(format!("Go to the {other}"));
        ctx.send_form(player.id, menu, move |ctx, player, response| match response {
            FormResponse::Submitted(0) => to_spawn(ctx, player.id),
            FormResponse::Submitted(1) => ctx.broadcast(&format!("§d{} says hello!", player.name)),
            FormResponse::Submitted(2) => ctx.send_toast(player.id, NAME, &format!("{} online", ctx.player_count())),
            FormResponse::Submitted(3) => open_settings(ctx, player.id),
            FormResponse::Submitted(_) => drop(change_world(ctx, player.id, other)),
            FormResponse::Closed => {}
        });
    }
}

#[derive(Command)]
#[command(name = "world", aliases = ["transfer"], description = "Takes you to another world, or lists them")]
struct WorldCommand {
    name: Option<String>,
}

impl Command for WorldCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else { return out.error("Only a player can change worlds") };
        match self.name {
            Some(name) if change_world(ctx, player.id, &name) => out.print(format!("Going to {name}")),
            Some(name) => out.error(format!("No way to {name}")),
            None => out.print(format!("You are in {}. Worlds: {}", ctx.world(), ctx.worlds().join(", "))),
        }
    }
}

fn open_settings(ctx: &mut ShardCtx<'_>, player: PlayerId) {
    let current = settings(ctx, player);
    ctx.send_form(player, current, |ctx, player, response| {
        let FormResponse::Submitted(chosen) = response else { return };
        show_boss_bar(ctx, player.id, &chosen);
        if !chosen.greeting.is_empty() {
            ctx.send_title(player.id, &Title::new(chosen.greeting.clone()));
        }
        ctx.state::<Chosen>().0.insert(player.id, chosen);
        show_sidebar(ctx, player.id);
    });
}

#[derive(Command)]
#[command(name = "settings", description = "Changes what the hub shows you")]
struct SettingsCommand;

impl Command for SettingsCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match sender.player() {
            Some(player) => open_settings(ctx, player.id),
            None => out.error("Only a player has settings"),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let offline = args.iter().any(|a| a == "--offline");
    let operators: Vec<String> = args.iter().filter_map(|a| a.strip_prefix("--op=")).map(str::to_owned).collect();
    let positional = || args.iter().filter(|a| !a.starts_with("--"));
    let port = positional().find_map(|a| a.parse::<u16>().ok()).unwrap_or(19132);
    let folder = positional().find(|a| a.parse::<u16>().is_err());

    let mut builder = Server::builder().bind(([0, 0, 0, 0], port).into()).name(NAME).online_mode(!offline).blob_cache(!args.iter().any(|a| a == "--no-cache")).console(true);
    let lan = args.iter().any(|a| a == "--lan");
    if lan || args.iter().any(|a| a == "--nethernet") {
        let nethernet = NetherNet { lan: lan.then(Lan::default), ..NetherNet::new(sapling::host_key(Path::new(HOST_KEY_FILE))?) };
        builder = builder.nethernet(nethernet);
    }
    if let Some(folder) = folder {
        let opened = sapling::World::open_folder(Path::new(folder))?;
        tracing::info!(folder, name = ?opened.name, spawn = ?opened.spawn, time = opened.world.level().time, "opened the world");
        builder = builder.folder(opened);
    }
    // The arena is on a second shard, so a move between the worlds is a handoff between threads.
    let builder = builder.default_world_name(HUB).add_platform_on(1, ARENA).handler_each(move || Hub { operators: operators.clone() });
    // A hub is looked at, not dug up, whatever the folder's own game mode.
    // Vanilla's operator commands, /gamemode among them.
    let builder = builder.game_mode(GameMode::Adventure).vanilla_commands();
    let builder = builder.command::<HubCommand>().command::<MenuCommand>().command::<SettingsCommand>().command::<WorldCommand>();
    let server = builder.start().await?;
    tokio::select! {
        signal = tokio::signal::ctrl_c() => signal?,
        () = server.stop_requested() => {}
    }
    tracing::info!("shutting down");
    server.shutdown().await;
    Ok(())
}
