//! The access lists ([`AccessLists`]) on a server: logins checked against them, roles from
//! `permissions.json` given on join, and the commands that edit them.

use sapling_api::{JoinEvent, Permission, Plugin, PluginBuilder, Priority, ShardCtx};
use sapling_net::AccessLists;
use sapling_net::access::Role;

use super::allowlist::Allowlist;
use super::bans::{BanCommand, BanIp, Banlist, Pardon, PardonIp};
use crate::ServerBuilder;

impl ServerBuilder {
    /// Checks every login against `lists` (bans by name, XUID and IP, and the allowlist when it is
    /// on), gives joining players the role `permissions.json` has for them, and adds `/allowlist`
    /// (`/whitelist`), `/permission` (`/ops`), `/ban`, `/ban-ip`, `/pardon`, `/pardon-ip` and `/banlist` for operators.
    /// [`ServerBuilder::vanilla_commands`]' `/op` and `/deop` then save to `permissions.json`.
    pub fn access(mut self, lists: AccessLists) -> Self {
        self.net.access = Some(lists.clone());
        self.plugin(Access(lists))
    }
}

/// This shard's lists, for the commands; set on the shard's first tick.
#[derive(Default)]
struct Installed(Option<AccessLists>);

/// The lists the server keeps, if it keeps any.
pub(super) fn lists(ctx: &mut ShardCtx<'_>) -> Option<AccessLists> {
    ctx.state::<Installed>().0.clone()
}

struct Access(AccessLists);

impl Plugin for Access {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let lists = self.0.clone();
        plugin.later(0, move |ctx| ctx.state::<Installed>().0 = Some(lists));
        let lists = self.0.clone();
        // First, so that handlers after it see the role and may change it.
        plugin.on::<JoinEvent>(Priority::Highest, move |ctx, event| {
            let player = event.player();
            match lists.role(&player.xuid) {
                Some(Role::Operator) => ctx.set_permission(player.id, Permission::Operator),
                Some(Role::Member | Role::Visitor) => ctx.set_permission(player.id, Permission::Member),
                None => {}
            }
        });
        plugin.command::<Allowlist>().command::<super::permission::PermissionCommand>().command::<BanCommand>().command::<BanIp>().command::<Pardon>().command::<PardonIp>().command::<Banlist>();
    }
}
