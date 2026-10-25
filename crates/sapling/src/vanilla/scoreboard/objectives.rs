//! `/scoreboard objectives`.

use sapling_api::{Display, DisplaySlot, Objective, Output, ShardCtx, SortOrder};

use super::find_objective;

pub(super) fn add(ctx: &mut ShardCtx<'_>, out: &mut Output, name: String, display_name: Option<String>) {
    match ctx.add_objective(&Objective::dummy(name.clone(), display_name.unwrap_or_default())) {
        true => out.translate("commands.scoreboard.objectives.add.success", vec![name], true),
        false => out.translate("commands.scoreboard.objectives.add.alreadyExists", vec![name], false),
    }
}

pub(super) fn remove(ctx: &mut ShardCtx<'_>, out: &mut Output, name: &str) {
    match ctx.remove_objective(name) {
        true => out.translate("commands.scoreboard.objectives.remove.success", vec![name.to_owned()], true),
        false => out.translate("commands.scoreboard.objectiveNotFound", vec![name.to_owned()], false),
    }
}

/// By name, each line a success.
pub(super) fn list(ctx: &mut ShardCtx<'_>, out: &mut Output) {
    let mut objectives = ctx.objectives();
    if objectives.is_empty() {
        return out.translate("commands.scoreboard.objectives.list.empty", Vec::new(), false);
    }
    objectives.sort_by(|a, b| a.name.cmp(&b.name));
    out.translate("§a%commands.scoreboard.objectives.list.count", vec![objectives.len().to_string()], true);
    for Objective { name, display_name, criteria } in &objectives {
        out.translate("commands.scoreboard.objectives.list.entry", vec![name.clone(), display_name.clone(), criteria.clone()], true);
    }
    out.done_to(objectives.len() as u32);
}

pub(super) fn set_display(ctx: &mut ShardCtx<'_>, out: &mut Output, slot: DisplaySlot, shown: Option<(String, SortOrder)>) {
    let Some((objective, order)) = shown else {
        ctx.set_display(slot, None);
        return out.translate("commands.scoreboard.objectives.setdisplay.successCleared", vec![slot.name().into()], true);
    };
    if find_objective(ctx, out, &objective) {
        ctx.set_display(slot, Some(&Display { objective: objective.clone(), order }));
        out.translate("commands.scoreboard.objectives.setdisplay.successSet", vec![slot.name().into(), objective], true);
    }
}
