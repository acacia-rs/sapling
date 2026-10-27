//! `/loot spawn|give|insert|replace ... loot|kill|mine ...`: runs a loot table, a mob's death or a
//! block's break without killing or breaking anything, and puts what it made somewhere, answering
//! as BDS 1.26.52 does (research/captures/bds-lootcmd.txt, bds-lootcmd2.txt,
//! research/captures/bds-commands-findings.md "/loot").

mod made;
mod placed;

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Coords, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

enum Target {
    Spawn(Coords),
    Give(Selector),
    Insert(BlockCoords),
    Entity { target: Selector, slot_type: String, slot: i32, count: Option<i32> },
    Block { position: BlockCoords, slot: i32, count: Option<i32> },
}

enum Source {
    /// The table's path under `loot_tables/`, without `.json`.
    Table(String),
    Kill(Selector),
    Mine(BlockCoords),
}

pub(crate) struct Loot {
    target: Target,
    source: Source,
    /// `mainhand`, `offhand` or an item's name; BDS takes any word, an unknown item being no tool.
    tool: Option<String>,
}

const SLOT_TYPES: [&str; 8] = ["slot.hotbar", "slot.inventory", "slot.armor.head", "slot.armor.chest", "slot.armor.legs", "slot.armor.feet", "slot.weapon.mainhand", "slot.weapon.offhand"];
const CONTAINER: &str = "slot.container";

fn word_param(name: &str, optional: bool) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind: ParamKind::String, optional }
}

fn literal(args: &mut Args<'_>, word: &str) -> Result<(), ParseError> {
    let mark = args.mark();
    args.word()?.eq_ignore_ascii_case(word).then_some(()).ok_or_else(|| args.error_at(mark))
}

/// A slot's count, where the next word is a number and not the source.
fn count(args: &mut Args<'_>) -> Result<Option<i32>, ParseError> {
    match args.peek() {
        Some(next) if next.parse::<i32>().is_ok() => i32::parse(args).map(Some),
        _ => Ok(None),
    }
}

/// A table path: quoted, any; unquoted, BDS stops at the first `/` and refuses it.
fn table(args: &mut Args<'_>) -> Result<String, ParseError> {
    let mark = args.mark();
    let quoted = args.peek().is_some_and(|next| next.starts_with('"'));
    let path = args.word()?;
    match path.find('/') {
        Some(slash) if !quoted => Err(args.error_span(mark + slash, mark + slash + 1)),
        _ => Ok(path.to_owned()),
    }
}

impl CommandSyntax for Loot {
    fn spec() -> CommandSpec {
        let lit = |word: &str| ParameterSpec::literal("loot", word);
        let targets = [
            vec![lit("spawn"), ParameterSpec::of::<Coords>("position")],
            vec![lit("give"), ParameterSpec::of::<Selector>("players")],
            vec![lit("insert"), ParameterSpec::of::<BlockCoords>("position")],
            vec![lit("replace"), lit("entity"), ParameterSpec::of::<Selector>("entity"), word_param("slotType", false), ParameterSpec::of::<i32>("slotId"), ParameterSpec::of::<i32>("count")],
            vec![lit("replace"), lit("block"), ParameterSpec::of::<BlockCoords>("position"), lit(CONTAINER), ParameterSpec::of::<i32>("slotId"), ParameterSpec::of::<i32>("count")],
        ];
        let sources = [
            vec![lit("loot"), word_param("loot_table", false), word_param("tool", true)],
            vec![lit("kill"), ParameterSpec::of::<Selector>("entity"), word_param("tool", true)],
            vec![lit("mine"), ParameterSpec::of::<BlockCoords>("TargetBlockPosition"), word_param("tool", true)],
        ];
        let overloads = targets.iter().flat_map(|target| sources.iter().map(move |source| Overload { parameters: target.iter().chain(source).cloned().collect() })).collect();
        CommandSpec { name: "loot".into(), aliases: Vec::new(), description: "Drops the given loot table into the specified inventory or into the world.".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let target = match args.word()?.to_ascii_lowercase().as_str() {
            "spawn" => Target::Spawn(Coords::parse(args)?),
            "give" => Target::Give(Selector::parse(args)?),
            "insert" => Target::Insert(BlockCoords::parse(args)?),
            "replace" => {
                let mark = args.mark();
                match args.word()?.to_ascii_lowercase().as_str() {
                    "entity" => {
                        let target = Selector::parse(args)?;
                        let mark = args.mark();
                        let slot_type = args.word()?.to_ascii_lowercase();
                        if !SLOT_TYPES.contains(&slot_type.as_str()) {
                            return Err(args.error_at(mark));
                        }
                        Target::Entity { target, slot_type, slot: i32::parse(args)?, count: count(args)? }
                    }
                    "block" => {
                        let position = BlockCoords::parse(args)?;
                        literal(args, CONTAINER)?;
                        Target::Block { position, slot: i32::parse(args)?, count: count(args)? }
                    }
                    _ => return Err(args.error_at(mark)),
                }
            }
            _ => return Err(args.error_at(mark)),
        };
        let mark = args.mark();
        let source = match args.word()?.to_ascii_lowercase().as_str() {
            "loot" => Source::Table(table(args)?),
            "kill" => Source::Kill(Selector::parse(args)?),
            "mine" => Source::Mine(BlockCoords::parse(args)?),
            _ => return Err(args.error_at(mark)),
        };
        let tool = if args.is_empty() { None } else { Some(args.word()?.to_owned()) };
        args.finish()?;
        Ok(Self { target, source, tool })
    }
}

impl Command for Loot {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let tool = made::tool(ctx, sender, self.tool.as_deref());
        let Some(items) = made::items(ctx, sender, &self.source, tool.as_ref(), out) else { return };
        placed::put(ctx, sender, &self.target, &items, out);
    }
}

#[cfg(test)]
mod tests;
