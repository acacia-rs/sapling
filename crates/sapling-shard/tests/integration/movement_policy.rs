//! A player's moves under Correct, and a plugin's own policy for one player.

use crate::common;

use acacia_proto::packets::CorrectPlayerMovePrediction;
use common::{Harness, all, config, input_at, platform};
use sapling_api::{MovementPolicy, PlayerId};
use sapling_data::EYE_HEIGHT;
use sapling_shard::{MovementCheck, ShardConfig};

const ALEX: PlayerId = PlayerId(1);

fn correcting() -> Harness {
    let check = MovementCheck { policy: MovementPolicy::Correct, ..MovementCheck::default() };
    let mut h = Harness::with(ShardConfig { movement_check: check, ..config() }, platform());
    h.enter(1, "Alex");
    h
}

/// The corrections Alex is sent as he says he stands at `from` (input tick `first`), then rises
/// half a block an input for twelve more.
fn rise(h: &mut Harness, from: f32, first: u64) -> Vec<CorrectPlayerMovePrediction> {
    (0..=12)
        .flat_map(|up| {
            let mut input = input_at([0.5, from + 0.5 * up as f32, 0.5]);
            input.tick = first + up;
            h.receive(1, &input);
            all::<CorrectPlayerMovePrediction>(h.tick_all().to(1))
        })
        .collect()
}

#[test]
fn a_player_rising_with_nothing_under_it_is_corrected_onto_the_ground_every_five_inputs() {
    let mut h = correcting();
    let told = rise(&mut h, 65.0, 100);
    let ticks: Vec<u64> = told.iter().map(|correction| correction.tick).collect();
    assert_eq!(ticks, [102, 107, 112], "past half a block, then once each correction is through");
    assert!(told.iter().all(|correction| correction.on_ground && correction.position.y < 65.01 + EYE_HEIGHT), "{told:?}");
}

#[test]
fn a_player_a_plugin_trusts_is_not_corrected_until_it_is_given_back() {
    let mut h = correcting();
    h.remote.run(|ctx| assert!(ctx.set_movement_policy(ALEX, Some(MovementPolicy::Trust)) && ctx.movement_policy(ALEX) == Some(MovementPolicy::Trust)));
    assert!(rise(&mut h, 65.0, 100).is_empty());
    h.remote.run(|ctx| assert!(ctx.set_movement_policy(ALEX, None) && ctx.movement_policy(ALEX) == Some(MovementPolicy::Correct)));
    // The server starts again from where the client is, here in the air, and sees it rise.
    assert!(!rise(&mut h, 71.0, 200).is_empty());
}
