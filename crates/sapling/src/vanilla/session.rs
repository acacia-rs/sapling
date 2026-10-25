//! The commands about the server itself: `/defaultgamemode`, `/setmaxplayers`, `/seed`,
//! `/version` and `/transfer`. Of these, BDS has only `/setmaxplayers` and `/transfer`, for its
//! console and owners (research/captures/bds-worldcmds.txt, bds-console-cmds.txt); here they are
//! for operators. The others answer with the game's own strings.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, Parameter, ParameterSpec, ParseError};
use sapling_api::{Command, GameMode, Output, Permission, Plugin, PluginBuilder, Sender, ShardCtx};
use sapling_net::PlayerLimit;
use sapling_net::proto::{GAME_VERSION, PROTOCOL_VERSION};

/// Where `/transfer` sends players when no port is named.
const DEFAULT_PORT: u16 = 19132;

/// The server's player limit, for `/setmaxplayers`; set on the shard's first tick.
#[derive(Default)]
struct Installed(Option<PlayerLimit>);

/// Hands each shard the server's [`PlayerLimit`].
pub(super) struct Limit(pub PlayerLimit);

impl Plugin for Limit {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let limit = self.0.clone();
        plugin.later(0, move |ctx| ctx.state::<Installed>().0 = Some(limit));
    }
}

fn mode_key(mode: GameMode) -> String {
    format!("%createWorldScreen.gameMode.{}", format!("{mode:?}").to_lowercase())
}

/// The game mode newcomers to this shard get.
#[derive(Command)]
#[command(crate = "crate", name = "defaultgamemode", description = "Sets the default game mode", permission = "operator")]
pub(crate) struct DefaultGameMode {
    game_mode: GameMode,
}

impl Command for DefaultGameMode {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        ctx.set_default_game_mode(self.game_mode);
        out.translate("commands.defaultgamemode.success", vec![mode_key(self.game_mode)], true);
    }
}

/// `/setmaxplayers <maxPlayers>`: kept between the players online and the most the listener
/// was made for, each bound noted after the answer as BDS notes it.
#[derive(Command)]
#[command(crate = "crate", name = "setmaxplayers", description = "Sets the maximum number of players for this game session", permission = "operator")]
pub(crate) struct SetMaxPlayers {
    max_players: i32,
}

impl Command for SetMaxPlayers {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(limit) = ctx.state::<Installed>().0.clone() else { return out.error("The player limit is not kept on this shard") };
        let (online, ceiling) = (ctx.player_count(), limit.ceiling());
        let max = usize::try_from(self.max_players).unwrap_or(0).clamp(online, ceiling.max(online));
        limit.set(max);
        out.translate("commands.setmaxplayers.success", vec![max.to_string()], true);
        match (max as i64).cmp(&i64::from(self.max_players)) {
            std::cmp::Ordering::Greater => out.translate("commands.setmaxplayers.success.lowerbound", Vec::new(), true),
            std::cmp::Ordering::Less => out.translate("commands.setmaxplayers.success.upperbound", Vec::new(), true),
            std::cmp::Ordering::Equal => {}
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "seed", description = "Shows the world's seed", permission = "operator")]
pub(crate) struct Seed;

impl Command for Seed {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        out.translate("commands.seed.success", vec![ctx.seed().to_string()], true);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "version", description = "Shows the server's version", permission = "operator")]
pub(crate) struct Version;

impl Command for Version {
    fn run(self, _: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        out.print(format!("sapling {} for Minecraft: Bedrock Edition {GAME_VERSION} (protocol {PROTOCOL_VERSION})", env!("CARGO_PKG_VERSION")));
    }
}

/// `/transfer <player> <server> [port]`: the player, by name (BDS takes no selector), is sent to
/// another server. Moving players between this server's worlds is [`ShardCtx::transfer`].
pub(crate) struct Transfer {
    player: String,
    server: String,
    port: Option<i64>,
}

impl CommandSyntax for Transfer {
    fn spec() -> CommandSpec {
        let parameters = vec![ParameterSpec::of::<String>("pfidOrMSA"), ParameterSpec::of::<String>("server"), ParameterSpec::of::<Option<i64>>("port")];
        CommandSpec { name: "transfer".into(), aliases: Vec::new(), description: "Transfers a player to another server".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let player = String::parse(args)?;
        if player.starts_with('@') {
            return Err(args.error_at(mark));
        }
        let transfer = Self { player, server: String::parse(args)?, port: Option::<i64>::parse(args)? };
        args.finish()?;
        Ok(transfer)
    }
}

impl Command for Transfer {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Ok(port) = u16::try_from(self.port.unwrap_or(i64::from(DEFAULT_PORT))) else {
            return out.translate("commands.transferserver.invalid.port", Vec::new(), false);
        };
        let found = ctx.players().find(|player| player.name.eq_ignore_ascii_case(&self.player) || player.xuid == self.player);
        match found.filter(|player| ctx.send_to_server(player.id, &self.server, port)) {
            Some(_) => out.translate("commands.transferserver.successful", Vec::new(), true),
            None => out.translate("commands.generic.player.notFound", Vec::new(), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_takes_a_name_not_a_selector() {
        let sender = Sender::Console;
        let parsed = |line: &str| Transfer::parse(&mut Args::new(line, 0, &sender, &[])).map(|transfer| (transfer.player, transfer.server, transfer.port));
        assert_eq!(parsed("Steve example.org 19133").unwrap(), ("Steve".into(), "example.org".into(), Some(19133)));
        assert_eq!(parsed("Steve 127.0.0.1").unwrap().2, None);
        assert!(parsed("@a 127.0.0.1").is_err() && parsed("Steve").is_err());
    }
}
