use super::{Args, Command, CommandSpec, Output, ParseError, Sender};
use crate::{Permission, Player, ShardCtx};

type Runner = fn(&mut ShardCtx<'_>, &Sender, &mut Args<'_>, &mut Output) -> Result<(), ParseError>;

struct Entry {
    spec: CommandSpec,
    run: Runner,
    check: fn(&mut Args<'_>) -> Result<(), ParseError>,
}

fn run<C: Command>(ctx: &mut ShardCtx<'_>, sender: &Sender, args: &mut Args<'_>, out: &mut Output) -> Result<(), ParseError> {
    let command = C::parse(args)?;
    args.finish()?;
    command.run(ctx, sender, out);
    Ok(())
}

fn check<C: Command>(args: &mut Args<'_>) -> Result<(), ParseError> {
    C::parse(args)?;
    args.finish()
}

/// The commands of a server.
#[derive(Default)]
pub struct Commands {
    entries: Vec<Entry>,
}

impl Commands {
    /// Adds `C`, replacing a command of the same name.
    ///
    /// Panics if a required parameter follows an optional one: no client could call that.
    pub fn register<C: Command>(&mut self) {
        let spec = C::spec();
        for overload in &spec.overloads {
            let first_optional = overload.parameters.iter().position(|p| p.optional).unwrap_or(overload.parameters.len());
            assert!(overload.parameters[first_optional..].iter().all(|p| p.optional), "/{}: a required parameter follows an optional one", spec.name);
        }
        self.entries.retain(|e| e.spec.name != spec.name);
        self.entries.push(Entry { spec, run: run::<C>, check: check::<C> });
    }

    /// Whether `line` reads as a command a sender with `permission` may run, without running it:
    /// what BDS asks of each line of a function. A name that is nobody online passes.
    pub fn parses(&self, line: &str, permission: Permission) -> bool {
        matches!(self.check(line, permission), Ok(()) | Err(ParseError::NoTarget))
    }

    /// Why `line` would not run for a sender with `permission`, without running it: what its
    /// arguments meet, or a syntax error at its name where it names no command (as BDS answers
    /// `/execute run banana`). A name that is nobody online is [`ParseError::NoTarget`].
    pub fn check(&self, line: &str, permission: Permission) -> Result<(), ParseError> {
        let line = line.trim();
        let line = line.strip_prefix('/').unwrap_or(line);
        let name = line.split_whitespace().next().unwrap_or("");
        let sender = Sender::Console;
        let Some(entry) = self.find(name, permission) else { return Err(Args::new(line, 0, &sender, &[]).error_at(0)) };
        (entry.check)(&mut Args::new(line, name.len(), &sender, &[]))
    }

    /// The commands a player with `permission` may use, in the order they were added.
    pub fn visible(&self, permission: Permission) -> impl Iterator<Item = &CommandSpec> {
        self.entries.iter().map(|e| &e.spec).filter(move |spec| spec.permission <= permission)
    }

    fn find(&self, name: &str, permission: Permission) -> Option<&Entry> {
        let named = |spec: &CommandSpec| spec.name.eq_ignore_ascii_case(name) || spec.aliases.iter().any(|a| a.eq_ignore_ascii_case(name));
        self.entries.iter().find(|e| e.spec.permission <= permission && named(&e.spec))
    }

    /// Runs `line` (a leading `/` is optional) for `sender` and returns what to tell it. A member's
    /// line is read first: a syntax error is answered as such, then a selector anywhere in it is
    /// refused, then a command above it is refused by its name (research/captures/bds-member.txt).
    pub fn run(&self, ctx: &mut ShardCtx<'_>, sender: &Sender, permission: Permission, line: &str) -> Output {
        let line = line.trim();
        let line = line.strip_prefix('/').unwrap_or(line);
        let name = line.split_whitespace().next().unwrap_or("");
        let mut out = Output::default();
        let Some(entry) = self.find(name, Permission::Operator) else {
            out.translate("commands.generic.unknown", vec![name.to_owned()], false);
            return out;
        };
        // By who sends it: BDS showed other operators an operator's `/tell`, which anyone may use.
        if permission == Permission::Operator {
            out.echo();
        }
        let players: Vec<Player> = ctx.players().collect();
        if permission < Permission::Operator {
            match (entry.check)(&mut Args::new(line, name.len(), sender, &players)) {
                Ok(()) | Err(ParseError::NoTarget) => {}
                Err(error) => {
                    out.refuse(error);
                    return out;
                }
            }
            if super::selector::mentioned(&line[name.len()..]) {
                out.translate("commands.generic.permission.selector", Vec::new(), false);
                return out;
            }
            if entry.spec.permission > permission {
                out.translate("commands.generic.error.permissions", vec![entry.spec.name.clone()], false);
                return out;
            }
        }
        let mut args = Args::new(line, name.len(), sender, &players);
        if let Err(error) = (entry.run)(ctx, sender, &mut args, &mut out) {
            out.refuse(error);
        }
        out
    }
}
