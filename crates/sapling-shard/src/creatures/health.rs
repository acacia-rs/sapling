//! A mob's health and what wears it down, tick by tick, as BDS was seen to
//! (docs/research/mob-damage.md); what finds the fire, the wall and the water, and tells the
//! viewers, is harm.rs.

use bevy_ecs::prelude::Component;
use sapling_data::enchant::Warding;

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
    /// The points and toughness of what it wears, and what that armour's enchantments ward off.
    armour: [f32; 2],
    warding: Warding,
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
        Self { left, max, last: 0.0, guarded: 0, burning, peak: None, air: AIR, walled: 0, landed: None, armour: [0.0; 2], warding: Warding::default(), cushion: 0.0, takes: 1.0, fireproof: false, springs: 0.0, told: left, lit: false }
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

    /// Sets the most it can have; what it has over that goes.
    pub(crate) fn grow(&mut self, max: f32) {
        (self.max, self.left) = (max, self.left.min(max));
    }

    pub(crate) fn spring(&mut self, blocks: f32) {
        self.springs = blocks;
    }

    pub(crate) fn is_fireproof(&self) -> bool {
        self.fireproof
    }

    pub(crate) fn wear(&mut self, armour: [f32; 2], warding: Warding) {
        (self.armour, self.warding) = (armour, warding);
    }

    /// What is left of a hit of `amount` by `cause` past its armour's protection enchantments.
    pub(crate) fn warded(&self, amount: f32, cause: Cause) -> f32 {
        crate::enchants::harm_of(cause).map_or(amount, |harm| self.warding.spared(amount, harm))
    }

    /// What is left of a hit of `amount` behind its armour: the Java game's rule, which BDS
    /// follows (docs/research/player-damage.md, "Armour").
    pub(crate) fn shielded(&self, amount: f32) -> f32 {
        self.shielded_but(amount, 0.0)
    }

    /// [`Health::shielded`] with `pierced` taken off the share the armour stops (a mace's Breach).
    pub(crate) fn shielded_but(&self, amount: f32, pierced: f32) -> f32 {
        let [points, toughness] = self.armour;
        let stops = (points - amount / (2.0 + toughness / 4.0)).clamp(points * 0.2, 20.0) / 25.0;
        amount * (1.0 - (stops - pierced).max(0.0))
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

    /// Takes `amount` as it is, past the guard and resistance, the cushion first: a loss a plugin
    /// set (damage.rs).
    pub(crate) fn lose(&mut self, amount: f32) {
        let soaked = amount.min(self.cushion);
        self.cushion -= soaked;
        self.left = (self.left - (amount - soaked)).max(0.0);
    }

    /// What it lost since `before`, its cushion too.
    pub(crate) fn lost_since(&self, before: &Health) -> f32 {
        before.left + before.cushion - self.left - self.cushion
    }

    /// Gives back what it lost since `before` and takes `asked` instead, or nothing (and the
    /// guard back too) where a plugin cancelled the hit (damage.rs). Timers stay as they ran.
    pub(crate) fn reweigh(&mut self, before: &Health, asked: Option<f32>) {
        (self.left, self.cushion) = (before.left, before.cushion);
        match asked {
            Some(amount) => self.lose(amount),
            None => (self.last, self.guarded) = (before.last, before.guarded),
        }
    }

    /// Its health and the most, where viewers were last told otherwise; they are taken as told.
    pub(crate) fn untold(&mut self) -> Option<(f32, f32)> {
        (self.left != self.told).then(|| {
            self.told = self.left;
            (self.left, self.max)
        })
    }

    /// Whether the last hit's guard (BDS's hurt time) is still running.
    pub(crate) fn is_guarded(&self) -> bool {
        self.guarded > 0
    }

    /// Starts a tick: a tick of the last hit's guard is over.
    pub(crate) fn begin(&mut self) {
        self.guarded = self.guarded.saturating_sub(1);
    }

    /// A tick's move from `from` to the height `to`, landing `on_ground` or `in_liquid`: the
    /// last tick's landing if it hurt, as what it cost and where that tick began. BDS hurts a
    /// tick after the move that lands.
    pub(crate) fn fall(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool) -> Option<(f32, [f32; 3])> {
        let due = self.landed.take().filter(|(harm, _)| self.hurt(self.warded(*harm, Cause::Fall)));
        let peak = self.peak.map_or(from[1], |peak| peak.max(from[1])).max(to);
        self.peak = (!on_ground && !in_liquid).then_some(peak);
        let harm = (peak - to - SAFE_FALL - self.springs).ceil();
        if on_ground && !in_liquid && harm > 0.0 {
            self.landed = Some((harm, from));
        }
        due
    }

    /// Whether it was off the ground when its last tick ended.
    pub(crate) fn is_airborne(&self) -> bool {
        self.peak.is_some()
    }

    /// What the landing of this tick will cost, the next.
    pub(crate) fn landing(&self) -> Option<f32> {
        self.landed.map(|(harm, _)| harm)
    }

    /// How far it has fallen when it comes from `from` to `to`.
    pub(crate) fn dropped(&self, from: f32, to: f32) -> f32 {
        self.peak.map_or(from, |peak| peak.max(from)).max(to) - to
    }

    /// Forgets how far it has fallen: a fall from `at` starts anew.
    pub(crate) fn break_fall(&mut self, at: f32) {
        if self.peak.is_some() {
            self.peak = Some(at);
        }
    }

    /// Sets it on fire for `ticks` more, less what Fire Protection spares it, if it would not burn
    /// that long anyway.
    pub(crate) fn ignite(&mut self, ticks: u16) {
        self.burning = self.burning.max(self.warding.burns_for(ticks));
    }

    /// Sets it on fire for `ticks` if it is not: the first hurts at once, as the sun's did on
    /// BDS (research/captures/bds-sun.txt).
    pub(crate) fn kindle(&mut self, ticks: u16) {
        if self.burning == 0 {
            self.burning = ticks + 1;
        }
    }

    /// A tick of fire, `touching` fire or lava, or in water: what hurt it, if anything.
    pub(crate) fn burn(&mut self, touching: bool, wet: bool) -> Option<Cause> {
        if touching {
            self.burning = self.warding.burns_for(BURNS);
            return (!self.fireproof && self.hurt(self.warded(self.shielded(FIRE_HARM), Cause::Fire))).then_some(Cause::Fire);
        }
        if wet || self.burning == 0 {
            self.burning = 0;
            return None;
        }
        self.burning -= 1;
        (self.burning > 0 && self.burning % BURN_EVERY == 0 && !self.fireproof && self.hurt(self.warded(FIRE_HARM, Cause::FireTick))).then_some(Cause::FireTick)
    }

    /// A tick with its head in a block or not: whether the block, borne `spared` ticks, hurt it.
    pub(crate) fn smother(&mut self, in_wall: bool, spared: u8) -> bool {
        self.walled = if in_wall { self.walled.saturating_add(1) } else { 0 };
        self.walled > spared && self.hurt(SMOTHER_HARM)
    }

    pub(crate) fn scald(&mut self) -> bool {
        !self.fireproof && self.hurt(self.warded(self.shielded(LAVA_HARM), Cause::Lava))
    }

    /// A tick of breath, held where it has none to draw: its breath when that changed.
    pub(crate) fn breathe(&mut self, held: bool) -> Option<i16> {
        let air = if held { (self.air - 1).max(0) } else { AIR };
        (std::mem::replace(&mut self.air, air) != air).then_some(air)
    }

    /// Whether, out of breath under water, it was hurt.
    pub(crate) fn drown(&mut self, under_water: bool) -> bool {
        under_water && self.air == 0 && self.hurt(self.warded(DROWN_HARM, Cause::Drowning))
    }
}

#[cfg(test)]
mod tests;
