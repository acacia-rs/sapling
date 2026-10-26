//! What the bow's, the crossbow's and the trident's enchantments do, as BDS reckons them
//! (docs/research/projectiles.md: `BowItem::releaseUsing` 142eee8d0, `CrossbowItem::use`
//! 1436ec1a0, `ImpactDamageSubcomponent::doOnHitEffect` 1424050a0).

/// Ticks Flame sets its arrow, and what the arrow hits, on fire.
pub const FLAME_BURNS: u16 = 100;
/// Power multiplies an arrow's harm by this and this much more a level.
const POWER: [f32; 2] = [1.25, 0.25];
/// Punch's strength a level, which throws this much of it across and this fast up.
const PUNCH: f32 = 1.6;
const PUNCH_ACROSS: f32 = 0.6;
const PUNCH_UP: f32 = 0.1;
const IMPALING: f32 = 2.5;
/// A crossbow loads in this many ticks, less this many a level of Quick Charge.
const CHARGE: u64 = 25;
const QUICKER: u64 = 5;
/// Multishot's side arrows leave this many degrees either way.
// TODO: BDS reads it from a field of the item that was not read; the Java game's.
const SIDE: f32 = 10.0;
/// Loyalty pulls a trident home this much a tick a level, and Riptide throws its rider at this
/// much a level and one more, in quarters.
// TODO: the Java game's; BDS's were not found.
const LOYALTY: f32 = 0.05;
const RIPTIDE: f32 = 3.0;

/// What Power at `level` multiplies an arrow's harm by.
pub fn power_bonus(level: u8) -> f32 {
    if level == 0 { 1.0 } else { POWER[0] + POWER[1] * f32::from(level) }
}

/// The speed away across and up Punch at `level` throws at.
pub fn punch(level: u8) -> [f32; 2] {
    if level == 0 { [0.0; 2] } else { [PUNCH * f32::from(level) * PUNCH_ACROSS, PUNCH_UP] }
}

/// What Impaling at `level` adds to a trident's harm on what stands in water or rain.
pub fn impaling(level: u8) -> f32 {
    IMPALING * f32::from(level)
}

/// Ticks a crossbow with Quick Charge at `level` takes to load.
pub fn charge_ticks(level: u8) -> u64 {
    CHARGE.saturating_sub(QUICKER * u64::from(level))
}

/// The turns, in degrees about the up axis, of the arrows a crossbow with Multishot at `level`
/// shoots; the first is the one that is picked up.
pub fn multishot(level: u8) -> &'static [f32] {
    if level == 0 { &[0.0] } else { &[0.0, -SIDE, SIDE] }
}

/// How many bodies an arrow with Piercing at `level` passes through before the one it stops in.
pub fn piercing(level: u8) -> u8 {
    level
}

/// How hard Loyalty at `level` pulls a trident home each tick.
pub fn loyalty_pull(level: u8) -> f32 {
    LOYALTY * f32::from(level)
}

/// The speed Riptide at `level` throws its rider at.
pub fn riptide(level: u8) -> f32 {
    RIPTIDE * (1.0 + f32::from(level)) / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archery_enchantments_are_reckoned_as_bds_reckons_them() {
        assert_eq!([0, 1, 5].map(power_bonus), [1.0, 1.5, 2.5]);
        assert_eq!([punch(0), punch(1)], [[0.0; 2], [1.6 * 0.6, 0.1]]);
        assert_eq!([0, 1, 3].map(charge_ticks), [25, 20, 10]);
        assert_eq!((multishot(0).len(), multishot(1).len(), impaling(2)), (1, 3, 5.0));
    }
}
