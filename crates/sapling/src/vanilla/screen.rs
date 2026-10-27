//! `/fog`, `/hud`, `/playanimation` and `/music`, answered and sent as BDS 1.26.52 does
//! (research/captures/bds-screen.txt, bds-music.txt).

use std::collections::HashMap;

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Animation, Command, CommandEnum, Music as MusicAction, Output, Permission, PlayerId, ScreenAction, Sender, ShardCtx};

use super::{no_target, players, split};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FogMode {
    Push,
    Pop,
    Remove,
}

/// Each player's fog stack, bottom first: the fog and the id `/fog` named it by.
#[derive(Default)]
struct Fogs(HashMap<PlayerId, Vec<(String, String)>>);

/// `/fog <victim> push <fog> <id>`, `/fog <victim> pop|remove <id>`: pop takes the topmost entry
/// of the id, remove every one; each change sends the player's whole stack. Any fog id is taken.
pub(crate) struct Fog {
    victim: Selector,
    mode: FogMode,
    fog: String,
    user_id: String,
}

impl CommandSyntax for Fog {
    fn spec() -> CommandSpec {
        let word = |name: &str| ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false };
        let victim = || ParameterSpec::of::<Selector>("victim");
        let take = ParameterSpec { name: "mode".into(), kind: ParamKind::Enum { name: "FogMode".into(), values: vec!["pop".into(), "remove".into()] }, optional: false };
        let overloads = [vec![victim(), ParameterSpec::literal("fog", "push"), word("fogId"), word("userProvidedId")], vec![victim(), take, word("userProvidedId")]];
        CommandSpec { name: "fog".into(), aliases: Vec::new(), description: "Add or remove fog settings file".into(), permission: Permission::Operator, overloads: overloads.map(|parameters| Overload { parameters }).to_vec() }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let victim = Selector::parse(args)?;
        let mark = args.mark();
        let mode = match args.word()?.to_ascii_lowercase().as_str() {
            "push" => FogMode::Push,
            "pop" => FogMode::Pop,
            "remove" => FogMode::Remove,
            _ => return Err(args.error_at(mark)),
        };
        let fog = if mode == FogMode::Push { args.word()?.to_owned() } else { String::new() };
        let user_id = args.word()?.to_owned();
        args.finish()?;
        Ok(Self { victim, mode, fog, user_id })
    }
}

impl Command for Fog {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, Some(&self.victim));
        if found.is_empty() {
            return no_target(out);
        }
        let user_id = &self.user_id;
        let mut done = 0;
        for player in found {
            let stack = ctx.state::<Fogs>().0.entry(player.id).or_default();
            let before = stack.len();
            let key = match self.mode {
                FogMode::Push => {
                    stack.push((self.fog.clone(), user_id.clone()));
                    "commands.fog.success.push"
                }
                FogMode::Pop => {
                    if let Some(at) = stack.iter().rposition(|(_, id)| id == user_id) {
                        stack.remove(at);
                    }
                    "commands.fog.success.pop"
                }
                FogMode::Remove => {
                    stack.retain(|(_, id)| id != user_id);
                    "commands.fog.success.remove"
                }
            };
            if stack.len() == before {
                out.translate("commands.fog.invalidUserId", vec![player.name, user_id.clone()], false);
                continue;
            }
            let fogs = stack.iter().map(|(fog, _)| fog.clone()).collect();
            ctx.screen_action(player.id, &ScreenAction::Fog(fogs));
            out.translate(key, vec![player.name, user_id.clone()], true);
            done += 1;
        }
        out.done_to(done);
    }
}

#[derive(CommandEnum, Clone, Copy, PartialEq, Eq)]
#[command(crate = "crate", name = "HudVisibility")]
pub(crate) enum HudVisibility {
    Hide,
    Reset,
}

/// The client's HUD elements, by number.
#[derive(CommandEnum, Clone, Copy)]
#[command(crate = "crate", name = "HudElement")]
pub(crate) enum HudElement {
    #[command(name = "paperdoll")]
    PaperDoll,
    Armor,
    #[command(name = "tooltips")]
    ToolTips,
    #[command(name = "touch_controls")]
    TouchControls,
    Crosshair,
    #[command(name = "hotbar")]
    HotBar,
    Health,
    #[command(name = "progress_bar")]
    ProgressBar,
    Hunger,
    #[command(name = "air_bubbles")]
    AirBubbles,
    #[command(name = "horse_health")]
    HorseHealth,
    #[command(name = "status_effects")]
    StatusEffects,
    #[command(name = "item_text")]
    ItemText,
    All,
}

/// Every element, in the order BDS listed them for `/hud @s hide`.
const ALL: [i32; 13] = [1, 4, 5, 0, 2, 3, 6, 7, 8, 9, 10, 11, 12];

/// `/hud <target> hide|reset [element]`; no element, or `all`, is every one.
#[derive(Command)]
#[command(crate = "crate", name = "hud", description = "Changes the visibility of hud elements.", permission = "operator")]
pub(crate) struct Hud {
    target: Selector,
    visible: HudVisibility,
    hud_element: Option<HudElement>,
}

impl Command for Hud {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, Some(&self.target));
        if found.is_empty() {
            return no_target(out);
        }
        let elements = match self.hud_element {
            None | Some(HudElement::All) => ALL.to_vec(),
            Some(element) => vec![element as i32],
        };
        let action = ScreenAction::Hud { elements, hide: self.visible == HudVisibility::Hide };
        found.iter().for_each(|player| ctx.screen_action(player.id, &action));
        out.translate("commands.hud.success", Vec::new(), true);
        out.done_to(found.len() as u32);
    }
}

/// `/playanimation <entity> <animation> [next_state] [blend_out_time] [stop_expression] [controller]`:
/// one AnimateEntity naming every target.
#[derive(Command)]
#[command(crate = "crate", name = "playanimation", description = "Makes one or more entities play a one-off animation. Assumes all variables are setup correctly.", permission = "operator")]
pub(crate) struct PlayAnimation {
    entity: Selector,
    animation: String,
    next_state: Option<String>,
    blend_out_time: Option<f32>,
    stop_expression: Option<String>,
    controller: Option<String>,
}

impl Command for PlayAnimation {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (found_players, found_entities) = split(ctx.select(sender, &self.entity));
        if found_players.is_empty() && found_entities.is_empty() {
            return no_target(out);
        }
        let mut animation = Animation::new(self.animation);
        animation.next_state = self.next_state.unwrap_or(animation.next_state);
        animation.blend_out_seconds = self.blend_out_time.unwrap_or(0.0);
        animation.stop_expression = self.stop_expression.unwrap_or(animation.stop_expression);
        animation.controller = self.controller.unwrap_or(animation.controller);
        let ids: Vec<_> = found_players.iter().map(|player| player.id).collect();
        let entities: Vec<_> = found_entities.iter().map(|entity| entity.id).collect();
        ctx.animate(&ids, &entities, &animation);
        out.translate("commands.playanimation.success", Vec::new(), true);
    }
}

#[derive(CommandEnum, Clone, Copy)]
#[command(crate = "crate", name = "MusicRepeatMode")]
pub(crate) enum MusicRepeatMode {
    #[command(name = "play_once")]
    PlayOnce,
    Loop,
}

/// `/music play|queue <track> [volume] [fade_seconds] [repeat_mode]`, `/music stop [fade_seconds]`,
/// `/music volume <volume>`.
/// Every player is told, the sender too (bds-music.txt).
#[derive(Command)]
#[command(crate = "crate", name = "music", description = "Allows you to control playing music tracks.", permission = "operator")]
pub(crate) enum Music {
    Play { track_name: String, volume: Option<f32>, fade_seconds: Option<f32>, repeat_mode: Option<MusicRepeatMode> },
    Queue { track_name: String, volume: Option<f32>, fade_seconds: Option<f32>, repeat_mode: Option<MusicRepeatMode> },
    Stop { fade_seconds: Option<f32> },
    Volume { volume: f32 },
}

const LOUDEST: f32 = 1.0;

impl Command for Music {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let volume = match &self {
            Self::Play { volume, .. } | Self::Queue { volume, .. } => *volume,
            Self::Volume { volume } => Some(*volume),
            Self::Stop { .. } => None,
        };
        if let Some(volume) = volume.filter(|volume| *volume > LOUDEST) {
            return out.translate("commands.generic.num.tooBig", vec![format!("{volume:.2}"), format!("{LOUDEST:.2}")], false);
        }
        let track = |track: String, volume: Option<f32>, fade: Option<f32>, mode: Option<MusicRepeatMode>, queued| {
            let looped = matches!(mode, Some(MusicRepeatMode::Loop));
            MusicAction::Play { track, volume: volume.unwrap_or(LOUDEST), fade_seconds: fade.unwrap_or(0.0), looped, queued }
        };
        let (key, shown, music) = match self {
            Self::Play { track_name, volume, fade_seconds, repeat_mode } => ("commands.music.success.playAction", vec![track_name.clone()], track(track_name, volume, fade_seconds, repeat_mode, false)),
            Self::Queue { track_name, volume, fade_seconds, repeat_mode } => ("commands.music.success.queueAction", vec![track_name.clone()], track(track_name, volume, fade_seconds, repeat_mode, true)),
            Self::Stop { fade_seconds } => ("commands.music.success.stopAction", Vec::new(), MusicAction::Stop { fade_seconds: fade_seconds.unwrap_or(0.0) }),
            Self::Volume { volume } => ("commands.music.success.volumeAction", vec![format!("{volume:.2}")], MusicAction::Volume(volume)),
        };
        let action = ScreenAction::Music(music);
        for player in ctx.players().collect::<Vec<_>>() {
            ctx.screen_action(player.id, &action);
        }
        out.translate(key, shown, true);
    }
}
