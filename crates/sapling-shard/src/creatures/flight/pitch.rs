//! How a flying shot pitches, as BDS's did (research/captures/bds-shoot.txt, bds-s7shots.txt): each move
//! it turns a fifth of the way to the move's own angle (the Java game's `lerpRotation`), and all the way
//! at once from a shot that was announced unturned, as a summoned one is (byte 192 at its first fall).

use bevy_ecs::prelude::*;

use super::{Arrow, facing};

const SHARE: f32 = 0.2;

/// The shot's pitch after a move by `gone`.
pub(super) fn turn(world: &mut World, arrow: Entity, gone: [f32; 3]) -> Option<f32> {
    let mut state = world.get_mut::<Arrow>(arrow)?;
    state.turn = turned(state.turn, gone);
    Some(state.turn[1])
}

fn turned(turn: [f32; 2], gone: [f32; 3]) -> [f32; 2] {
    let toward = facing(gone);
    if turn == [0.0, 0.0] {
        return toward;
    }
    [turn[0], turn[1] + (toward[1] - turn[1]) * SHARE]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_data::angle_byte;

    #[test]
    fn a_skeletons_arrow_pitches_as_bdss_did() {
        // bds-shoot.txt: the arrow's announced motion, then its moves; BDS told no pitch byte until
        // the 4th move, then 2, 1, (1), 0, (0), 255, 254, 252, 251.
        let at = [
            [6.5, -17.390001, 13.233621], [6.5179486, -17.226528, 14.9385195], [6.5357175, -17.11469, 16.62637], [6.553309, -17.053972, 18.29734],
            [6.5707245, -17.04386, 19.951603], [6.587966, -17.083849, 21.589323], [6.605035, -17.173439, 23.210665], [6.621933, -17.312132, 24.815794],
            [6.6386623, -17.499437, 26.404871], [6.6552243, -17.73487, 27.978058], [6.671621, -18.01795, 29.535513], [6.6878533, -18.348198, 31.077393],
            [6.703923, -18.725143, 32.603855],
        ];
        let mut turn = facing([0.017948544, 0.16347232, 1.7048991]);
        let mut bytes = vec![angle_byte(turn[1])];
        for pair in at.windows(2) {
            turn = turned(turn, std::array::from_fn(|axis| pair[1][axis] - pair[0][axis]));
            bytes.push(angle_byte(turn[1]));
        }
        assert_eq!(bytes, [3, 3, 3, 3, 2, 1, 1, 0, 0, 255, 254, 252, 251]);
    }

    #[test]
    fn a_summoned_shot_turns_to_its_fall_at_once() {
        assert_eq!(angle_byte(turned([0.0, 0.0], [0.0, -0.05, 0.0])[1]), 192);
    }
}
