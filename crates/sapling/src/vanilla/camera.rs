//! `/camera` and `/camerashake`, answered and sent as BDS 1.26.52 does
//! (research/captures/bds-screen.txt); the other forms are forms.rs.

mod forms;

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selected, Selector};
use sapling_api::{Camera as Set, Command, CommandEnum, Output, Permission, Player, ScreenAction, Sender, ShardCtx};

use super::{no_target, players};

/// CameraPresets.bin's order.
const PRESETS: [&str; 6] = ["minecraft:first_person", "minecraft:fixed_boom", "minecraft:follow_orbit", "minecraft:free", "minecraft:third_person", "minecraft:third_person_front"];
/// acacia-proto's EaseType order.
const EASES: [&str; 32] = [
    "linear", "spring", "in_quad", "out_quad", "in_out_quad", "in_cubic", "out_cubic", "in_out_cubic", "in_quart", "out_quart", "in_out_quart", "in_quint", "out_quint", "in_out_quint", "in_sine", "out_sine",
    "in_out_sine", "in_expo", "out_expo", "in_out_expo", "in_circ", "out_circ", "in_out_circ", "in_bounce", "out_bounce", "in_out_bounce", "in_back", "out_back", "in_out_back", "in_elastic", "out_elastic",
    "in_out_elastic",
];
const MOST_FADE: f32 = 10.0;
const PLAYER_HEIGHT: f32 = 1.8;

enum Facing {
    At(Coords),
    Entity(Selector),
}

struct Setting {
    preset: String,
    ease: Option<(u8, f32)>,
    position: Option<Coords>,
    rotation: Option<[f32; 2]>,
    facing: Option<Facing>,
    default: bool,
    view_offset: Option<[f32; 2]>,
    entity_offset: Option<[f32; 3]>,
}

enum Action {
    Set(Setting),
    Clear,
    Fade { times: Option<[f32; 3]>, colour: Option<[f32; 3]> },
    FovSet { degrees: f32, ease: Option<(u8, f32)> },
    FovClear,
    Other(forms::Form),
}

pub(crate) struct Camera {
    players: Selector,
    action: Action,
}

fn ease(args: &mut Args<'_>) -> Result<(u8, f32), ParseError> {
    let seconds = f32::parse(args)?;
    let mark = args.mark();
    let name = args.word()?;
    let kind = EASES.iter().position(|known| known.eq_ignore_ascii_case(name)).ok_or_else(|| args.error_at(mark))?;
    Ok((kind as u8, seconds))
}

fn set(args: &mut Args<'_>) -> Result<Action, ParseError> {
    let mut set = Setting { preset: args.word()?.to_owned(), ease: None, position: None, rotation: None, facing: None, default: false, view_offset: None, entity_offset: None };
    while let Some(word) = args.peek().map(str::to_ascii_lowercase) {
        let mark = args.mark();
        args.word()?;
        match word.as_str() {
            "ease" => set.ease = Some(ease(args)?),
            "pos" => set.position = Some(Coords::parse(args)?),
            "rot" => set.rotation = Some([f32::parse(args)?, f32::parse(args)?]),
            "facing" if args.peek().is_some_and(|next| next.starts_with('@') || !next.starts_with(['~', '^', '-', '.']) && next.parse::<f32>().is_err()) => set.facing = Some(Facing::Entity(Selector::parse(args)?)),
            "facing" => set.facing = Some(Facing::At(Coords::parse(args)?)),
            "default" => set.default = true,
            "view_offset" => set.view_offset = Some([f32::parse(args)?, f32::parse(args)?]),
            "entity_offset" => set.entity_offset = Some([f32::parse(args)?, f32::parse(args)?, f32::parse(args)?]),
            _ => return Err(args.error_at(mark)),
        }
    }
    Ok(Action::Set(set))
}

fn fade(args: &mut Args<'_>) -> Result<Action, ParseError> {
    let (mut times, mut colour) = (None, None);
    let channel = |args: &mut Args<'_>| Ok::<_, ParseError>(i32::parse(args)? as f32 / 255.0);
    while let Some(word) = args.peek().map(str::to_ascii_lowercase) {
        let mark = args.mark();
        args.word()?;
        match word.as_str() {
            "time" => times = Some([f32::parse(args)?, f32::parse(args)?, f32::parse(args)?]),
            "color" => colour = Some([channel(args)?, channel(args)?, channel(args)?]),
            _ => return Err(args.error_at(mark)),
        }
    }
    Ok(Action::Fade { times, colour })
}

impl CommandSyntax for Camera {
    fn spec() -> CommandSpec {
        let lit = |word: &str| ParameterSpec::literal("camera", word);
        let target = || ParameterSpec::of::<Selector>("players");
        let preset = ParameterSpec { name: "preset".into(), kind: ParamKind::String, optional: false };
        let overloads = [vec![target(), lit("clear")], vec![target(), lit("fade")], vec![target(), lit("set"), preset]];
        CommandSpec { name: "camera".into(), aliases: Vec::new(), description: "Issues a camera instruction".into(), permission: Permission::Operator, overloads: overloads.map(|parameters| Overload { parameters }).to_vec() }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let players = Selector::parse(args)?;
        let mark = args.mark();
        let word = args.word()?.to_ascii_lowercase();
        let action = match word.as_str() {
            "set" => set(args)?,
            "clear" => Action::Clear,
            "fade" => fade(args)?,
            "fov_set" => {
                let degrees = f32::parse(args)?;
                let ease = if args.is_empty() { None } else { Some(ease(args)?) };
                Action::FovSet { degrees, ease }
            }
            "fov_clear" => Action::FovClear,
            other => Action::Other(forms::parse(other, args)?.ok_or_else(|| args.error_at(mark))?),
        };
        args.finish()?;
        Ok(Self { players, action })
    }
}

/// Where `facing <entity>` turns the camera: the middle of the entity's box (a player's 0.9 up).
fn middle(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: &Selector) -> Option<[f32; 3]> {
    let height = |kind: &str| sapling_data::mob_size(kind).map_or(0.0, |[_, height]| height);
    let ([x, y, z], height) = match ctx.select(sender, selector).into_iter().next()? {
        Selected::Player(player) => (ctx.position(player.id)?.feet, PLAYER_HEIGHT),
        Selected::Entity(entity) => (entity.feet, height(&entity.kind)),
    };
    Some([x, y + height / 2.0, z])
}

impl Command for Camera {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, Some(&self.players));
        if found.is_empty() {
            return no_target(out);
        }
        let action = match self.action {
            Action::Set(set) => {
                let Some(preset) = PRESETS.iter().position(|known| *known == set.preset).map(|preset| preset as u32) else { return out.translate("commands.camera.unknown.preset", Vec::new(), false) };
                let eased = set.ease.is_some_and(|(_, seconds)| seconds > 0.0);
                if let Some(refused) = forms::refused_set(preset, eased, None, None) {
                    return out.translate(refused, Vec::new(), false);
                }
                if set.rotation.is_some_and(|[pitch, _]| !(-90.0..=90.0).contains(&pitch)) {
                    return out.translate("commands.camera.invalid.pitch", Vec::new(), false);
                }
                if let Some(refused) = forms::refused_set(preset, false, set.view_offset, set.entity_offset) {
                    return out.translate(refused, Vec::new(), false);
                }
                let facing = match set.facing {
                    Some(Facing::At(coords)) => Some(coords.resolve(ctx, sender)),
                    Some(Facing::Entity(selector)) => middle(ctx, sender, &selector),
                    None => None,
                };
                let position = set.position.map(|coords| coords.resolve(ctx, sender));
                forms::remember(ctx, &found, Some(preset));
                Some(ScreenAction::Camera(Set { preset, ease: set.ease, position, rotation: set.rotation, facing, default: set.default, view_offset: set.view_offset, entity_offset: set.entity_offset }))
            }
            Action::Clear => {
                forms::remember(ctx, &found, None);
                Some(ScreenAction::ClearCamera)
            }
            Action::Other(form) => match forms::action(ctx, sender, &found, form) {
                Ok(action) => action,
                Err(refused) => return out.translate(refused, Vec::new(), false),
            },
            Action::Fade { times, colour } => {
                if times.is_some_and(|times| times.iter().any(|seconds| !(0.0..=MOST_FADE).contains(seconds))) {
                    return out.translate("commands.camera.invalid.time", Vec::new(), false);
                }
                Some(ScreenAction::CameraFade { seconds: times, colour })
            }
            Action::FovSet { degrees, ease } => {
                let (kind, ease_seconds) = ease.unwrap_or((0, 0.0));
                Some(ScreenAction::CameraFov { degrees, ease_seconds, ease: EASES[usize::from(kind)].to_owned() })
            }
            Action::FovClear => Some(ScreenAction::ClearCameraFov),
        };
        if let Some(action) = action {
            found.iter().for_each(|player| ctx.screen_action(player.id, &action));
        }
        answer(out, "commands.camera.success", &found);
    }
}

/// `key` naming every player, counted once each.
fn answer(out: &mut Output, key: &str, found: &[Player]) {
    let names = found.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ");
    out.translate(key, vec![names], true);
    out.done_to(found.len() as u32);
}

#[derive(CommandEnum, Clone, Copy)]
#[command(crate = "crate", name = "CameraShakeType")]
pub(crate) enum ShakeType {
    Positional,
    Rotational,
}

/// `/camerashake add <player> [intensity] [seconds] [type]` and `/camerashake stop [player]`, the
/// latter for every player without one.
#[derive(Command)]
#[command(crate = "crate", name = "camerashake", description = "Applies shaking to the players' camera with a specified intensity and duration.", permission = "operator")]
pub(crate) enum CameraShake {
    Add { player: Selector, intensity: Option<f32>, seconds: Option<f32>, shake_type: Option<ShakeType> },
    Stop { player: Option<Selector> },
}

const MOST_SHAKE: f32 = 4.0;

impl Command for CameraShake {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (found, action) = match self {
            Self::Add { player, intensity, seconds, shake_type } => {
                let intensity = intensity.unwrap_or(0.5);
                if intensity > MOST_SHAKE {
                    return out.translate("commands.generic.num.tooBig", vec![format!("{intensity:.2}"), format!("{MOST_SHAKE:.2}")], false);
                }
                let rotational = matches!(shake_type, Some(ShakeType::Rotational));
                (players(ctx, sender, Some(&player)), ScreenAction::Shake { intensity, seconds: seconds.unwrap_or(1.0), rotational })
            }
            Self::Stop { player: Some(player) } => (players(ctx, sender, Some(&player)), ScreenAction::StopShake),
            Self::Stop { player: None } => (ctx.players().collect(), ScreenAction::StopShake),
        };
        if found.is_empty() {
            return no_target(out);
        }
        found.iter().for_each(|player| ctx.screen_action(player.id, &action));
        let key = if action == ScreenAction::StopShake { "commands.screenshake.successStop" } else { "commands.screenshake.success" };
        answer(out, key, &found);
    }
}
