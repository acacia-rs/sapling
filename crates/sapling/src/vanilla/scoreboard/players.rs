//! `/scoreboard players`. A name is the online player of that name or a fake player, `*` is
//! everyone with a score ([`ShardCtx::select_holders`]). One target answers with its name, more
//! with how many.

use std::hash::BuildHasher;

use sapling_api::command::Selector;
use sapling_api::{Output, ScoreHolder, ScoreOperation, Sender, ShardCtx};

use super::{Change, find_objective};
use crate::vanilla::no_target;

/// Those `target` names; `None`, answered, when it names nobody.
fn holders(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: &Selector) -> Option<Vec<ScoreHolder>> {
    let found = ctx.select_holders(sender, target);
    if found.is_empty() {
        no_target(out);
    }
    (!found.is_empty()).then_some(found)
}

/// Whether `target` is a name (not `*`) for one without any score, answered as BDS answers it.
fn unknown(ctx: &ShardCtx<'_>, out: &mut Output, target: &Selector, holders: &[ScoreHolder]) -> bool {
    let Selector::Named(name) = target else { return false };
    let unknown = name != "*" && holders.iter().all(|holder| ctx.scores(holder).is_empty());
    if unknown {
        out.translate("commands.scoreboard.players.list.player.empty", vec![name.clone()], false);
    }
    unknown
}

fn one_or_many(ctx: &ShardCtx<'_>, out: &mut Output, done: &[(ScoreHolder, i32)], objective: &str, multiple: (&str, Vec<String>)) {
    match done {
        [(holder, score)] => out.translate("commands.scoreboard.players.set.success", vec![objective.to_owned(), ctx.holder_name(holder), score.to_string()], true),
        _ => out.translate(multiple.0, multiple.1, true),
    }
}

pub(in crate::vanilla) fn change(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, change: Change, target: &Selector, objective: &str, count: i32) {
    if !find_objective(ctx, out, objective) {
        return;
    }
    let Some(holders) = holders(ctx, sender, out, target) else { return };
    let mut done = Vec::new();
    for holder in holders {
        let old = ctx.score(&holder, objective).unwrap_or(0);
        // BDS leaves a score that would overflow and answers nothing for it.
        let new = match change {
            Change::Set => Some(count),
            Change::Add => old.checked_add(count),
            Change::Remove => old.checked_sub(count),
        };
        if let Some(new) = new.filter(|new| ctx.set_score(&holder, objective, *new)) {
            done.push((holder, new));
        }
    }
    let (amount, objective, count_done) = (count.to_string(), objective.to_owned(), done.len().to_string());
    match (change, done.as_slice()) {
        (_, []) => {}
        (Change::Set, _) => one_or_many(ctx, out, &done, &objective, ("commands.scoreboard.players.set.multiple.success", vec![objective.clone(), count_done, amount])),
        (Change::Add | Change::Remove, [(holder, new)]) => {
            let key = if matches!(change, Change::Add) { "commands.scoreboard.players.add.success" } else { "commands.scoreboard.players.remove.success" };
            out.translate(key, vec![amount, objective, ctx.holder_name(holder), new.to_string()], true);
        }
        (Change::Add | Change::Remove, _) => {
            let key = if matches!(change, Change::Add) { "commands.scoreboard.players.add.multiple.success" } else { "commands.scoreboard.players.remove.multiple.success" };
            out.translate(key, vec![amount, objective, count_done], true);
        }
    }
}

/// Without a target the names of everyone with a score, `, `-joined under BDS's `%%s` line;
/// with one, each holder's scores.
pub(in crate::vanilla) fn list(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: Option<&Selector>) {
    let Some(target) = target else {
        let names: Vec<String> = ctx.score_holders().iter().map(|holder| ctx.holder_name(holder)).collect();
        if names.is_empty() {
            return out.translate("commands.scoreboard.players.list.empty", Vec::new(), false);
        }
        out.translate("§a%commands.scoreboard.players.list.count", vec![names.len().to_string()], true);
        return out.translate(vec!["%%s"; names.len()].join(", "), names, true);
    };
    let Some(holders) = holders(ctx, sender, out, target) else { return };
    if unknown(ctx, out, target, &holders) {
        return;
    }
    let mut entries = 0;
    for holder in holders {
        let scores = ctx.scores(&holder);
        if scores.is_empty() {
            continue;
        }
        out.translate("§a%commands.scoreboard.players.list.player.count", vec![scores.len().to_string(), ctx.holder_name(&holder)], true);
        for (objective, score) in &scores {
            let display_name = ctx.objective(objective).map_or_else(|| objective.clone(), |objective| objective.display_name);
            out.translate("commands.scoreboard.players.list.player.entry", vec![score.to_string(), display_name, objective.clone()], true);
        }
        entries += scores.len();
    }
    out.done_to(entries as u32);
}

/// BDS answered `*` for every holder, those without that score too.
pub(in crate::vanilla) fn reset(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: &Selector, objective: Option<&str>) {
    if objective.is_some_and(|objective| !find_objective(ctx, out, objective)) {
        return;
    }
    let Some(holders) = holders(ctx, sender, out, target) else { return };
    if unknown(ctx, out, target, &holders) {
        return;
    }
    for holder in &holders {
        ctx.reset_score(holder, objective);
        let name = ctx.holder_name(holder);
        match objective {
            Some(objective) => out.translate("commands.scoreboard.players.resetscore.success", vec![objective.to_owned(), name], true),
            None => out.translate("commands.scoreboard.players.reset.success", vec![name], true),
        }
    }
    out.done_to(holders.len() as u32);
}

pub(in crate::vanilla) fn test(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: &Selector, objective: &str, [min, max]: [i32; 2]) {
    if !find_objective(ctx, out, objective) {
        return;
    }
    let Some(holders) = holders(ctx, sender, out, target) else { return };
    if unknown(ctx, out, target, &holders) {
        return;
    }
    let mut passed = 0;
    for holder in holders {
        let Some(score) = ctx.score(&holder, objective) else {
            // Unverified on BDS: a holder with scores, none on this objective.
            out.translate("commands.scoreboard.players.score.notFound", vec![objective.to_owned(), ctx.holder_name(&holder)], false);
            continue;
        };
        let holds = (min..=max).contains(&score);
        passed += u32::from(holds);
        let key = if holds { "commands.scoreboard.players.test.success" } else { "commands.scoreboard.players.test.failed" };
        out.translate(key, vec![score.to_string(), min.to_string(), max.to_string()], holds);
    }
    out.done_to(passed);
}

pub(in crate::vanilla) fn random(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, target: &Selector, objective: &str, [min, max]: [i32; 2]) {
    if min >= max {
        return out.translate("commands.scoreboard.players.random.invalidRange", vec![min.to_string(), max.to_string()], false);
    }
    if !find_objective(ctx, out, objective) {
        return;
    }
    let Some(holders) = holders(ctx, sender, out, target) else { return };
    let span = (i64::from(max) - i64::from(min) + 1) as u64;
    let mut done = Vec::new();
    for (index, holder) in holders.into_iter().enumerate() {
        let roll = std::collections::hash_map::RandomState::new().hash_one((ctx.tick(), index));
        let score = (i64::from(min) + (roll % span) as i64) as i32;
        if ctx.set_score(&holder, objective, score) {
            done.push((holder, score));
        }
    }
    let multiple = ("commands.scoreboard.players.operation.success", vec![objective.to_owned(), done.len().to_string()]);
    one_or_many(ctx, out, &done, objective, multiple);
}

/// Each target's score with each source's in turn; a target without a score starts at 0, a
/// source without one is passed over, and a target no source reached fails.
pub(in crate::vanilla) fn operation(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, (target, objective): (&Selector, &str), operation: ScoreOperation, (source, source_objective): (&Selector, &str)) {
    if !find_objective(ctx, out, objective) || !find_objective(ctx, out, source_objective) {
        return;
    }
    let Some(sources) = holders(ctx, sender, out, source) else { return };
    let unknown_source = unknown(ctx, out, source, &sources);
    let Some(targets) = holders(ctx, sender, out, target) else { return };
    let mut done = Vec::new();
    for holder in targets.into_iter().filter(|_| !unknown_source) {
        let mut now = None;
        for from in &sources {
            let Some(given) = ctx.score(from, source_objective) else { continue };
            let (mine, theirs) = operation.apply(ctx.score(&holder, objective).unwrap_or(0), given);
            ctx.set_score(&holder, objective, mine);
            if operation == ScoreOperation::Swap {
                ctx.set_score(from, source_objective, theirs);
            }
            now = Some(mine);
        }
        if let Some(now) = now {
            done.push((holder, now));
        }
    }
    if done.is_empty() {
        return out.translate("commands.scoreboard.allMatchesFailed", Vec::new(), false);
    }
    let multiple = ("commands.scoreboard.players.operation.success", vec![objective.to_owned(), done.len().to_string()]);
    one_or_many(ctx, out, &done, objective, multiple);
}
