//! A fish's pitch, as BDS's cod kept it (fourteenth1ak gdb on the ah1swim scene; docs/research/mob-kinds.md):
//! each tick its `SwimMoveControl` (Linux 0xbdf8d50, written at 0xbdf952d) sets the look-at a step towards its
//! spot from its head, `(2 dx / d, dy / d, 2 dz / d)` off it (only x is doubled in place, so y is not), and its
//! look control (0xbdf78f0) turns the pitch at most 40 degrees towards that point from its eyes. Head and eyes
//! are last tick's feet raised 0.85 and 0.9 of its height (0.255 and 0.27 on the 0.3-tall cod). Its push up or
//! down is signed by the pitch it had, down at 0, and its push across shrinks by the pitch's cosine.

/// Its head and eyes are this much of its height over last tick's feet.
const HEAD: f32 = 0.85;
const EYES: f32 = 0.9;
/// The pitch turns at most this many degrees a tick.
const TURN: f32 = 40.0;
const IDLE_TURN: f32 = 10.0;
/// Spots nearer than this give no step.
const NEAR: f32 = 1e-4;
/// The push up or down is the push across times this much of the height to the spot, at least |sin pitch|.
const RISE: f32 = 5.0;
const RISE_MOST: f32 = 1.25;

/// The pitch a tick on from `pitch`, aiming from `was` (last tick's feet) a step from `feet` towards `spot`.
pub(super) fn next(pitch: f32, was: [f32; 3], feet: [f32; 3], spot: [f32; 3], height: f32) -> f32 {
    let off: [f32; 3] = std::array::from_fn(|axis| spot[axis] - feet[axis]);
    let far = off.iter().map(|c| c * c).sum::<f32>().sqrt();
    let step = if far < NEAR { [0.0; 3] } else { [2.0 * off[0] / far, off[1] / far, 2.0 * off[2] / far] };
    let look = [was[0] + step[0], was[1] + HEAD * height + step[1], was[2] + step[2]];
    let dy = look[1] - (was[1] + EYES * height);
    let wanted = -dy.atan2(step[0].hypot(step[2])).to_degrees();
    let turn = (wanted - pitch + 180.0).rem_euclid(360.0) - 180.0;
    pitch + turn.clamp(-TURN, TURN)
}

/// The pitch a tick on with no spot to swim to: [`IDLE_TURN`] at most back to level (0 error over 34 such ticks).
pub(super) fn idle(pitch: f32) -> f32 {
    pitch - pitch.clamp(-IDLE_TURN, IDLE_TURN)
}

/// Its push across and up or down for a push of `push` at `pitch`, `dy` under or over its spot.
pub(super) fn push(pitch: f32, push: f32, dy: f32) -> (f32, f32) {
    let sin = pitch.to_radians().sin();
    let up = sin.abs().max((RISE * dy.abs()).min(RISE_MOST)) * push;
    (push * pitch.to_radians().cos(), if sin >= 0.0 { -up } else { up })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cods_pitch_aims_from_its_eyes_a_step_towards_its_spot() {
        // bds-ah1swim gdb (fourteenth1ak): feet (32.5, -58, 24.5098), last tick's (32.5, -58, 24.5), spot 1 under and
        // 2 on: look-at (32.5, -58.192, 26.289), pitch 0 → 14.487.
        let pitch = next(0.0, [32.5, -58.0, 24.5], [32.5, -58.0, 24.5], [32.5, -59.0, 26.5], 0.3);
        assert!((pitch - 14.487).abs() < 0.01, "{pitch}");
    }

    #[test]
    fn a_cod_with_no_spot_levels_out_ten_degrees_a_tick() {
        assert!(idle(25.0) == 15.0 && idle(-4.0) == 0.0);
    }

    #[test]
    fn a_cods_push_is_signed_by_its_pitch_and_down_at_none() {
        // bds-ah1swim gdb: at pitch 0, 1 over its spot, it fell 0.0098 (1.25 x 0.0098 after water's 0.8).
        let (across, up) = push(0.0, 0.0098, -1.0);
        assert!((across - 0.0098).abs() < 1e-7 && (up + 0.01225).abs() < 1e-6, "{across} {up}");
        // Pitched up while under its spot's height it still rises, by |sin pitch| near the spot.
        let (_, up) = push(-30.0, 0.0098, 0.01);
        assert!((up - 0.5 * 0.0098).abs() < 1e-6, "{up}");
    }
}
