//! `/locate` and `/place`, over the world generator (sapling_api::GenerationBackend), answered as
//! BDS answered (research/captures/bds-locate.txt, bds-place.txt): a place is named by its block
//! and its distance, rounded down, from the sender.
//! `/place` takes BDS's names and options (bds-t1dplace.txt); a player of BDS may not run it (admin).
// TODO: a feature rule is not placed: BDS answered nearly every rule `invalidposition`, on a flat world
// and a generated one, but placed `overworld_underground_dirt_feature` at the feet (bds-u3place.txt).

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, parse_choice};
use sapling_api::{Command, Output, Permission, PlaceError, PlaceOptions, Sender, ShardCtx};

use super::structure::heights;

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
}

/// The distance BDS shows: across, from the sender's block, rounded down (research/captures/bds-bb1biome.txt:
/// 1312 to (-928, -928) from feet at (0.5, 0.5)).
fn distance(from: [i32; 3], [x, z]: [i32; 2]) -> String {
    (f64::from(x - from[0]).hypot(f64::from(z - from[2])).floor() as i64).to_string()
}

pub(crate) enum Locate {
    Structure(String),
    /// A biome name BDS takes only with its namespace.
    Biome(String),
}

impl CommandSyntax for Locate {
    fn spec() -> CommandSpec {
        let named = |verb: &str, name: &str| vec![ParameterSpec::literal("locate", verb), ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false }];
        let structure = [named("structure", "structure"), vec![ParameterSpec::of::<Option<bool>>("useNewChunksOnly")]].concat();
        let overloads = vec![Overload { parameters: structure }, Overload { parameters: named("biome", "biome") }];
        CommandSpec { name: "locate".into(), aliases: Vec::new(), description: "Displays the coordinates for the closest structure or biome of a given type".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let biome = parse_choice(args, &[("structure", false), ("biome", true)])?;
        let mark = args.mark();
        let name = String::parse(args)?;
        let locate = if biome {
            if !name.starts_with("minecraft:") || sapling_data::biome_id(&name).is_none() {
                return Err(args.error_at(mark));
            }
            Locate::Biome(name)
        } else {
            Option::<bool>::parse(args)?;
            Locate::Structure(namespaced(&name))
        };
        args.finish()?;
        Ok(locate)
    }
}

impl Command for Locate {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let from = ctx.origin(sender).map(|axis| axis.floor() as i32);
        match self {
            Locate::Structure(name) => match ctx.locate_structure(&name, from) {
                Some([x, z]) => out.translate("commands.locate.structure.success", vec![name, x.to_string(), z.to_string(), distance(from, [x, z])], true),
                None => out.translate("commands.locate.structure.fail.nostructurefound", Vec::new(), false),
            },
            Locate::Biome(name) => match ctx.locate_biome(&name, from) {
                Some([x, y, z]) => out.translate("commands.locate.biome.success", vec![name, x.to_string(), y.to_string(), z.to_string(), distance(from, [x, z])], true),
                None => out.translate("commands.locate.biome.fail", vec![name], false),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum What {
    Feature(String),
    FeatureRule(String),
    Structure(String),
    Jigsaw { pool: String, target: String, max_depth: u32 },
}

pub(crate) struct Place {
    what: What,
    at: Option<BlockCoords>,
    options: PlaceOptions,
}

/// `liquidSettings`' words: BDS took both and refused `banana` (bds-t1dplace.txt).
const LIQUID_SETTINGS: [&str; 2] = ["apply_waterlogging", "ignore_waterlogging"];

/// How deep `/place jigsaw` may go: BDS took 3 and refused 0 and 30.
const MOST_DEPTH: u32 = 20;

impl CommandSyntax for Place {
    fn spec() -> CommandSpec {
        let name = |name: &str| ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false };
        let at = || ParameterSpec::of::<Option<BlockCoords>>("position");
        let liquids = ParameterSpec { name: "liquidSettings".into(), kind: ParamKind::Enum { name: "LiquidSettings".into(), values: LIQUID_SETTINGS.map(str::to_owned).to_vec() }, optional: true };
        let flags = || [["ignoreStartHeight", "keepJigsaws", "includeEntities"].map(ParameterSpec::of::<Option<bool>>).to_vec(), vec![liquids.clone()]].concat();
        let overload = |verb: &str, parameters: Vec<ParameterSpec>| Overload { parameters: [vec![ParameterSpec::literal("place", verb)], parameters].concat() };
        let overloads = vec![
            overload("feature", vec![name("feature"), at()]),
            overload("featurerule", vec![name("featureRule"), at()]),
            overload("structure", [vec![name("structure"), at()], flags()].concat()),
            overload("jigsaw", [vec![name("pool"), name("target"), ParameterSpec::of::<i32>("maxDepth"), at()], flags()[1..].to_vec()].concat()),
        ];
        CommandSpec { name: "place".into(), aliases: Vec::new(), description: "Places a jigsaw structure, feature, or feature rule in the world".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let verb = parse_choice(args, &[("feature", 0), ("featurerule", 1), ("structure", 2), ("jigsaw", 3)])?;
        let mark = args.mark();
        let first = String::parse(args)?;
        // BDS takes only the feature and rule names it lists; any structure name (a soft enum).
        let known = match verb {
            0 => sapling_world::is_feature(&first),
            1 => sapling_data::command_takes_feature_rule(&first),
            _ => true,
        };
        // A pool without its namespace parses, and fails to place (bds-k1jig.txt).
        if (verb != 3 && !first.starts_with("minecraft:")) || !known {
            return Err(args.error_at(mark));
        }
        let what = match verb {
            0 => What::Feature(first),
            1 => What::FeatureRule(first),
            2 => What::Structure(first),
            _ => {
                let target = String::parse(args)?;
                let mark = args.mark();
                let max_depth = u32::try_from(i32::parse(args)?).map_err(|_| args.error_at(mark))?;
                What::Jigsaw { pool: first, target, max_depth }
            }
        };
        let at = Option::<BlockCoords>::parse(args)?;
        let mut options = PlaceOptions::default();
        if matches!(what, What::Structure(_) | What::Jigsaw { .. }) {
            // The structure's flags are ignoreStartHeight, keepJigsaws, includeEntities; a jigsaw's the last two.
            let names = if matches!(what, What::Structure(_)) { 3 } else { 2 };
            let mut flags = Vec::new();
            while flags.len() < names {
                let Some(flag) = Option::<bool>::parse(args)? else { break };
                flags.push(flag);
            }
            let flag = |index: usize, default: bool| flags.get(index).copied().unwrap_or(default);
            options = PlaceOptions::with_entities(flag(names - 1, true)).keeping_jigsaws(flag(names - 2, false)).ignoring_start_height(names == 3 && flag(0, false));
            let mark = args.mark();
            if !args.is_empty() {
                let word = args.word()?.to_owned();
                let Some(liquids) = LIQUID_SETTINGS.iter().position(|setting| *setting == word) else {
                    return Err(args.error_at(mark));
                };
                options = options.ignoring_waterlogging(liquids == 1);
            }
        }
        args.finish()?;
        Ok(Self { what, at, options })
    }
}

impl Command for Place {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let at = self.at.map_or_else(|| ctx.origin(sender).map(|axis| axis.floor() as i32), |at| at.resolve(ctx, sender));
        let [lowest, end] = heights(ctx, sender);
        if !(lowest..end).contains(&at[1]) {
            return out.translate("commands.place.invalidposition.error", Vec::new(), false);
        }
        let (placed, unknown) = match &self.what {
            What::Feature(name) => (ctx.place_feature(name, at), "commands.place.invalidfeature.error"),
            What::FeatureRule(_) => (Err(PlaceError::Unknown), "commands.place.invalidposition.error"),
            What::Structure(name) => (ctx.place_structure(name, at, self.options), "commands.place.invalidstructure.error"),
            What::Jigsaw { max_depth, .. } if !(1..=MOST_DEPTH).contains(max_depth) => return out.translate("commands.place.maxdepth.error", Vec::new(), false),
            // BDS answered an unknown pool so, not as an unknown structure, and a pool or target without its namespace (bds-k1jig.txt).
            What::Jigsaw { pool, target, .. } if !pool.contains(':') || !target.contains(':') => (Err(PlaceError::Unknown), "commands.place.generation.error"),
            What::Jigsaw { pool, target, max_depth } => (ctx.place_jigsaw(pool, target, *max_depth, at, self.options), "commands.place.generation.error"),
        };
        match placed {
            Ok(()) => out.translate("commands.place.success", Vec::new(), true),
            Err(PlaceError::Unknown) => out.translate(unknown, Vec::new(), false),
            Err(PlaceError::Unloaded) => out.translate("commands.place.chunknotloaded.error", Vec::new(), false),
            Err(PlaceError::Failed | PlaceError::NoGenerator) => out.translate("commands.place.generation.error", Vec::new(), false),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_distance_counts_from_the_sender_s_block() {
        // bds-bb1biome.txt from feet (0.5, 0.5) and (-1000.5, 700.5).
        assert_eq!([super::distance([0, 200, 0], [-928, -928]), super::distance([-1001, 200, 700], [-1033, 668])], ["1312", "45"]);
    }
}
