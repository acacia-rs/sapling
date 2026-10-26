//! `/scriptevent <id> [message]`, answered as BDS answered it (research/captures/bds-functions.txt).
//! BDS hands the event to its Script API; sapling raises a [`ScriptEvent`] for plugins instead.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, ParameterSpec, ParseError};
use sapling_api::{Command, Output, Permission, ScriptEvent, Sender, ShardCtx};

// TODO: the Script API's limit as remembered; BDS's own was not read.
const MAX_MESSAGE: usize = 2048;

pub(crate) struct ScriptEventCommand {
    id: String,
    message: String,
}

impl CommandSyntax for ScriptEventCommand {
    fn spec() -> CommandSpec {
        let parameters = vec![ParameterSpec { name: "messageId".into(), kind: ParamKind::String, optional: false }, ParameterSpec { name: "message".into(), kind: ParamKind::Text, optional: false }];
        CommandSpec { name: "scriptevent".into(), aliases: Vec::new(), description: "Triggers a script event with an ID and message.".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let id = args.word()?.to_owned();
        let message = if args.is_empty() { String::new() } else { args.rest()?.to_owned() };
        Ok(Self { id, message })
    }
}

/// Whether `id` is `namespace:name`, both there, in a namespace not the game's own.
fn namespaced(id: &str) -> bool {
    id.split_once(':').is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty() && namespace != "minecraft")
}

impl Command for ScriptEventCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        if !namespaced(&self.id) {
            return out.translate("commands.scriptevent.invalidNamespace", Vec::new(), false);
        }
        if self.message.chars().count() > MAX_MESSAGE {
            return out.translate("commands.scriptevent.messageSizeExceed", vec![MAX_MESSAGE.to_string()], false);
        }
        // BDS's own words, untranslated, `%%s` and all.
        out.translate("Script event %%s has been sent", vec![self.id.clone()], true);
        ctx.send_script_event(ScriptEvent { id: self.id, message: self.message, sender: sender.clone() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_needs_a_namespace_of_its_own() {
        assert!(namespaced("ns:hi"));
        assert!(!namespaced("hi") && !namespaced("minecraft:hi") && !namespaced("ns:") && !namespaced(":hi"));
        let sender = Sender::Console;
        let parsed = |line: &str| ScriptEventCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|c| (c.id, c.message)).ok();
        assert_eq!(parsed("ns:hi hello world"), Some(("ns:hi".into(), "hello world".into())));
        assert_eq!(parsed("ns:hi"), Some(("ns:hi".into(), String::new())));
    }
}
