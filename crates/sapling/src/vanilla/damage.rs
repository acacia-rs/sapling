//! `/damage <target> <amount> [cause]`, for players and mobs (research/captures/bds-hurt.txt,
//! bds-mob-effects.txt).
// TODO: what BDS answers an unknown cause was not recorded.

use sapling_api::command::Selector;
use sapling_api::{Command, DamageCause, Output, Sender, ShardCtx};

use super::{display_name, entities, players};

#[derive(Command)]
#[command(crate = "crate", name = "damage", description = "Apply damage to the specified entities", permission = "operator")]
pub(crate) struct Damage {
    target: Selector,
    amount: i32,
    cause: Option<String>,
}

impl Command for Damage {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (found, mobs) = (players(ctx, sender, &self.target), entities(ctx, &self.target));
        let names = || found.iter().map(|player| player.name.clone()).chain(mobs.iter().map(|mob| display_name(&mob.kind))).collect::<Vec<_>>().join(", ");
        // BDS fails `suffocation` for anyone (research/captures/bds-worn.txt).
        let Some(cause) = DamageCause::named(self.cause.as_deref().unwrap_or("none")).filter(|cause| *cause != DamageCause::Suffocation) else {
            return out.translate("commands.damage.failed", vec![names()], false);
        };
        let amount = self.amount as f32;
        let mut hurt: Vec<String> = found.iter().filter(|player| ctx.hurt_player(player.id, amount, cause)).map(|player| player.name.clone()).collect();
        hurt.extend(mobs.iter().filter(|mob| ctx.hurt_entity(mob.id, amount, cause)).map(|mob| display_name(&mob.kind)));
        match hurt.is_empty() {
            true => out.translate("commands.damage.failed", vec![names()], false),
            false => out.translate("commands.damage.success", vec![hurt.join(", ")], true),
        }
    }
}
