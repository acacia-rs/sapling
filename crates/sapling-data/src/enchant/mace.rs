//! A mace's smash and its enchantments, as BDS answered `bds_command_probe mace` and `mace-push`
//! (research/captures/bds-mace.txt, bds-mace-push.txt, bds-mace-breach.txt; summed up in
//! research/captures/bds-gameplay-findings.md).

/// A hit smashes once its hitter has fallen this far; past the second it is a heavy one.
pub const SMASHES_FROM: f32 = 1.5;
pub const HEAVY_FROM: f32 = 5.0;
/// What a block fallen adds: of the first 3, the next 5, and the rest; Density adds its own a level.
const PER_BLOCK: [(f32, f32); 3] = [(3.0, 4.0), (8.0, 2.0), (f32::INFINITY, 1.0)];
const DENSITY: f32 = 0.5;
/// How far around what was smashed mobs are shoved (each way, from its position), and the shove:
/// out by `SHOVE` a block nearer than `SHOVE_EVEN`, in by as much a block further, and up.
pub const SHOVE_REACH: f32 = 3.5;
const SHOVE: f32 = 0.6;
const SHOVE_EVEN: f32 = 2.5;
const SHOVE_UP: f32 = 0.6;
/// The hitter's launch up by Wind Burst I to III.
const WIND_BURST: [f32; 3] = [1.1, 1.63, 2.06];

/// What Breach takes a level off the share of a hit armour stops (bds-mace-breach.txt: a mace's 6
/// through a zombie's 10 points cost 4.176, then 5.076, 5.976 and 6 with Breach I to III).
const BREACH: f32 = 0.15;

/// The share of a hit Breach at `level` takes off what armour stops.
pub fn breach(level: u8) -> f32 {
    BREACH * f32::from(level)
}

/// What a smash after a fall of `fallen` adds to the hit, with Density at `density`; it goes in
/// before a critical's half again (a fall of 6.594 cost a golem 37.78: (6 + 19.19) x 1.5).
pub fn smash_bonus(fallen: f32, density: u8) -> f32 {
    let mut from = 0.0;
    let mut bonus = 0.0;
    for (to, each) in PER_BLOCK {
        bonus += (fallen.min(to) - from).max(0.0) * each;
        from = to;
    }
    bonus + DENSITY * f32::from(density) * fallen
}

/// The shove on a mob `apart` from what was smashed, the way `away` (across, of length one):
/// added to its motion, which BDS then tells after the tick's drag (0.6 up was told 0.43276805).
/// BDS shoves in, not out, past 2.5, and as hard from a heavy smash.
pub fn shove(away: [f32; 2], apart: f32) -> [f32; 3] {
    let out = SHOVE * (SHOVE_EVEN - apart);
    [away[0] * out, SHOVE_UP, away[1] * out]
}

/// How fast Wind Burst at `level` launches the hitter up after a smash.
pub fn wind_burst(level: u8) -> Option<f32> {
    WIND_BURST.get(usize::from(level).checked_sub(1)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_smash_adds_what_bds_took_from_a_golem() {
        // Falls of 1.593, 3.346, 6.594 and 15.909 cost (6 + bonus) x 1.5: 18.556, 28.039, 37.781, 53.864.
        for (fallen, cost) in [(1.593, 18.556), (3.346, 28.039), (6.594, 37.781), (15.909, 53.864)] {
            assert!(((6.0 + smash_bonus(fallen, 0)) * 1.5 - cost).abs() < 0.01, "{fallen}");
        }
    }

    #[test]
    fn the_shove_turns_inward_past_two_and_a_half() {
        // bds-mace-push.txt: cows 2.209 and 2.707 from the golem, told 0.0953 out and 0.0679 in after
        // a tick's ground drag.
        assert!((shove([1.0, 0.0], 2.209)[0] * 0.546 - 0.0953).abs() < 1e-3);
        assert!((shove([1.0, 0.0], 2.707)[0] * 0.546 + 0.0679).abs() < 1e-3);
    }
}
