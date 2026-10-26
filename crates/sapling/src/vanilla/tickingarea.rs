//! `/tickingarea add|remove|remove_all|list|preload`, answered as BDS answered
//! (research/captures/bds-ticking.txt): a box of at most 100 chunks or a circle of radius at most
//! 4, ten areas a dimension, each area listed on a line of its own with its bounds widened to whole
//! chunks; what a change touched is listed after it, each line ended.
// TODO: `list all-dimensions` lists the dimension in scope only, under the overworld's name.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, parse_choice};
use sapling_api::{Command, MOST_TICKING_AREAS, Output, Permission, Sender, ShardCtx, TickingArea};

const MOST_CHUNKS: usize = 100;
const MOST_RADIUS: i32 = 4;

/// An area named by its name, or by a place inside it.
#[derive(Debug, PartialEq)]
enum Pick {
    Name(String),
    At(BlockCoords),
}

#[derive(Debug, PartialEq)]
enum Action {
    Add { from: BlockCoords, to: BlockCoords, name: Option<String>, preload: bool },
    AddCircle { centre: BlockCoords, radius: i32, name: Option<String>, preload: bool },
    Remove(Pick),
    RemoveAll,
    List { all_dimensions: bool },
    Preload { pick: Pick, preload: Option<bool> },
}

pub(crate) struct TickingAreaCommand(Action);

impl CommandSyntax for TickingAreaCommand {
    fn spec() -> CommandSpec {
        let verb = |word: &str| ParameterSpec::literal("tickingarea", word);
        let name = |optional: bool| ParameterSpec { name: "name".into(), kind: ParamKind::String, optional };
        let at = |name: &str| ParameterSpec::of::<BlockCoords>(name);
        let preload = || ParameterSpec::of::<Option<bool>>("preload");
        let circle = ParameterSpec::literal("tickingarea", "circle");
        let all = ParameterSpec { name: "all-dimensions".into(), kind: ParamKind::Enum { name: "TickingAreaAllDimensions".into(), values: vec!["all-dimensions".into()] }, optional: true };
        let overloads = [
            vec![verb("add"), at("from"), at("to"), name(true), preload()],
            vec![verb("add"), circle, at("center"), ParameterSpec::of::<i32>("radius"), name(true), preload()],
            vec![verb("remove"), at("position")],
            vec![verb("remove"), name(false)],
            vec![verb("remove_all")],
            vec![verb("list"), all],
            vec![verb("preload"), at("position"), preload()],
            vec![verb("preload"), name(false), preload()],
        ];
        let overloads = overloads.into_iter().map(|parameters| Overload { parameters }).collect();
        CommandSpec { name: "tickingarea".into(), aliases: Vec::new(), description: "Add, remove, or list ticking areas".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let verb = parse_choice(args, &[("add", 0), ("remove", 1), ("remove_all", 2), ("list", 3), ("preload", 4)])?;
        let action = match verb {
            0 if args.peek().is_some_and(|word| word.eq_ignore_ascii_case("circle")) => {
                args.word()?;
                let (centre, radius) = (BlockCoords::parse(args)?, i32::parse(args)?);
                Action::AddCircle { centre, radius, name: Option::<String>::parse(args)?, preload: Option::<bool>::parse(args)?.unwrap_or(false) }
            }
            0 => {
                let (from, to) = (BlockCoords::parse(args)?, BlockCoords::parse(args)?);
                Action::Add { from, to, name: Option::<String>::parse(args)?, preload: Option::<bool>::parse(args)?.unwrap_or(false) }
            }
            1 => Action::Remove(pick(args)?),
            2 => Action::RemoveAll,
            3 => Action::List { all_dimensions: Option::<String>::parse(args)?.is_some_and(|word| word.eq_ignore_ascii_case("all-dimensions")) },
            _ => Action::Preload { pick: pick(args)?, preload: Option::<bool>::parse(args)? },
        };
        args.finish()?;
        Ok(Self(action))
    }
}

/// A name, or three coordinates where the next word starts like one.
fn pick(args: &mut Args<'_>) -> Result<Pick, ParseError> {
    let is_place = args.peek().is_some_and(|word| word.starts_with(['~', '^', '-']) || word.parse::<f64>().is_ok());
    Ok(if is_place { Pick::At(BlockCoords::parse(args)?) } else { Pick::Name(String::parse(args)?) })
}

/// An area's line in BDS's lists: its bounds in blocks, or a circle's centre and radius.
fn line(area: &TickingArea) -> String {
    let preload = if area.preload { " %commands.tickingarea-list.preload" } else { "" };
    if area.circle {
        let radius = (area.max[0] - area.min[0]) / 2;
        let [x, z] = [area.min[0] + radius, area.min[1] + radius].map(|chunk| chunk * 16 + 8);
        format!("- {} (%commands.tickingarea-list.type.circle): {x} 0 {z} %commands.tickingarea-list.circle.radius: {radius} %commands.tickingarea-list.chunks{preload}", area.name)
    } else {
        let ([x0, z0], [x1, z1]) = (area.min.map(|chunk| chunk * 16), area.max.map(|chunk| chunk * 16 + 15));
        format!("- {}: {x0} 0 {z0} %commands.tickingarea-list.to {x1} 0 {z1}{preload}", area.name)
    }
}

/// The areas touched by a change, each line ended, as BDS lists them after it.
fn touched(areas: &[TickingArea]) -> String {
    areas.iter().map(|area| line(area) + "\n").collect()
}

fn chunk([x, _, z]: [i32; 3]) -> [i32; 2] {
    [x >> 4, z >> 4]
}

fn picked(pick: &Pick, here: [f32; 3]) -> impl Fn(&TickingArea) -> bool + '_ {
    let at = match pick {
        Pick::At(at) => Some(chunk(at.from(here))),
        Pick::Name(_) => None,
    };
    move |area| match pick {
        Pick::Name(name) => area.name == *name,
        Pick::At(_) => at.is_some_and(|at| area.holds(at)),
    }
}

/// The failure of a pick that found nothing.
fn missed(out: &mut Output, verb: &str, pick: &Pick, here: [f32; 3]) {
    match pick {
        Pick::Name(name) => out.translate(format!("commands.tickingarea-{verb}.byname.failure"), vec![name.clone()], false),
        Pick::At(at) => {
            let [x, y, z] = at.from(here);
            let key = if verb == "remove" { "commands.tickingarea-remove.failure".to_owned() } else { format!("commands.tickingarea-{verb}.byposition.failure") };
            out.translate(key, vec![format!("{x}, {y}, {z}")], false);
        }
    }
}

fn in_use(ctx: &ShardCtx<'_>, out: &mut Output) {
    out.translate("commands.tickingarea.inuse", vec![ctx.ticking_areas().len().to_string(), MOST_TICKING_AREAS.to_string()], true);
}

/// The first `AreaN` no area is named.
fn default_name(ctx: &ShardCtx<'_>) -> String {
    let taken = ctx.ticking_areas();
    (0..).map(|n| format!("Area{n}")).find(|name| taken.iter().all(|area| area.name != *name)).expect("an unused name")
}

fn add(ctx: &mut ShardCtx<'_>, out: &mut Output, area: TickingArea, shown: [String; 2], key: &str) {
    if ctx.ticking_areas().iter().any(|other| other.name == area.name) {
        return out.translate("commands.tickingarea-add.conflictingname", vec![area.name], false);
    }
    let preload = area.preload;
    if !ctx.add_ticking_area(area) {
        return out.translate("commands.tickingarea-add.failure", vec![MOST_TICKING_AREAS.to_string()], false);
    }
    let key = if preload { format!("{key}.preload.success") } else { format!("{key}.success") };
    out.translate(key, shown.to_vec(), true);
    in_use(ctx, out);
}

impl Command for TickingAreaCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        match self.0 {
            Action::Add { from, to, name, preload } => {
                let (from, to) = (chunk(from.from(here)), chunk(to.from(here)));
                let (min, max) = ([from[0].min(to[0]), from[1].min(to[1])], [from[0].max(to[0]), from[1].max(to[1])]);
                let name = name.unwrap_or_else(|| default_name(ctx));
                let area = TickingArea { name, min, max, circle: false, preload };
                if area.chunk_count() > MOST_CHUNKS {
                    return out.translate("commands.tickingarea-add.chunkfailure", vec![MOST_CHUNKS.to_string()], false);
                }
                let shown = [format!("{}, 0, {}", min[0] * 16, min[1] * 16), format!("{}, 0, {}", max[0] * 16 + 15, max[1] * 16 + 15)];
                add(ctx, out, area, shown, "commands.tickingarea-add-bounds");
            }
            Action::AddCircle { centre, radius, name, preload } => {
                if !(0..=MOST_RADIUS).contains(&radius) {
                    return out.translate("commands.tickingarea-add.radiusfailure", vec![MOST_RADIUS.to_string()], false);
                }
                let [x, z] = chunk(centre.from(here));
                let name = name.unwrap_or_else(|| default_name(ctx));
                let area = TickingArea { name, min: [x - radius, z - radius], max: [x + radius, z + radius], circle: true, preload };
                add(ctx, out, area, [format!("{}, 0, {}", x * 16 + 8, z * 16 + 8), radius.to_string()], "commands.tickingarea-add-circle");
            }
            Action::Remove(pick) => {
                let removed = ctx.remove_ticking_areas(&picked(&pick, here));
                if removed.is_empty() {
                    return missed(out, "remove", &pick, here);
                }
                out.translate("commands.tickingarea-remove.success", Vec::new(), true);
                out.print(touched(&removed));
                in_use(ctx, out);
            }
            Action::RemoveAll => {
                let removed = ctx.remove_ticking_areas(&|_| true);
                if removed.is_empty() {
                    return out.translate("commands.tickingarea.noneExist.currentDimension", Vec::new(), false);
                }
                out.translate("commands.tickingarea-remove_all.success", Vec::new(), true);
                out.print(touched(&removed));
                in_use(ctx, out);
            }
            Action::List { all_dimensions } => {
                let areas = ctx.ticking_areas();
                if areas.is_empty() {
                    let key = if all_dimensions { "commands.tickingarea-list.failure.allDimensions" } else { "commands.tickingarea.noneExist.currentDimension" };
                    return out.translate(key, Vec::new(), false);
                }
                let lines = areas.iter().map(line).collect::<Vec<_>>().join("\n");
                match all_dimensions {
                    true => {
                        out.print("§a%commands.tickingarea-list.success.allDimensions");
                        out.print(format!("%dimension.dimensionName0: \n{lines}"));
                    }
                    false => {
                        out.print("§a%commands.tickingarea-list.success.currentDimension");
                        out.print(lines);
                    }
                }
                in_use(ctx, out);
            }
            Action::Preload { pick, preload } => {
                let is_picked = picked(&pick, here);
                if !ctx.ticking_areas().iter().any(&is_picked) {
                    return missed(out, "preload", &pick, here);
                }
                if let Some(preload) = preload {
                    ctx.set_ticking_preload(&is_picked, preload);
                    out.translate("commands.tickingarea-preload.success", Vec::new(), true);
                }
                let areas: Vec<TickingArea> = ctx.ticking_areas().into_iter().filter(|area| is_picked(area)).collect();
                out.print(touched(&areas));
                let count = areas.iter().filter(|area| area.preload).count();
                out.translate("commands.tickingarea-preload.count", vec![count.to_string()], true);
            }
        }
    }
}
