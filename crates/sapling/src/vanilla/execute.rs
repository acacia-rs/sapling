//! `/execute` as BDS runs it (research/captures/bds-execute.txt, bds-execute-if.txt,
//! bds-execute-run.txt). The whole line is read first, the command after `run` too: a word wrong
//! anywhere is a syntax error of the `/execute` line. Then each sub-command in turn changes every
//! frame (frames.rs) or drops those that fail a test (conditions.rs), and the command runs once per
//! frame left as [`Sender::Executed`]. Its answers are passed on, `commands.execute.failed` after
//! each that failed; without `run`, each frame left answers `trueCondition`.

mod conditions;
mod frames;
mod parse;

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, ParamKind, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::chat::speaker;
use super::no_target;
use frames::Frame;
use parse::{Condition, Step, DIMENSIONS};

/// Commands an `/execute` may not run: BDS runs its commands below the level that may change who
/// is an operator (bds-execute-run.txt `execute run op`).
// TODO: only `op` was drilled; BDS's other commands of that level are not known here.
const ABOVE_EXECUTE: [&str; 2] = ["op", "deop"];

pub(crate) struct Execute {
    steps: Vec<Step>,
    /// The command after `run` and where it starts in `line`.
    run: Option<(String, usize)>,
    line: String,
}

impl CommandSyntax for Execute {
    fn spec() -> CommandSpec {
        CommandSpec {
            name: "execute".into(),
            aliases: Vec::new(),
            description: "Executes a command on behalf of one or more entities".into(),
            permission: Permission::Operator,
            overloads: overloads(),
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (steps, run) = parse::steps(args)?;
        Ok(Self { steps, run, line: args.line().to_owned() })
    }
}

/// BDS's overloads (research/captures/bds-command-syntax.txt), each sub-command followed by the
/// rest of the line: sapling's list cannot chain them.
fn overloads() -> Vec<Overload> {
    let word = |word: &str| ParameterSpec::literal("execute", word);
    let words = |name: &str, values: &[&str]| ParameterSpec { name: name.into(), kind: ParamKind::Enum { name: name.into(), values: values.iter().map(|v| v.to_string()).collect() }, optional: false };
    let of = |name: &str, kind: ParamKind| ParameterSpec { name: name.into(), kind, optional: false };
    let anchor = || words("ActorLocation", &["eyes", "feet"]);
    let chained = |optional: bool| ParameterSpec { name: "chainedCommand".into(), kind: ParamKind::Text, optional };
    let test = |kind: &str| vec![words("Option_If_Unless", &["if", "unless"]), word(kind)];
    let score = || vec![ParameterSpec::of::<Selector>("target"), of("objective", ParamKind::String)];
    let then = |mut first: Vec<ParameterSpec>, optional: bool| {
        first.push(chained(optional));
        Overload { parameters: first }
    };
    vec![
        then(vec![word("as"), ParameterSpec::of::<Selector>("origin")], false),
        then(vec![word("at"), ParameterSpec::of::<Selector>("origin")], false),
        then(vec![word("in"), words("Dimension", &DIMENSIONS)], false),
        then(vec![word("positioned"), ParameterSpec::of::<Coords>("position")], false),
        then(vec![word("positioned"), word("as"), ParameterSpec::of::<Selector>("origin")], false),
        then(vec![word("rotated"), of("yaw", ParamKind::Value), of("pitch", ParamKind::Value)], false),
        then(vec![word("rotated"), word("as"), ParameterSpec::of::<Selector>("origin")], false),
        then(vec![word("facing"), ParameterSpec::of::<Coords>("position")], false),
        then(vec![word("facing"), word("entity"), ParameterSpec::of::<Selector>("origin"), anchor()], false),
        then(vec![word("align"), of("axes", ParamKind::String)], false),
        then(vec![word("anchored"), anchor()], false),
        then([test("block"), vec![of("position", ParamKind::BlockPosition), of("block", ParamKind::String)]].concat(), true),
        then([test("blocks"), ["begin", "end", "destination"].map(|name| of(name, ParamKind::BlockPosition)).to_vec(), vec![words("BlocksScanMode", &["masked", "all"])]].concat(), true),
        then([test("entity"), vec![ParameterSpec::of::<Selector>("target")]].concat(), true),
        then([test("score"), score(), vec![of("operation", ParamKind::String)], score()].concat(), true),
        then([test("score"), score(), vec![words("ScoreRangeMode", &["matches"]), of("range", ParamKind::String)]].concat(), true),
        Overload { parameters: vec![word("run"), of("command", ParamKind::Text)] },
    ]
}

impl Command for Execute {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        if let Some(error) = self.refused(ctx, sender) {
            return out.refuse(error);
        }
        let mut frames = vec![(Frame::of(ctx, sender), None)];
        for step in &self.steps {
            frames = match step {
                Step::If(wanted, condition) => test(ctx, frames, *wanted, condition, out),
                _ => {
                    let mut next = Vec::new();
                    for (frame, _) in frames {
                        next.extend(frames::apply(ctx, step, frame).into_iter().map(|frame| (frame, None)));
                    }
                    if next.is_empty() {
                        return no_target(out);
                    }
                    next
                }
            };
            if frames.is_empty() {
                return;
            }
        }
        let Some((command, _)) = &self.run else {
            for (_, count) in &frames {
                match count {
                    Some(count) => out.translate("commands.execute.trueConditionWithCount", vec![count.to_string()], true),
                    None => out.translate("commands.execute.trueCondition", Vec::new(), true),
                }
            }
            return out.done_to(frames.len() as u32);
        };
        let mut done = 0;
        for (frame, _) in frames {
            done += run_in(ctx, &frame, command, out);
        }
        out.done_to(done);
    }
}

impl Execute {
    /// The syntax error of the command after `run`, placed in the whole line as BDS places it.
    fn refused(&self, ctx: &ShardCtx<'_>, sender: &Sender) -> Option<ParseError> {
        let (command, at) = self.run.as_ref()?;
        let error = match ctx.commands().check(command, Permission::Operator) {
            Ok(()) => self.nested_refusal(ctx, sender)?,
            Err(ParseError::NoTarget) => return None,
            Err(error) => error,
        };
        match error {
            ParseError::Syntax { before, token, after } => {
                // The error keeps only ten characters each side; find them in the command again.
                let quoted = format!("/{command}");
                let found = quoted.find(&format!("{before}{token}{after}")).map_or(command.len() - token.len(), |pos| pos + before.len() - 1);
                let start = at + found;
                Some(Args::new(&self.line, 0, sender, &[]).error_span(start, start + token.len()))
            }
            other => Some(other),
        }
    }

    /// The refusal of an `/execute` this one runs: BDS refuses the whole line, with no
    /// `commands.execute.failed` (bds-execute2.txt `execute run execute run banana`).
    fn nested_refusal(&self, ctx: &ShardCtx<'_>, sender: &Sender) -> Option<ParseError> {
        let (command, _) = self.run.as_ref()?;
        let name = command.split_whitespace().next()?;
        if !name.eq_ignore_ascii_case("execute") {
            return None;
        }
        let mut args = Args::new(command, name.len(), sender, &[]);
        Self::parse(&mut args).ok()?.refused(ctx, sender)
    }
}

/// The frames that pass a test, each with how many blocks `if blocks` counted; each that does not
/// is answered `falseCondition`.
fn test(ctx: &mut ShardCtx<'_>, frames: Vec<(Frame, Option<u32>)>, wanted: bool, condition: &Condition, out: &mut Output) -> Vec<(Frame, Option<u32>)> {
    let mut kept = Vec::new();
    for (frame, _) in frames {
        let (sender, dimension) = (frame.sender(), frame.executed.dimension);
        match in_dimension(ctx, dimension, |ctx| conditions::test(ctx, &sender, dimension, condition, out)) {
            Some((held, count)) if held == wanted => kept.push((frame, count)),
            _ => {
                let kind = match condition {
                    Condition::Block(..) => "block",
                    Condition::Blocks(..) => "blocks",
                    Condition::Entity(_) => "entity",
                    Condition::Score(..) => "score",
                };
                let mode = if wanted { "if" } else { "unless" };
                out.translate("commands.execute.falseCondition", vec![mode.into(), kind.into()], false);
            }
        }
    }
    kept
}

/// Runs `command` in `frame`, passes its answer on and returns how many it acted on.
fn run_in(ctx: &mut ShardCtx<'_>, frame: &Frame, command: &str, out: &mut Output) -> u32 {
    let sender = frame.sender();
    let name = command.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    let failed = |out: &mut Output| out.translate("commands.execute.failed", vec![name.clone(), speaker(&sender)], false);
    if ABOVE_EXECUTE.contains(&name.as_str()) {
        out.translate("commands.generic.error.permissions", vec![name.clone()], false);
        failed(out);
        return 0;
    }
    let commands = ctx.commands();
    let answer = in_dimension(ctx, frame.executed.dimension, |ctx| commands.run(ctx, &sender, Permission::Operator, command));
    for line in answer.messages() {
        out.translate(line.text.clone(), line.parameters.clone(), line.success);
    }
    if answer.answers_always() {
        out.always_answer();
    }
    let done = answer.success_count();
    if done == 0 && !answer.messages().is_empty() {
        failed(out);
    }
    done
}

/// Runs `f` with the world of `dimension` that the world in scope belongs to in scope; in the
/// world in scope where that dimension is not on this shard yet.
fn in_dimension<R>(ctx: &mut ShardCtx<'_>, dimension: i32, f: impl FnOnce(&mut ShardCtx<'_>) -> R) -> R {
    let world = ctx.world();
    let (base, here) = match world.rsplit_once(':') {
        Some((base, suffix)) if DIMENSIONS.contains(&suffix) => (base.to_owned(), DIMENSIONS.iter().position(|one| *one == suffix).unwrap_or(0) as i32),
        _ => (world.clone(), 0),
    };
    if here == dimension {
        return f(ctx);
    }
    let name = if dimension == 0 { base } else { format!("{base}:{}", DIMENSIONS[dimension.clamp(0, 2) as usize]) };
    let mut f = Some(f);
    match ctx.within(&name, |ctx| f.take().map(|f| f(ctx))) {
        Some(Some(result)) => result,
        _ => f.take().map(|f| f(ctx)).expect("not run in the other world"),
    }
}

#[cfg(test)]
mod tests;
