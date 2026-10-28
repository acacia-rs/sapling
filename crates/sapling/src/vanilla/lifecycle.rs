//! `/list` for anyone and `/stop`, as BDS answers them (research/captures/bds-cmdlist.txt,
//! bds-ui-operator.txt). BDS keeps `/stop` for its console and owners; here it is for operators.

use sapling_api::{Command, Output, Plugin, PluginBuilder, Sender, ShardCtx};

use super::session::player_limit;
use crate::server::StopRequest;

/// Hands each shard the server's [`StopRequest`].
pub(super) struct Stopping(pub StopRequest);

#[derive(Default)]
struct Installed(Option<StopRequest>);

impl Plugin for Stopping {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let stop = self.0.clone();
        plugin.later(0, move |ctx| ctx.state::<Installed>().0 = Some(stop));
    }
}

/// `/list`: the count against the server's limit, then the names, `, `-joined, in the order the
/// players came.
#[derive(Command)]
#[command(crate = "crate", name = "list", description = "Lists players on the server.")]
pub(crate) struct List;

impl Command for List {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let names: Vec<String> = ctx.players().map(|player| player.name).collect();
        let max = player_limit(ctx).map_or(names.len(), |limit| limit.get());
        out.translate("commands.players.list", vec![names.len().to_string(), max.to_string()], true);
        out.translate("commands.players.list.names", vec![names.join(", ")], true);
    }
}

/// `/stop`: answers, then asks the server to stop ([`crate::Server::stop_requested`]).
#[derive(Command)]
#[command(crate = "crate", name = "stop", description = "Stops the server.", permission = "operator")]
pub(crate) struct Stop;

impl Command for Stop {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(stop) = ctx.state::<Installed>().0.clone() else { return out.error("This server cannot be stopped by command") };
        out.translate("commands.stop.start", Vec::new(), true);
        stop.ask();
    }
}
