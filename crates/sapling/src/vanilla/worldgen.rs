//! `/locate` and `/place`, over the world generator (sapling_api::GenerationBackend), answered as
//! BDS answered (research/captures/bds-locate.txt, bds-place.txt): a place is named by its block
//! and its distance, rounded down, from the sender.
// TODO: `/place featurerule` (BDS's feature rules are not ported), BDS's names for `/place structure`, and the structure options
// (`ignoreStartHeight`, `keepJigsaws`, `includeEntities`, `liquidSettings`) are taken and not followed.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, parse_choice};
use sapling_api::{Command, Output, Permission, PlaceError, Sender, ShardCtx};

use super::structure::{HEIGHT_END, LOWEST_Y};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
}

/// The distance BDS shows: across, from the sender's feet, rounded down.
fn distance(here: [f32; 3], [x, z]: [i32; 2]) -> String {
    ((x as f32 - here[0]).hypot(z as f32 - here[2]).floor() as i64).to_string()
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
        let here = ctx.origin(sender);
        let from = here.map(|axis| axis.floor() as i32);
        match self {
            Locate::Structure(name) => match ctx.locate_structure(&name, from) {
                Some([x, z]) => out.translate("commands.locate.structure.success", vec![name, x.to_string(), z.to_string(), distance(here, [x, z])], true),
                None => out.translate("commands.locate.structure.fail.nostructurefound", Vec::new(), false),
            },
            Locate::Biome(name) => match ctx.locate_biome(&name, from) {
                Some([x, y, z]) => out.translate("commands.locate.biome.success", vec![name, x.to_string(), y.to_string(), z.to_string(), distance(here, [x, z])], true),
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
}

/// How deep `/place jigsaw` may go: BDS took 3 and refused 0 and 30.
const MOST_DEPTH: u32 = 20;

impl CommandSyntax for Place {
    fn spec() -> CommandSpec {
        let name = |name: &str| ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false };
        let at = || ParameterSpec::of::<Option<BlockCoords>>("position");
        let flags = || ["ignoreStartHeight", "keepJigsaws", "includeEntities", "liquidSettings"].map(ParameterSpec::of::<Option<bool>>).to_vec();
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
        // BDS takes only the feature names it knows; sapling knows no feature rules.
        let known = match verb {
            0 => sapling_world::is_feature(&first),
            1 => false,
            _ => true,
        };
        if !first.starts_with("minecraft:") || !known {
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
        while Option::<bool>::parse(args)?.is_some() {}
        args.finish()?;
        Ok(Self { what, at })
    }
}

impl Command for Place {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        let at = self.at.map_or_else(|| here.map(|axis| axis.floor() as i32), |at| at.from(here));
        if !(LOWEST_Y..HEIGHT_END).contains(&at[1]) {
            return out.translate("commands.place.invalidposition.error", Vec::new(), false);
        }
        let (placed, unknown) = match &self.what {
            What::Feature(name) => (ctx.place_feature(name, at), "commands.place.invalidfeature.error"),
            What::FeatureRule(_) => (Err(PlaceError::Unknown), "commands.place.invalidfeaturerule.error"),
            What::Structure(name) => (ctx.place_structure(name, at), "commands.place.invalidstructure.error"),
            What::Jigsaw { max_depth, .. } if !(1..=MOST_DEPTH).contains(max_depth) => return out.translate("commands.place.maxdepth.error", Vec::new(), false),
            // BDS answered an unknown pool so, not as an unknown structure.
            What::Jigsaw { pool, target, max_depth } => (ctx.place_jigsaw(pool, target, *max_depth, at), "commands.place.generation.error"),
        };
        match placed {
            Ok(()) => out.translate("commands.place.success", Vec::new(), true),
            Err(PlaceError::Unknown) => out.translate(unknown, Vec::new(), false),
            Err(PlaceError::Unloaded) => out.translate("commands.place.chunknotloaded.error", Vec::new(), false),
            Err(PlaceError::Failed | PlaceError::NoGenerator) => out.translate("commands.place.generation.error", Vec::new(), false),
        }
    }
}
