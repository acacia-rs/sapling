//! `/kick`, `/op` and `/deop` for operators, under BDS's translation keys. `/op` and `/deop` keep
//! the role in `permissions.json` when the server keeps access lists and the player has an XUID,
//! as BDS keeps it only for players Xbox Live vouched for (bedrock_server_how_to.html).

use sapling_api::command::{Selector, Text};
use sapling_api::{Command, Output, Permission, Player, Sender, ShardCtx};
use sapling_net::access::Role;

use super::access::lists;
use super::{no_target, players, tell_target};

/// Where no one is found, a name is answered by name, as BDS's `/kick` answers.
fn nobody(out: &mut Output, target: &Selector, key: &str) {
    match target {
        Selector::Named(name) => out.translate(key, vec![name.clone()], false),
        _ => no_target(out),
    }
}

fn names(players: &[Player]) -> String {
    players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ")
}

#[derive(Command)]
#[command(crate = "crate", name = "kick", description = "Kicks a player from the server.", permission = "operator")]
pub(crate) struct Kick {
    target: Selector,
    reason: Option<Text>,
}

impl Command for Kick {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let mut kicked = players(ctx, sender, &self.target);
        if kicked.is_empty() {
            return nobody(out, &self.target, "commands.kick.not.found");
        }
        let me = sender.player().map(|player| player.id);
        kicked.retain(|player| Some(player.id) != me);
        if kicked.is_empty() {
            return out.translate("commands.kick.not.yourself", Vec::new(), false);
        }
        let reason = self.reason.map(|reason| reason.0).filter(|reason| !reason.is_empty());
        for player in &kicked {
            ctx.kick(player.id, reason.as_deref().unwrap_or("disconnect.kicked"));
        }
        match reason {
            Some(reason) => out.translate("commands.kick.success.reason", vec![names(&kicked), reason], true),
            None => out.translate("commands.kick.success", vec![names(&kicked)], true),
        }
        out.done_to(kicked.len() as u32);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "op", description = "Grants operator status to a player.", permission = "operator")]
pub(crate) struct Op {
    player: Selector,
}

impl Command for Op {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        set_role(ctx, sender, out, &self.player, Role::Operator);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "deop", description = "Revokes operator status from a player.", permission = "operator")]
pub(crate) struct Deop {
    player: Selector,
}

impl Command for Deop {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        set_role(ctx, sender, out, &self.player, Role::Member);
    }
}

/// Makes the targets operators or members; those who already were are the failure's names.
fn set_role(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: &Selector, role: Role) {
    let (permission, key) = if role == Role::Operator { (Permission::Operator, "commands.op") } else { (Permission::Member, "commands.deop") };
    let targets = players(ctx, sender, target);
    if targets.is_empty() {
        return nobody(out, target, "commands.generic.player.notFound");
    }
    let (changed, unchanged): (Vec<Player>, Vec<Player>) = targets.into_iter().partition(|player| ctx.permission(player.id) != Some(permission));
    if changed.is_empty() {
        return out.translate(format!("{key}.failed"), vec![names(&unchanged)], false);
    }
    let lists = lists(ctx);
    for player in &changed {
        ctx.set_permission(player.id, permission);
        if let Some(lists) = &lists {
            lists.set_role(&player.xuid, role);
        }
        tell_target(ctx, player.id, &format!("{key}.message"), &[]);
    }
    out.translate(format!("{key}.success"), vec![names(&changed)], true);
    out.done_to(changed.len() as u32);
}
