//! A mob's health and what wears it down, tick by tick, as BDS was seen to
//! (docs/research/mob-damage.md); what finds the fire, the wall and the water, and tells the
//! viewers, is harm.rs.

use bevy_ecs::prelude::Component;

/// Ticks after a hit in which only a harder one lands, and only by what it adds.
const GUARD: u8 = 10;
/// Ticks a mob burns on after it last touched fire or lava, hurt each `BURN_EVERY`.
const BURNS: u16 = 160;
const BURN_EVERY: u16 = 20;
const FIRE_HARM: f32 = 1.0;
const LAVA_HARM: f32 = 4.0;
/// A fall this far does not hurt; each block more, begun, costs 1.
const SAFE_FALL: f32 = 3.0;
/// Ticks of breath; out of it under water, each tick hurts.
pub(super) const AIR: i16 = 300;
/// Ticks a mob's head is in a block before the block hurts it.
pub(super) const WALLED: u8 = 6;
const SMOTHER_HARM: f32 = 2.0;
const DROWN_HARM: f32 = 2.0;

/// What hurt: `EntityEvent` names it by its number.
pub(crate) use sapling_api::DamageCause as Cause;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Health {
    left: f32,
    max: f32,
    /// The last hit that landed, while `guarded` ticks of its guard are left.
    last: f32,
    guarded: u8,
    burning: u16,
    /// The highest it has been since it left the ground.
    peak: Option<f32>,
    air: i16,
    /// Ticks running its head has been in a block.
    walled: u8,
    /// A landing that hurts in the tick after it: what it costs, and where that tick began.
    landed: Option<(f32, [f32; 3])>,
    /// The points and toughness of what it wears.
    armour: [f32; 2],
    /// Health on top of its own that hits take first (the absorption effect).
    cushion: f32,
    /// What it takes of a hit (the resistance effect), and whether fire does nothing to it.
    takes: f32,
    fireproof: bool,
    /// Blocks of a fall forgiven on top of the three anyone is (jump boost, slow falling).
    springs: f32,
    /// What viewers know: its health, and whether it is on fire.
    pub(super) told: f32,
    pub(super) lit: bool,
}

impl Health {
    /// A mob of `max` health with `left` of it, `burning` ticks more.
    pub(crate) fn new(max: f32, left: Option<f32>, burning: u16) -> Self {
        let left = left.map_or(max, |left| left.clamp(0.0, max));
        Self { left, max, last: 0.0, guarded: 0, burning, peak: None, air: AIR, walled: 0, landed: None, armour: [0.0; 2], cushion: 0.0, takes: 1.0, fireproof: false, springs: 0.0, told: left, lit: false }
    }

    pub(crate) fn left(&self) -> f32 {
        self.left
    }

    pub(crate) fn max(&self) -> f32 {
        self.max
    }

    pub(crate) fn burning(&self) -> u16 {
        self.burning
    }

    pub(crate) fn is_burning(&self) -> bool {
        self.burning > 0
    }

    pub(crate) fn mend(&mut self, by: f32) {
        self.left = (self.left + by).min(self.max);
    }

    pub(crate) fn end(&mut self) {
        self.left = 0.0;
    }

    pub(crate) fn cushion(&self) -> f32 {
        self.cushion
    }

    pub(crate) fn pad(&mut self, cushion: f32) {
        self.cushion = cushion;
    }

    pub(crate) fn ward(&mut self, takes: f32, fireproof: bool) {
        (self.takes, self.fireproof) = (takes, fireproof);
    }

    pub(crate) fn spring(&mut self, blocks: f32) {
        self.springs = blocks;
    }

    pub(crate) fn is_fireproof(&self) -> bool {
        self.fireproof
    }

    pub(crate) fn wear(&mut self, armour: [f32; 2]) {
        self.armour = armour;
    }

    /// What is left of a hit of `amount` behind its armour: the Java game's rule, which BDS
    /// follows (docs/research/player-damage.md, "Armour").
    pub(crate) fn shielded(&self, amount: f32) -> f32 {
        let [points, toughness] = self.armour;
        amount * (1.0 - (points - amount / (2.0 + toughness / 4.0)).clamp(points * 0.2, 20.0) / 25.0)
    }

    /// Whether a hit of `amount` landed.
    pub(crate) fn hurt(&mut self, amount: f32) -> bool {
        let amount = amount * self.takes;
        let taken = if self.guarded > 0 { amount - self.last } else { amount };
        if taken <= 0.0 || self.left <= 0.0 {
            return false;
        }
        (self.last, self.guarded) = (amount, GUARD);
        let soaked = taken.min(self.cushion);
        self.cushion -= soaked;
        self.left = (self.left - (taken - soaked)).max(0.0);
        true
    }

    /// Starts a tick: a tick of the last hit's guard is over.
    pub(crate) fn begin(&mut self) {
        self.guarded = self.guarded.saturating_sub(1);
    }

    /// A tick's move from `from` to the height `to`, landing `on_ground` or `in_liquid`: the
    /// last tick's landing if it hurt, as what it cost and where that tick began. BDS hurts a
    /// tick after the move that lands.
    pub(crate) fn fall(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool) -> Option<(f32, [f32; 3])> {
        let due = self.landed.take().filter(|(harm, _)| self.hurt(*harm));
        let peak = self.peak.map_or(from[1], |peak| peak.max(from[1])).max(to);
        self.peak = (!on_ground && !in_liquid).then_some(peak);
        let harm = (peak - to - SAFE_FALL - self.springs).ceil();
        if on_ground && !in_liquid && harm > 0.0 {
            self.landed = Some((harm, from));
        }
        due
    }

    /// A tick of fire, `touching` fire or lava, or in water: what hurt it, if anything.
    pub(crate) fn burn(&mut self, touching: bool, wet: bool) -> Option<Cause> {
        if touching {
            self.burning = BURNS;
            return (!self.fireproof && self.hurt(self.shielded(FIRE_HARM))).then_some(Cause::Fire);
        }
        if wet || self.burning == 0 {
            self.burning = 0;
            return None;
        }
        self.burning -= 1;
        (self.burning > 0 && self.burning % BURN_EVERY == 0 && !self.fireproof && self.hurt(FIRE_HARM)).then_some(Cause::FireTick)
    }

    /// A tick with its head in a block or not: whether the block, borne `spared` ticks, hurt it.
    pub(crate) fn smother(&mut self, in_wall: bool, spared: u8) -> bool {
        self.walled = if in_wall { self.walled.saturating_add(1) } else { 0 };
        self.walled > spared && self.hurt(SMOTHER_HARM)
    }

    pub(crate) fn scald(&mut self) -> bool {
        !self.fireproof && self.hurt(self.shielded(LAVA_HARM))
    }

    /// A tick of breath, held where it has none to draw: its breath when that changed.
    pub(crate) fn breathe(&mut self, held: bool) -> Option<i16> {
        let air = if held { (self.air - 1).max(0) } else { AIR };
        (std::mem::replace(&mut self.air, air) != air).then_some(air)
    }

    /// Whether, out of breath under water, it was hurt.
    pub(crate) fn drown(&mut self, under_water: bool) -> bool {
        under_water && self.air == 0 && self.hurt(DROWN_HARM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_takes_off_a_hit_what_it_took_off_on_bds() {
        // research/captures/bds-armour.txt: a leather and an iron chestplate, all of iron, of
        // diamond and of netherite against 5, and the netherite against 2.
        let behind = |armour: [f32; 2], amount: f32| {
            let mut health = Health::new(20.0, None, 0);
            health.wear(armour);
            (health.shielded(amount) * 1000.0).round() / 1000.0
        };
        let taken = [behind([0.0, 0.0], 5.0), behind([3.0, 0.0], 5.0), behind([6.0, 0.0], 5.0), behind([15.0, 0.0], 5.0), behind([20.0, 8.0], 5.0), behind([20.0, 12.0], 5.0), behind([20.0, 12.0], 2.0)];
        assert_eq!(taken, [5.0, 4.88, 4.3, 2.5, 1.25, 1.2, 0.432]);
    }

    #[test]
    fn a_pig_in_lava_is_hurt_as_bds_hurt_one() {
        // research/captures/bds-burn.txt: touching from its 1st tick, in the lava from its 3rd.
        let mut pig = Health::new(10.0, None, 0);
        let mut seen = Vec::new();
        for tick in 1..=23 {
            pig.begin();
            let fire = pig.burn(true, false);
            let after_fire = pig.left;
            let lava = tick >= 3 && pig.scald();
            if fire.is_some() || lava {
                seen.push((tick, after_fire, pig.left));
            }
        }
        assert_eq!(seen, [(1, 9.0, 9.0), (3, 9.0, 6.0), (13, 5.0, 2.0), (23, 1.0, 0.0)]);
    }

    #[test]
    fn out_of_lava_a_mob_burns_on_for_eight_seconds_hurt_each_second() {
        let mut golem = Health::new(100.0, None, 0);
        assert_eq!(golem.burn(true, false), Some(Cause::Fire));
        let hits: Vec<u16> = (1..=200)
            .filter(|_| {
                golem.begin();
                golem.burn(false, false) == Some(Cause::FireTick)
            })
            .collect();
        assert_eq!(hits, [20, 40, 60, 80, 100, 120, 140]);
        assert!(!golem.is_burning() && golem.left() == 92.0);
        golem.burn(true, false);
        assert!(golem.is_burning() && golem.burn(false, true).is_none() && !golem.is_burning(), "water puts it out");
    }

    #[test]
    fn a_fall_costs_each_block_begun_past_three_as_on_bds() {
        // research/captures/bds-harm.txt: cows dropped 3 to 8 blocks.
        let at = |y: f32| [0.5, y, 0.5];
        let cost = |height: f32| {
            let mut cow = Health::new(10.0, None, 0);
            let middle = 60.0 + height / 2.0;
            assert_eq!((cow.fall(at(60.0 + height), middle, false, false), cow.fall(at(middle), 60.0, true, false)), (None, None), "not in the tick it lands");
            cow.fall(at(60.0), 60.0, true, false)
        };
        let from = |height: f32| at(60.0 + height / 2.0);
        assert_eq!([3.0, 3.5, 4.0, 8.0].map(cost), [None, Some((1.0, from(3.5))), Some((1.0, from(4.0))), Some((5.0, from(8.0)))]);
        assert_eq!([4.5, 5.0].map(|height| cost(height).map(|(harm, _)| harm)), [Some(2.0), Some(2.0)]);
        let mut cow = Health::new(10.0, None, 0);
        cow.fall(at(80.0), 70.0, false, false);
        cow.fall(at(70.0), 65.0, false, true);
        assert_eq!((cow.fall(at(65.0), 64.0, true, false), cow.fall(at(64.0), 64.0, true, false)), (None, None), "water breaks a fall");
    }

    #[test]
    fn a_block_smothers_from_the_seventh_tick_and_water_once_breath_is_out() {
        let mut cow = Health::new(10.0, None, 0);
        let hits: Vec<i32> = (1..=27)
            .filter(|_| {
                cow.begin();
                cow.smother(true, WALLED)
            })
            .collect();
        assert_eq!((hits, cow.left()), (vec![7, 17, 27], 4.0));
        assert!(!cow.smother(false, WALLED) && !cow.smother(true, WALLED), "counted anew");

        let mut cow = Health::new(10.0, None, 0);
        let breaths: Vec<i16> = (0..AIR).filter_map(|_| cow.breathe(true)).collect();
        assert_eq!((breaths.len(), breaths.last()), (AIR as usize, Some(&0)));
        assert!(cow.breathe(true).is_none() && cow.drown(true) && !cow.drown(true), "2 a hit, guarded");
        assert_eq!((cow.left(), cow.breathe(false)), (8.0, Some(AIR)));
    }
}
