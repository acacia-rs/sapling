//! `/ban`, `/ban-ip`, `/pardon`, `/pardon-ip` and `/banlist`: BDS has none, so these are Java's
//! and PocketMine's commands, answered under the old `commands.ban*` keys the client still has. A
//! ban is kept in the access lists, and the listener drops the players it names.

use std::net::IpAddr;

use sapling_api::command::Text;
use sapling_api::{Command, CommandEnum, Output, Sender, ShardCtx};
use sapling_net::AccessLists;
use sapling_net::access::{Ban, IpBan, Terms};

use super::access::lists;

/// Java's reason for a ban that gives none.
const NO_REASON: &str = "Banned by an operator.";

fn terms(reason: Option<Text>, sender: &Sender) -> Terms {
    let reason = reason.map(|reason| reason.0).filter(|reason| !reason.is_empty());
    Terms::new(reason.as_deref().unwrap_or(NO_REASON), sender.name(), None)
}

fn kept(ctx: &mut ShardCtx<'_>, out: &mut Output) -> Option<AccessLists> {
    let found = lists(ctx);
    if found.is_none() {
        out.error("The server keeps no ban list");
    }
    found
}

#[derive(Command)]
#[command(crate = "crate", name = "ban", description = "Adds player to banlist.", permission = "operator")]
pub(crate) struct BanCommand {
    player: String,
    reason: Option<Text>,
}

impl Command for BanCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(lists) = kept(ctx, out) else { return };
        // A player in the game is banned by XUID too, which outlasts a new gamertag.
        let xuid = ctx.players().find(|player| player.name.eq_ignore_ascii_case(&self.player)).map(|player| player.xuid).unwrap_or_default();
        let ban = Ban { name: self.player.clone(), xuid, terms: terms(self.reason, sender) };
        let banned = lists.ban(ban);
        out.translate(if banned { "commands.ban.success" } else { "commands.ban.failed" }, vec![self.player], banned);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "ban-ip", description = "Adds IP address to banlist.", permission = "operator")]
pub(crate) struct BanIp {
    target: String,
    reason: Option<Text>,
}

impl Command for BanIp {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(lists) = kept(ctx, out) else { return };
        let Some(ip) = self.target.parse::<IpAddr>().ok().or_else(|| lists.address_of(&self.target)) else {
            return out.translate("commands.banip.invalid", Vec::new(), false);
        };
        if !lists.ban_ip(IpBan { ip, terms: terms(self.reason, sender) }) {
            return out.error(format!("{ip} is banned already"));
        }
        let mut there = lists.players_at(ip);
        there.sort();
        if there.is_empty() {
            out.translate("commands.banip.success", vec![ip.to_string()], true);
        } else {
            out.translate("commands.banip.success.players", vec![ip.to_string(), there.join(", ")], true);
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "pardon", description = "Removes entries from the banlist.", permission = "operator")]
pub(crate) struct Pardon {
    player: String,
}

impl Command for Pardon {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(lists) = kept(ctx, out) else { return };
        let pardoned = lists.pardon(&self.player);
        out.translate(if pardoned { "commands.unban.success" } else { "commands.unban.failed" }, vec![self.player], pardoned);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "pardon-ip", description = "Removes entries from the IP banlist.", permission = "operator")]
pub(crate) struct PardonIp {
    ip: String,
}

impl Command for PardonIp {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(lists) = kept(ctx, out) else { return };
        let Ok(ip) = self.ip.parse::<IpAddr>() else { return out.translate("commands.unbanip.invalid", Vec::new(), false) };
        if !lists.pardon_ip(ip) {
            return out.error(format!("{ip} is not banned"));
        }
        out.translate("commands.unbanip.success", vec![ip.to_string()], true);
    }
}

#[derive(CommandEnum)]
#[command(crate = "crate", name = "BanlistKind")]
pub(crate) enum BanlistKind {
    Players,
    Ips,
}

#[derive(Command)]
#[command(crate = "crate", name = "banlist", description = "Displays the banlist.", permission = "operator")]
pub(crate) struct Banlist {
    kind: Option<BanlistKind>,
}

impl Command for Banlist {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(lists) = kept(ctx, out) else { return };
        let (key, banned): (_, Vec<String>) = match self.kind.unwrap_or(BanlistKind::Players) {
            BanlistKind::Players => ("commands.banlist.players", lists.bans().into_iter().map(|ban| ban.name).collect()),
            BanlistKind::Ips => ("commands.banlist.ips", lists.ip_bans().into_iter().map(|ban| ban.ip.to_string()).collect()),
        };
        out.translate(key, vec![banned.len().to_string()], true);
        out.print(banned.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use sapling_api::command::{Args, CommandSyntax};

    use super::*;

    #[test]
    fn bans_take_a_name_and_the_rest_of_the_line_as_the_reason() {
        let sender = Sender::Console;
        let ban = BanCommand::parse(&mut Args::new("Steve for good reason", 0, &sender, &[])).ok().map(|ban| (ban.player, ban.reason.map(|reason| reason.0)));
        assert_eq!(ban, Some(("Steve".to_owned(), Some("for good reason".to_owned()))));
        assert_eq!(terms(None, &sender).reason, NO_REASON);
        let banlist = |line: &str| Banlist::parse(&mut Args::new(line, 0, &sender, &[])).ok().map(|list| list.kind);
        assert!(matches!(banlist(""), Some(None)) && matches!(banlist("IPS"), Some(Some(BanlistKind::Ips))));
        assert_eq!([BanIp::spec().name, PardonIp::spec().name], ["ban-ip", "pardon-ip"]);
    }
}
