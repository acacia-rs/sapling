use super::{Args, Command, CommandSpec, Output, ParseError, Sender};
use crate::{Permission, Player, ShardCtx};

type Runner = fn(&mut ShardCtx<'_>, &Sender, &mut Args<'_>, &mut Output) -> Result<(), ParseError>;

struct Entry {
    spec: CommandSpec,
    run: Runner,
}

fn run<C: Command>(ctx: &mut ShardCtx<'_>, sender: &Sender, args: &mut Args<'_>, out: &mut Output) -> Result<(), ParseError> {
    let command = C::parse(args)?;
    args.finish()?;
    command.run(ctx, sender, out);
    Ok(())
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
        self.entries.push(Entry { spec, run: run::<C> });
    }

    /// The commands a player with `permission` may use, in the order they were added.
    pub fn visible(&self, permission: Permission) -> impl Iterator<Item = &CommandSpec> {
        self.entries.iter().map(|e| &e.spec).filter(move |spec| spec.permission <= permission)
    }

    fn find(&self, name: &str, permission: Permission) -> Option<&Entry> {
        let named = |spec: &CommandSpec| spec.name.eq_ignore_ascii_case(name) || spec.aliases.iter().any(|a| a.eq_ignore_ascii_case(name));
        self.entries.iter().find(|e| e.spec.permission <= permission && named(&e.spec))
    }

    /// Runs `line` (a leading `/` is optional) for `sender` and returns what to tell it. A command
    /// the sender may not use is as unknown as one that does not exist.
    pub fn run(&self, ctx: &mut ShardCtx<'_>, sender: &Sender, permission: Permission, line: &str) -> Output {
        let line = line.trim();
        let line = line.strip_prefix('/').unwrap_or(line);
        let name = line.split_whitespace().next().unwrap_or("");
        let mut out = Output::default();
        let Some(entry) = self.find(name, permission) else {
            out.translate("commands.generic.unknown", vec![name.to_owned()], false);
            return out;
        };
        let players: Vec<Player> = ctx.players().collect();
        let mut args = Args::new(line, name.len(), sender, &players);
        if let Err(error) = (entry.run)(ctx, sender, &mut args, &mut out) {
            out.refuse(error);
        }
        out
    }
}
