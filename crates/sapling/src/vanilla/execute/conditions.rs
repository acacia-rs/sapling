//! `/execute if|unless`: block, blocks, entity and score tests, and what BDS adds to the answer
//! where a test cannot be made (research/captures/bds-execute-if.txt).

use sapling_api::command::Selector;
use sapling_api::{Block, Output, ScoreHolder, Sender, ShardCtx};

use super::parse::{Compare, Condition, Score};
use crate::vanilla::blocks::{block, has_states};
use crate::vanilla::copying::matched;
use crate::vanilla::regions::{MOST_BLOCKS, Region};

/// Lowest and highest block of each dimension, as `if block` checks a place.
const HEIGHTS: [(i32, i32); 3] = [(-64, 319), (0, 127), (0, 255)];

/// What a test found: whether it held, and how many blocks `if blocks` compared. `None` where it
/// could not be made, the reason told to `out`; that fails `if` and `unless` alike.
pub(super) fn test(ctx: &mut ShardCtx<'_>, sender: &Sender, dimension: i32, condition: &Condition, out: &mut Output) -> Option<(bool, Option<u32>)> {
    match condition {
        Condition::Block(coords, name, states) => {
            let pos = coords.resolve(ctx, sender);
            let (low, high) = HEIGHTS[dimension.clamp(0, 2) as usize];
            if !(low..=high).contains(&pos[1]) {
                out.translate("commands.execute.outRangedDetectPosition", pos.map(|c| c.to_string()).to_vec(), false);
                return None;
            }
            let there = ctx.block(pos).unwrap_or_else(Block::air);
            let wanted = block(&name.0, states.as_ref());
            Some((there.name == wanted.name && states.as_ref().is_none_or(|states| has_states(&there, &states.0)), None))
        }
        Condition::Blocks([begin, end, destination], masked) => {
            let [begin, end] = [begin, end].map(|corner| corner.resolve(ctx, sender));
            // BDS counts the box by its corners' differences: 41 blocks a side answered 64000.
            let counted: i64 = (0..3).map(|axis| i64::from((end[axis] - begin[axis]).abs())).product();
            if counted > MOST_BLOCKS {
                out.translate("commands.execute.ifUnlessBLocks.tooManyBlocks", vec![MOST_BLOCKS.to_string(), counted.to_string()], false);
                return None;
            }
            let source = Region::between(begin, end);
            let found = matched(ctx, source, source.moved_to(destination.resolve(ctx, sender)), *masked);
            Some((found.is_some(), found))
        }
        Condition::Entity(selector) => Some((!ctx.select(sender, selector).is_empty(), None)),
        Condition::Score(target, compare) => {
            let score = score_of(ctx, sender, target, out)?;
            let held = match compare {
                Compare::Matches(range) => score.is_some_and(|score| range.holds(score)),
                Compare::Against(order, source) => {
                    let other = score_of(ctx, sender, source, out)?;
                    score.zip(other).is_some_and(|(left, right)| order.holds(left, right))
                }
            };
            Some((held, None))
        }
    }
}

/// The score of the one holder `score` names, `None` where it has none; `None` outside, told to
/// `out`, where it names nobody or several or the objective does not exist.
fn score_of(ctx: &mut ShardCtx<'_>, sender: &Sender, score: &Score, out: &mut Output) -> Option<Option<i32>> {
    let mut holders = ctx.select_holders(sender, &score.target);
    // A name is somebody only once it has a score.
    if matches!(&score.target, Selector::Named(name) if name != "*") {
        holders.retain(|holder| !ctx.scores(holder).is_empty());
    }
    let failed = |out: &mut Output, key: &str, parameter: String| {
        out.translate(format!("commands.execute.ifUnlessScore.{key}"), vec![parameter], false);
        None
    };
    let holder: ScoreHolder = match holders.len() {
        0 => return failed(out, "targets.empty", score.typed.clone()),
        1 => holders.remove(0),
        many => {
            failed(out, "targets.tooMany", many.to_string());
            out.translate("commands.selector.error.moreThanOne", Vec::new(), false);
            return None;
        }
    };
    if ctx.objective(&score.objective).is_none() {
        return failed(out, "objectiveNotFound", score.objective.clone());
    }
    Some(ctx.score(&holder, &score.objective))
}
