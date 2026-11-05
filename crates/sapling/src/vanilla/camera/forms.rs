//! `/camera`'s forms beyond `set`, `clear`, `fade` and the field of view, and the checks a `set`
//! makes of its offsets and easing, as BDS 1.26.52 answered them (research/captures/bds-h1camera.txt,
//! bds-h1cmds.txt; its run, 0x140ff03e0). A target or a spline wants the free camera.
// TODO: `play_spline` on a free camera: BDS's answer and packet were not drilled.

use std::collections::HashMap;

use sapling_api::command::{Args, Parameter, ParseError, Selector};
use sapling_api::{Output, Player, PlayerId, ScreenAction, Sender, ShardCtx, Target};

use super::super::targets;

/// `minecraft:free`'s place in CameraPresets.bin.
const FREE: u32 = 3;
/// Presets that take no offsets and no easing: first_person, third_person, third_person_front;
/// third_person's easing was not drilled.
const UNMOVED: [u32; 3] = [0, 4, 5];
/// The most an offset may be on each axis.
const MOST_OFFSET: f32 = 64.0;

pub(super) enum Form {
    Attach(Selector),
    Detach,
    Target { entity: Selector, offset: Option<[f32; 3]> },
    RemoveTarget,
    Spline,
}

/// The preset each player's camera was last `set` to; `clear` forgets it.
#[derive(Default)]
struct Presets(HashMap<PlayerId, u32>);

pub(super) fn parse(word: &str, args: &mut Args<'_>) -> Result<Option<Form>, ParseError> {
    Ok(Some(match word {
        "attach_to_entity" => Form::Attach(Selector::parse(args)?),
        "detach_from_entity" => Form::Detach,
        "target_entity" => {
            let entity = Selector::parse(args)?;
            let offset = match args.peek().is_some_and(|word| word.eq_ignore_ascii_case("target_center_offset")) {
                true => {
                    args.word()?;
                    Some([f32::parse(args)?, f32::parse(args)?, f32::parse(args)?])
                }
                false => None,
            };
            Form::Target { entity, offset }
        }
        "remove_target" => Form::RemoveTarget,
        "play_spline" => {
            args.word()?;
            Form::Spline
        }
        _ => return Ok(None),
    }))
}

/// Why a `set` of `preset` with these is refused, in the order BDS checks.
pub(super) fn refused_set(preset: u32, eased: bool, view: Option<[f32; 2]>, entity: Option<[f32; 3]>) -> Option<&'static str> {
    let unmoved = UNMOVED.contains(&preset);
    let far = |axes: &[f32]| axes.iter().any(|axis| axis.abs() > MOST_OFFSET);
    match () {
        _ if eased && unmoved => Some("commands.camera.invalid.easing"),
        _ if view.is_some_and(|view| far(&view)) => Some("commands.camera.invalid.view.offset"),
        _ if view.is_some() && unmoved => Some("commands.camera.invalid.offset.camera"),
        _ if entity.is_some_and(|entity| far(&entity)) => Some("commands.camera.invalid.entity.offset"),
        _ if entity.is_some() && unmoved => Some("commands.camera.invalid.offset.camera"),
        _ => None,
    }
}

/// Remembers the preset the players' cameras were set to, or forgets it (`None`).
pub(super) fn remember(ctx: &mut ShardCtx<'_>, found: &[Player], preset: Option<u32>) {
    let presets = &mut ctx.state::<Presets>().0;
    for player in found {
        match preset {
            Some(preset) => presets.insert(player.id, preset),
            None => presets.remove(&player.id),
        };
    }
}

/// The one entity or player `selector` names; an answer where none or many.
fn one(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: &Selector) -> Result<Target, &'static str> {
    let (players, entities) = targets(ctx, sender, Some(selector));
    let mut all = players.into_iter().map(Target::Player).chain(entities.into_iter().map(|entity| Target::Entity(entity.id)));
    match (all.next(), all.next()) {
        (None, _) => Err("commands.generic.noTargetMatch"),
        (Some(_), Some(_)) => Err("commands.generic.tooManyTargets"),
        (Some(target), None) => Ok(target),
    }
}

/// The action of `form` for `found`; an answer where it is refused.
pub(super) fn action(ctx: &mut ShardCtx<'_>, sender: &Sender, found: &[Player], form: Form) -> Result<Option<ScreenAction>, &'static str> {
    let all_free = found.iter().all(|player| ctx.state::<Presets>().0.get(&player.id) == Some(&FREE));
    Ok(match form {
        Form::Attach(selector) => Some(ScreenAction::AttachCamera(one(ctx, sender, &selector)?)),
        Form::Detach => Some(ScreenAction::DetachCamera),
        Form::RemoveTarget => Some(ScreenAction::RemoveCameraTarget),
        Form::Target { .. } | Form::Spline if !all_free => return Err("commands.generic.wrongTargetCamera"),
        Form::Target { entity, offset } => Some(ScreenAction::CameraTarget { target: one(ctx, sender, &entity)?, offset }),
        Form::Spline => None,
    })
}

#[cfg(test)]
mod tests {
    use super::refused_set;

    #[test]
    fn offsets_and_easing_are_refused_as_bds_refused_them() {
        let [first_person, fixed_boom, follow_orbit, third_person] = [0, 1, 2, 4];
        assert_eq!(refused_set(first_person, true, Some([1.0, 2.0]), Some([1.0, 2.0, 3.0])), Some("commands.camera.invalid.easing"));
        assert_eq!(refused_set(first_person, false, Some([1.0, 2.0]), None), Some("commands.camera.invalid.offset.camera"));
        assert_eq!(refused_set(third_person, false, None, Some([1.0, 2.0, 3.0])), Some("commands.camera.invalid.offset.camera"));
        assert_eq!(refused_set(follow_orbit, false, Some([40.0, 2.0]), None), None);
        assert_eq!(refused_set(follow_orbit, false, None, Some([1.0, 200.0, 3.0])), Some("commands.camera.invalid.entity.offset"));
        assert_eq!(refused_set(fixed_boom, true, Some([1.0, 2.0]), Some([1.0, 2.0, 3.0])), None);
    }
}
