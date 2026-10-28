//! `/say` and `/tell`, as BDS answered them (research/captures/bds-feedback.txt): `/say` is
//! answered by its announcement alone; `/tell` whispers to each target and answers its sender even
//! where `sendCommandFeedback` is off. Selectors in a message are resolved (message.rs).

use sapling_api::command::{Selected, Selector};
use sapling_api::{Command, Output, Sender, ShardCtx};

use super::message::Message;
use super::{display_name, no_target, players};

/// Who a line is from: BDS's console is `Server`, a mob an `/execute as` names its kind's name
/// (research/captures/bds-execute.txt: `[Cow] moo`).
pub(super) fn speaker(sender: &Sender) -> String {
    match sender {
        Sender::Executed(executed) => match &executed.executor {
            Some(Selected::Entity(entity)) => display_name(entity),
            Some(Selected::Player(player)) => player.name.clone(),
            None => speaker(&executed.by),
        },
        _ => sender.player().map_or_else(|| "Server".to_owned(), |player| player.name.clone()),
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "say", description = "Sends a message in the chat to other players", permission = "operator")]
pub(crate) struct Say {
    message: Message,
}

impl Command for Say {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let message = self.message.resolve(ctx, sender);
        ctx.announce(&speaker(sender), &message);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "tell", aliases = ["w", "msg"], description = "Sends a private message to one or more players")]
pub(crate) struct Tell {
    target: Selector,
    message: Message,
}

impl Command for Tell {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let told = players(ctx, sender, Some(&self.target));
        if told.is_empty() {
            return no_target(out);
        }
        let from = speaker(sender);
        let message = self.message.resolve(ctx, sender);
        told.iter().for_each(|player| ctx.whisper(player.id, &from, &message));
        let names = told.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ");
        out.translate("commands.message.display.outgoing", vec![names, message], true);
        out.done_to(told.len() as u32);
        out.always_answer();
    }
}
