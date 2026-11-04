//! A skeleton's aim and spread, against research/captures/bds-e1arrow.txt (mob-attacks.md "In the way").

use crate::common;

use acacia_proto::packets::AddEntity;
use common::{Harness, all};
use sapling_api::{Difficulty, Effect, GameMode, PlayerId};

/// The arrows a skeleton 6 north of Alex (who takes no harm) looses in 520 ticks on `difficulty`:
/// each one's launch made one long.
fn aims(difficulty: Difficulty) -> Vec<[f32; 3]> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(2, &common::input_at([0.5, 65.0, -6.5]));
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        assert!(ctx.give_effect(PlayerId(1), Effect::Resistance, 1000, 4, false));
        assert!(ctx.spawn_entity("minecraft:skeleton", [0.5, 65.0, 6.5]).is_some());
    });
    let arrows: Vec<AddEntity> = (0..520).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).filter(|added| added.entity_type == "minecraft:arrow").collect();
    arrows
        .iter()
        .map(|arrow| {
            let v = &arrow.velocity;
            let long = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
            [v.x / long, v.y / long, v.z / long]
        })
        .collect()
}

fn rms(parts: impl Iterator<Item = f32>) -> f32 {
    let parts: Vec<f32> = parts.collect();
    (parts.iter().map(|part| part * part).sum::<f32>() / parts.len() as f32).sqrt()
}

#[test]
fn a_skeleton_aims_level_at_its_preys_eyes_and_spreads_less_on_harder_levels() {
    let (easy, hard) = (aims(Difficulty::Easy), aims(Difficulty::Hard));
    assert!(easy.len() >= 7 && hard.len() >= 7, "a shot each 61 ticks: {} {}", easy.len(), hard.len());
    let mean_rise = easy.iter().map(|aim| aim[1]).sum::<f32>() / easy.len() as f32;
    // 0.0075 x 12 a part on easy, x 4 on hard; across is x and up y, the prey due south.
    let (easy_off, hard_off) = (rms(easy.iter().flat_map(|aim| [aim[0], aim[1]])), rms(hard.iter().flat_map(|aim| [aim[0], aim[1]])));
    assert!(mean_rise.abs() < 0.07 && (0.05..0.14).contains(&easy_off) && hard_off < 0.05, "{mean_rise} {easy_off} {hard_off}");
}
