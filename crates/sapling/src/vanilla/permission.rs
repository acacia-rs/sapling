//! `/permission` (`/ops`) over `permissions.json`, as BDS's permissions command answers (bdsre
//! 0x143ae6b60, docs/research/server-leftovers.md): `reload` reads the file again and gives the
//! players online their roles; `list` prints the operators' XUIDs as BDS's tagged JSON.

use sapling_api::{Command, Output, Permission, Sender, ShardCtx};
use sapling_net::access::Role;

use super::access::lists;

#[derive(Command)]
#[command(crate = "crate", name = "permission", aliases = ["ops"], description = "Reloads and applies permissions.", permission = "operator")]
pub(crate) enum PermissionCommand {
    List,
    Reload,
}

impl Command for PermissionCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let lists = lists(ctx).filter(|lists| lists.on_disk());
        match (self, lists) {
            (Self::List, Some(lists)) => out.print(tagged_json("ops", &lists.operators())),
            (Self::List, None) => out.translate("command.permissions.list.fail.filenotfound", Vec::new(), false),
            (Self::Reload, Some(lists)) if lists.reload_permissions().is_ok() => {
                for player in ctx.players().collect::<Vec<_>>() {
                    match lists.role(&player.xuid) {
                        Some(Role::Operator) => ctx.set_permission(player.id, Permission::Operator),
                        Some(Role::Member | Role::Visitor) => ctx.set_permission(player.id, Permission::Member),
                        None => {}
                    }
                }
                out.translate("commands.ops.reloaded", Vec::new(), true);
                out.translate("commands.permissions.reloaded", Vec::new(), true);
            }
            (Self::Reload, _) => out.translate("command.permissions.reload.fail.filenotfound", Vec::new(), false),
        }
    }
}

/// BDS's machine-readable answer: `###* {"command":..,"result":..}` and ` *###` on the next line.
fn tagged_json(command: &str, result: &[String]) -> String {
    format!("###* {}\n *###", serde_json::json!({ "command": command, "result": result }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_tagged_json_of_xuids() {
        assert_eq!(tagged_json("ops", &["2535413803071674".into()]), "###* {\"command\":\"ops\",\"result\":[\"2535413803071674\"]}\n *###");
    }
}
