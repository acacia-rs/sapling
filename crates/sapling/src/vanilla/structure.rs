//! `/structure save|load|delete`, answered as BDS answered (research/captures/bds-structures.txt):
//! a name without a namespace is `mystructure:`'s, the box is at most 64x384x64, and a load stays
//! within the world's height. Animated loads place at once.
// TODO: heights are the overworld's in every dimension; BDS's answer for an unloaded box is not known.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, parse_choice};
use sapling_api::{Command, CommandEnum, Mirror, Output, Permission, Rotation, Sender, ShardCtx, StructureError, StructureLoad};

/// The largest box a structure holds.
const MOST: [i32; 3] = [64, 384, 64];
/// The overworld's heights, which a structure or a placed feature must be within.
pub(super) const LOWEST_Y: i32 = -64;
pub(super) const HEIGHT_END: i32 = 320;

#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum SaveMode {
    Memory,
    Disk,
}

#[derive(Debug, PartialEq)]
enum Action {
    Save { from: BlockCoords, to: BlockCoords, include_entities: bool, include_blocks: bool, to_disk: bool },
    Load { to: BlockCoords, how: StructureLoad, integrity: f32 },
    Delete,
}

pub(crate) struct StructureCommand {
    name: String,
    action: Action,
}

fn choice(name: &str, values: &[&str]) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind: ParamKind::Enum { name: name.into(), values: values.iter().map(|v| (*v).to_owned()).collect() }, optional: true }
}

const ROTATIONS: [(&str, Rotation); 4] = [("0_degrees", Rotation::None), ("90_degrees", Rotation::Clockwise90), ("180_degrees", Rotation::Clockwise180), ("270_degrees", Rotation::Clockwise270)];
const MIRRORS: [(&str, Mirror); 4] = [("none", Mirror::None), ("x", Mirror::X), ("z", Mirror::Z), ("xz", Mirror::XZ)];
const ANIMATIONS: [&str; 2] = ["layer_by_layer", "block_by_block"];

impl CommandSyntax for StructureCommand {
    fn spec() -> CommandSpec {
        let name = || ParameterSpec::of::<String>("name");
        let corners = || vec![ParameterSpec::of::<BlockCoords>("from"), ParameterSpec::of::<BlockCoords>("to")];
        let flag = |name: &str| ParameterSpec::of::<Option<bool>>(name);
        let save = |rest: Vec<ParameterSpec>| Overload { parameters: [vec![ParameterSpec::literal("structure", "save"), name()], corners(), rest].concat() };
        let placed = || vec![flag("includeEntities"), flag("includeBlocks"), flag("waterlogged"), ParameterSpec::of::<Option<f32>>("integrity"), ParameterSpec::of::<Option<String>>("seed")];
        let load = |rest: Vec<ParameterSpec>| Overload { parameters: [vec![ParameterSpec::literal("structure", "load"), name(), ParameterSpec::of::<BlockCoords>("to")], rest, placed()].concat() };
        let turns = || vec![choice("Rotation", &ROTATIONS.map(|(word, _)| word)), choice("Mirror", &MIRRORS.map(|(word, _)| word))];
        let animated = [turns(), vec![choice("StructureAnimationMode", &ANIMATIONS), ParameterSpec::of::<Option<f32>>("animationSeconds")]].concat();
        CommandSpec {
            name: "structure".into(),
            aliases: Vec::new(),
            description: "Saves or loads a structure in the world".into(),
            permission: Permission::Operator,
            overloads: vec![
                save(vec![flag("includeEntities"), ParameterSpec::of::<Option<SaveMode>>("saveMode"), flag("includeBlocks")]),
                save(vec![ParameterSpec::of::<Option<SaveMode>>("saveMode")]),
                load(turns()),
                load(animated),
                Overload { parameters: vec![ParameterSpec::literal("structure", "delete"), name()] },
            ],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let verb = parse_choice(args, &[("save", 0), ("load", 1), ("delete", 2)])?;
        let name = String::parse(args)?;
        let action = match verb {
            0 => parse_save(args)?,
            1 => parse_load(args)?,
            _ => Action::Delete,
        };
        args.finish()?;
        Ok(Self { name, action })
    }
}

fn parse_save(args: &mut Args<'_>) -> Result<Action, ParseError> {
    let (from, to) = (BlockCoords::parse(args)?, BlockCoords::parse(args)?);
    let is_mode = |args: &Args<'_>| args.peek().is_some_and(|word| ["memory", "disk"].iter().any(|mode| mode.eq_ignore_ascii_case(word)));
    let include_entities = if is_mode(args) { None } else { Option::<bool>::parse(args)? };
    let mode = Option::<SaveMode>::parse(args)?;
    let include_blocks = Option::<bool>::parse(args)?;
    Ok(Action::Save { from, to, include_entities: include_entities.unwrap_or(true), include_blocks: include_blocks.unwrap_or(true), to_disk: mode == Some(SaveMode::Disk) })
}

fn parse_load(args: &mut Args<'_>) -> Result<Action, ParseError> {
    let to = BlockCoords::parse(args)?;
    let mut how = StructureLoad::default();
    if args.peek().is_some() && !is_bool(args.peek()) {
        how.rotation = parse_choice(args, &ROTATIONS)?;
    }
    if args.peek().is_some() && !is_bool(args.peek()) {
        how.mirror = parse_choice(args, &MIRRORS)?;
    }
    if args.peek().is_some_and(|word| ANIMATIONS.iter().any(|mode| mode.eq_ignore_ascii_case(word))) {
        args.word()?;
        Option::<f32>::parse(args)?;
    }
    how.include_entities = Option::<bool>::parse(args)?.unwrap_or(true);
    how.include_blocks = Option::<bool>::parse(args)?.unwrap_or(true);
    how.waterlogged = Option::<bool>::parse(args)?.unwrap_or(false);
    let integrity = Option::<f32>::parse(args)?.unwrap_or(100.0);
    how.seed = Option::<String>::parse(args)?.unwrap_or_default();
    Ok(Action::Load { to, how, integrity })
}

fn is_bool(word: Option<&str>) -> bool {
    word.is_some_and(|word| word.eq_ignore_ascii_case("true") || word.eq_ignore_ascii_case("false"))
}

impl Command for StructureCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let full = sapling_world::structure_name(&self.name);
        match self.action {
            Action::Save { from, to, include_entities, include_blocks, to_disk } => {
                let (from, to) = (from.resolve(ctx, sender), to.resolve(ctx, sender));
                let size: [i32; 3] = std::array::from_fn(|axis| (from[axis] - to[axis]).abs() + 1);
                if (0..3).any(|axis| size[axis] > MOST[axis]) {
                    let limits = MOST.iter().chain(&size).map(i32::to_string).collect();
                    return out.translate("commands.structure.size.tooBig", limits, false);
                }
                if !include_entities && !include_blocks {
                    return out.translate("commands.structure.empty.save", Vec::new(), false);
                }
                match ctx.save_structure(&self.name, from, to, include_entities, include_blocks, to_disk) {
                    Ok(()) => out.translate("commands.structure.save.success", vec![full], true),
                    Err(_) => out.translate("commands.structure.error", Vec::new(), false),
                }
            }
            Action::Load { to, mut how, integrity } => {
                if !(0.0..=100.0).contains(&integrity) {
                    return out.translate("commands.structure.invalid.integrity", Vec::new(), false);
                }
                how.integrity = integrity / 100.0;
                let Some(size) = ctx.structure_size(&self.name) else { return out.translate("commands.structure.notFound", vec![full], false) };
                let at = to.resolve(ctx, sender);
                if at[1] > HEIGHT_END - size[1] {
                    return out.translate("commands.structure.y.tooHigh", vec![(HEIGHT_END - size[1]).to_string()], false);
                }
                if at[1] < LOWEST_Y {
                    return out.translate("commands.structure.y.tooLow", vec![LOWEST_Y.to_string()], false);
                }
                if !how.include_entities && !how.include_blocks {
                    return out.translate("commands.structure.empty.load", Vec::new(), false);
                }
                match ctx.load_structure(&self.name, at, &how) {
                    Ok(()) => out.translate("commands.structure.load.success", vec![self.name], true),
                    Err(StructureError::NotFound) => out.translate("commands.structure.notFound", vec![full], false),
                    Err(_) => out.translate("commands.structure.error", Vec::new(), false),
                }
            }
            Action::Delete => match ctx.delete_structure(&self.name) {
                true => out.translate("commands.structure.delete.success", vec![full], true),
                false => out.translate("commands.structure.notFound", vec![full], false),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> Result<(String, Action), ParseError> {
        let sender = Sender::Console;
        StructureCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|command| (command.name, command.action))
    }

    #[test]
    fn save_takes_a_mode_with_or_without_flags() {
        let Ok((name, Action::Save { include_entities, include_blocks, to_disk, .. })) = parsed("save \"bad name\" 4 99 4 6 101 6 false disk true") else { panic!() };
        assert_eq!((name.as_str(), include_entities, include_blocks, to_disk), ("bad name", false, true, true));
        let Ok((_, Action::Save { include_entities, to_disk, .. })) = parsed("save s3 4 99 4 6 101 6 disk") else { panic!() };
        assert!(include_entities && to_disk);
    }

    #[test]
    fn load_takes_turns_an_animation_and_flags() {
        let Ok((_, Action::Load { how, integrity, .. })) = parsed("load s1 58 99 4 0_degrees none false true false 50 abc") else { panic!() };
        assert_eq!((how.include_entities, how.include_blocks, integrity, how.seed.as_str()), (false, true, 50.0, "abc"));
        let Ok((_, Action::Load { how, .. })) = parsed("load s1 16 99 12 90_degrees xz block_by_block 0 true false") else { panic!() };
        assert_eq!((how.rotation, how.mirror, how.include_entities, how.include_blocks), (Rotation::Clockwise90, Mirror::XZ, true, false));
        assert!(parsed("load s1 1 2 3 45_degrees").is_err() && parsed("move s1").is_err());
    }
}
