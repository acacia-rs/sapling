//! `/allowlist` (`/whitelist`), as BDS's: `add` and `remove` take a gamertag, whose XUID is filled
//! in when the player first joins; `on` and `off` are not saved (bedrock_server_how_to.html).

use sapling_api::{Command, Output, Sender, ShardCtx};
use sapling_net::access::AllowEntry;

use super::access::lists;

#[derive(Command)]
#[command(crate = "crate", name = "allowlist", aliases = ["whitelist"], description = "Manages the server allowlist.", permission = "operator")]
pub(crate) enum Allowlist {
    Add { name: String },
    Remove { name: String },
    List,
    On,
    Off,
    Reload,
}

impl Command for Allowlist {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(lists) = lists(ctx) else { return out.error("The server keeps no allowlist") };
        match self {
            Self::Add { name } if lists.allow(AllowEntry::named(name.clone())) => out.translate("commands.allowlist.add.success", vec![name], true),
            Self::Add { name } => out.translate("commands.allowlist.add.failed", vec![name], false),
            Self::Remove { name } if lists.disallow(&name) => out.translate("commands.allowlist.remove.success", vec![name], true),
            Self::Remove { name } => out.translate("commands.allowlist.remove.failed", vec![name], false),
            Self::List => {
                let names: Vec<String> = lists.allowlist().into_iter().map(|entry| entry.name).collect();
                let count = names.len().to_string();
                out.translate("commands.allowlist.list", vec![count.clone(), count], true);
                out.print(names.join(", "));
            }
            Self::On => {
                lists.set_allowlist_enabled(true);
                out.translate("commands.allowlist.enabled", Vec::new(), true);
            }
            Self::Off => {
                lists.set_allowlist_enabled(false);
                out.translate("commands.allowlist.disabled", Vec::new(), true);
            }
            Self::Reload => match lists.reload_allowlist() {
                Ok(()) => out.translate("commands.allowlist.reloaded", Vec::new(), true),
                Err(error) => out.error(error.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use sapling_api::command::{Args, CommandSyntax};

    use super::*;

    #[test]
    fn allowlist_takes_what_bds_takes() {
        let sender = Sender::Console;
        let allowlist = |line: &str| Allowlist::parse(&mut Args::new(line, 0, &sender, &[])).ok();
        // A gamertag with a space is quoted (bedrock_server_how_to.html).
        assert!(matches!(allowlist("add \"Example Name\""), Some(Allowlist::Add { name }) if name == "Example Name"));
        assert!(matches!(allowlist("REMOVE Steve"), Some(Allowlist::Remove { .. })) && matches!(allowlist("reload"), Some(Allowlist::Reload)));
        assert!(allowlist("add").is_none() && allowlist("banana").is_none());
        assert_eq!(Allowlist::spec().aliases, ["whitelist"]);
    }
}
