//! `/me` and `/tellraw`, as BDS answered them (research/captures/bds-messaging.txt): both go
//! unanswered when they work; `/tellraw` sends each target its rawtext as a JsonWhisper.

use sapling_api::command::{Args, Parameter, RawPart, RawText, Selector, Text};
use sapling_api::{Command, Output, Player, Sender, ShardCtx};

use super::chat::speaker;
use super::{display_name, no_target, players, targets};

/// `text` with each selector replaced by the names it finds from `sender`, `, `-joined, and left
/// out where it finds none, as BDS did.
// TODO: scores are always left out: sapling has no scoreboard. BDS left a `*` score raw for a
// reader without a score and dropped the others it could not find.
pub(super) fn resolved(ctx: &mut ShardCtx<'_>, sender: &Sender, text: &RawText) -> RawText {
    let online: Vec<Player> = ctx.players().collect();
    text.resolve(&mut |part| {
        let RawPart::Selector(written) = part else { return None };
        let selector = Selector::parse(&mut Args::new(written, 0, sender, &online)).ok()?;
        let (players, entities) = targets(ctx, sender, Some(&selector));
        let found = players.into_iter().map(|player| player.name);
        let names: Vec<String> = found.chain(entities.iter().map(|entity| display_name(entity))).collect();
        (!names.is_empty()).then(|| RawPart::Text(names.join(", ")))
    })
}

#[derive(Command)]
#[command(crate = "crate", name = "me", description = "Displays a message about yourself")]
pub(crate) struct Me {
    message: Option<Text>,
}

impl Command for Me {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match self.message {
            Some(message) => ctx.emote(&speaker(sender), &message.0),
            None => out.translate("commands.generic.invalidMessage", Vec::new(), false),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "tellraw", description = "Sends a JSON message to players", permission = "operator")]
pub(crate) struct TellRaw {
    target: Selector,
    message: RawText,
}

impl Command for TellRaw {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let told = players(ctx, sender, Some(&self.target));
        if told.is_empty() {
            return no_target(out);
        }
        let text = resolved(ctx, sender, &self.message);
        told.iter().for_each(|player| ctx.tell_raw(player.id, &text));
    }
}
