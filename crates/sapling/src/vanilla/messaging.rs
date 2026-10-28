//! `/me` and `/tellraw`, as BDS answered them (research/captures/bds-messaging.txt): both go
//! unanswered when they work; `/tellraw` sends each target its rawtext as a JsonWhisper.

use sapling_api::command::{Args, Parameter, RawPart, RawText, Selector};
use sapling_api::{Command, Output, Player, PlayerId, ScoreHolder, Sender, ShardCtx};

use super::chat::speaker;
use super::message::{Message, names};
use super::{no_target, players};

/// `text` with each selector replaced by the names it finds from `sender`, `, `-joined, and each
/// score by the scores of those it names, `*` being `reader`; either left out where it finds
/// none, as BDS did, but a `*` score without one is left for the client
/// (research/captures/bds-messaging.txt).
pub(super) fn resolved(ctx: &mut ShardCtx<'_>, sender: &Sender, text: &RawText, reader: Option<PlayerId>) -> RawText {
    let online: Vec<Player> = ctx.players().collect();
    text.resolve(&mut |part| {
        let written = match part {
            RawPart::Score { name, objective } if name == "*" => {
                let score = reader.and_then(|reader| ctx.score(&ScoreHolder::Player(reader), objective));
                return Some(score.map_or_else(|| part.clone(), |score| RawPart::Text(score.to_string())));
            }
            RawPart::Score { name, objective } => return ctx.score_text(sender, name, objective).map(RawPart::Text),
            RawPart::Selector(written) => written,
            _ => return None,
        };
        let selector = Selector::parse(&mut Args::new(written, 0, sender, &online)).ok()?;
        let names = names(ctx, sender, &selector);
        (!names.is_empty()).then(|| RawPart::Text(names.join(", ")))
    })
}

#[derive(Command)]
#[command(crate = "crate", name = "me", description = "Displays a message about yourself")]
pub(crate) struct Me {
    message: Option<Message>,
}

impl Command for Me {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match self.message {
            Some(message) => {
                let message = message.resolve(ctx, sender);
                ctx.emote(&speaker(sender), &message);
            }
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
        for player in told {
            let text = resolved(ctx, sender, &self.message, Some(player.id));
            ctx.tell_raw(player.id, &text);
        }
    }
}
