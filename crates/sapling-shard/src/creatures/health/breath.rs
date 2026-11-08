//! A mob's breath: 300 ticks of it, or its kind's own supply (a dolphin's 4800, mob-traits.tsv
//! `breath=`), a tick of it lost each tick it is held and all of it back the first it is not.

use super::{AIR, Cause, DROWN_HARM, Health, SMOTHER_HARM};

impl Health {
    /// It holds `supply` ticks of breath, full now; `None` keeps the common 300.
    pub(crate) fn hold_air(&mut self, supply: Option<u32>) {
        let supply = supply.map_or(AIR, |ticks| ticks.min(i16::MAX as u32) as i16);
        (self.air, self.supply) = (supply, supply);
    }

    pub(crate) fn air(&self) -> i16 {
        self.air
    }

    /// A tick of breath, held where it has none to draw: its breath when that changed.
    pub(crate) fn breathe(&mut self, held: bool) -> Option<i16> {
        let air = if held { (self.air - 1).max(0) } else { self.supply };
        (std::mem::replace(&mut self.air, air) != air).then_some(air)
    }

    /// Whether, out of breath under water, it was hurt.
    pub(crate) fn drown(&mut self, under_water: bool) -> bool {
        under_water && self.air == 0 && self.hurt(self.warded(DROWN_HARM, Cause::Drowning))
    }

    /// Whether, out of breath out of the water it needs (a fish), it was hurt: 2 a hit, as a cod, a
    /// salmon and a pufferfish on dry stone 15 s on BDS (research/captures/bds-harm2.txt).
    pub(crate) fn gasp(&mut self, dry: bool) -> bool {
        dry && self.air == 0 && self.hurt(SMOTHER_HARM)
    }
}
