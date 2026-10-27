//! Who and where each command an `/execute` runs is run for: a frame per entity a sub-command
//! names, its place, turn, anchor and dimension changed by each sub-command in turn
//! (research/captures/bds-execute.txt).

use sapling_api::command::{Executed, Selected, Selector};
use sapling_api::{Sender, ShardCtx};

use super::parse::Step;

/// Where BDS's `anchored eyes` and `facing entity .. eyes` put the eyes: 1.52 above the feet, for
/// a player (not the 1.62 its camera is), a pig, a cow, a chicken and an armour stand alike
/// (bds-execute.txt and bds-execute2.txt, `anchored eyes run tp @s ^ ^ ^`).
const EYES: f32 = 1.52;

#[derive(Clone)]
pub(super) struct Frame {
    pub executed: Executed,
}

impl Frame {
    /// The frame of the sender itself: its place and look, or the spawn for the console.
    pub fn of(ctx: &ShardCtx<'_>, sender: &Sender) -> Self {
        if let Sender::Executed(executed) = sender {
            return Self { executed: (**executed).clone() };
        }
        let player = sender.player();
        let position = player.and_then(|player| ctx.position(player.id));
        let rotation = position.as_ref().map_or([0.0, 0.0], |at| [at.yaw, at.pitch]);
        let dimension = player.and_then(|player| ctx.dimension_of(player.id)).unwrap_or(0);
        let executed = Executed::new(sender.clone(), player.cloned().map(Selected::Player), ctx.origin(sender), rotation, dimension);
        Self { executed }
    }

    /// The sender a command run in this frame has.
    pub fn sender(&self) -> Sender {
        Sender::Executed(Box::new(self.executed.clone()))
    }

    /// BDS keeps the anchor through `positioned`, `positioned as` and `at` (bds-execute2.txt).
    fn anchor(mut self, eyes: bool) -> Self {
        self.executed.anchor = if eyes && self.executed.executor.is_some() { EYES } else { 0.0 };
        self
    }

    fn placed(mut self, position: [f32; 3]) -> Self {
        self.executed.position = position;
        self
    }

    fn turned(mut self, rotation: [f32; 2]) -> Self {
        self.executed.rotation = rotation;
        self
    }
}

/// An entity's feet, look and dimension.
fn stance(ctx: &ShardCtx<'_>, entity: &Selected, dimension: i32) -> Option<([f32; 3], [f32; 2], i32)> {
    match entity {
        Selected::Player(player) => {
            let at = ctx.position(player.id)?;
            Some((at.feet, [at.yaw, at.pitch], ctx.dimension_of(player.id).unwrap_or(dimension)))
        }
        Selected::Entity(entity) => Some((entity.feet, [entity.yaw, entity.pitch], dimension)),
    }
}

/// The yaw and pitch of a look from `from` to `to`, as Java's `facing` turns.
fn facing(from: [f32; 3], to: [f32; 3]) -> [f32; 2] {
    let [dx, dy, dz] = std::array::from_fn(|axis| to[axis] - from[axis]);
    let wrap = |degrees: f32| (degrees + 180.0).rem_euclid(360.0) - 180.0;
    [wrap(dz.atan2(dx).to_degrees() - 90.0), wrap(-dy.atan2(dx.hypot(dz)).to_degrees())]
}

/// The frames `step` makes of `frame`: one, or one per entity it names (none when it names none).
/// Conditions are not steps here.
pub(super) fn apply(ctx: &mut ShardCtx<'_>, step: &Step, frame: Frame) -> Vec<Frame> {
    let each = |ctx: &mut ShardCtx<'_>, selector: &Selector, change: &dyn Fn(&ShardCtx<'_>, Frame, &Selected) -> Option<Frame>| -> Vec<Frame> {
        let named = ctx.select(&frame.sender(), selector);
        named.iter().filter_map(|entity| change(ctx, frame.clone(), entity)).collect()
    };
    let dimension = frame.executed.dimension;
    match step {
        Step::As(selector) => each(ctx, selector, &|_, mut frame, entity| {
            frame.executed.executor = Some(entity.clone());
            Some(frame)
        }),
        Step::At(selector) => each(ctx, selector, &|ctx, mut frame, entity| {
            let (feet, look, dimension) = stance(ctx, entity, dimension)?;
            (frame.executed.position, frame.executed.dimension) = (feet, dimension);
            Some(frame.turned(look))
        }),
        Step::PositionedAs(selector) => each(ctx, selector, &|ctx, frame, entity| Some(frame.placed(stance(ctx, entity, dimension)?.0))),
        Step::RotatedAs(selector) => each(ctx, selector, &|ctx, frame, entity| Some(frame.turned(stance(ctx, entity, dimension)?.1))),
        Step::FacingEntity(selector, eyes) => each(ctx, selector, &|ctx, frame, entity| {
            let [x, y, z] = stance(ctx, entity, dimension)?.0;
            let lift = if *eyes { EYES } else { 0.0 };
            let look = facing(frame.executed.position, [x, y + lift, z]);
            Some(frame.turned(look))
        }),
        Step::In(dimension) => {
            let mut frame = frame;
            frame.executed.dimension = *dimension;
            vec![frame]
        }
        Step::Positioned(coords) => {
            let at = coords.resolve(ctx, &frame.sender());
            vec![frame.placed(at)]
        }
        Step::Rotated([yaw, pitch]) => {
            let [was_yaw, was_pitch] = frame.executed.rotation;
            vec![frame.turned([yaw.from(was_yaw), pitch.from(was_pitch)])]
        }
        // BDS turns from the feet even when anchored at the eyes (bds-execute2.txt `anchored eyes facing`).
        Step::Facing(coords) => {
            let look = facing(frame.executed.position, coords.resolve(ctx, &frame.sender()));
            vec![frame.turned(look)]
        }
        Step::Align(axes) => {
            let mut frame = frame;
            for (axis, at) in frame.executed.position.iter_mut().enumerate() {
                if axes[axis] {
                    *at = at.floor();
                }
            }
            vec![frame]
        }
        Step::Anchored(eyes) => vec![frame.anchor(*eyes)],
        Step::If(..) => vec![frame],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_turns_as_java_turns() {
        let [yaw, pitch] = facing([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        assert_eq!((yaw, pitch), (-90.0, 0.0), "east is -90");
        let [yaw, pitch] = facing([0.0, 0.0, 0.0], [0.0, 1.0, 1.0]);
        assert!((yaw.abs(), pitch) == (0.0, -45.0), "south, looking up");
    }
}
