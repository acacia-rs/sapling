//! Torches, repeaters and comparators: what an evaluation flips by the power the search brought it,
//! for dust and one another to see at the next (docs/research/redstone.md, C10, "Repeaters" and
//! "Comparators").

use sapling_api::BlockPos;

/// Evaluations a torch powering itself flips for before it stays out (research R20).
const BURNOUT: u8 = 16;
/// Ticks a burnt torch stays out whatever changes beside it: BDS counts 17 more evaluations.
const REIGNITE: u64 = 34;

#[derive(Default)]
pub(super) struct TorchState {
    /// Evaluations in a row that found it powering itself.
    count: u8,
    /// Whether its own power came back to it when it was last lit.
    looped: bool,
    burnt_at: u64,
}

impl TorchState {
    /// Lets a burnt torch light again, when a block beside it changed long enough after.
    pub(super) fn reignite(&mut self, now: u64) {
        if self.count >= BURNOUT && now >= self.burnt_at + REIGNITE {
            self.count = 0;
        }
    }

    /// Whether the torch at `pos` is lit after this evaluation, and whether it fizzes going out:
    /// `input` is the power into its block and the source of it.
    pub(super) fn next(&mut self, pos: BlockPos, lit: bool, input: Option<(u8, BlockPos)>, now: u64) -> (bool, bool) {
        let (input, origin) = input.unwrap_or((0, pos));
        self.looped = if lit { input > 0 && origin == pos } else { self.looped && input == 0 };
        if !self.looped {
            self.count = 0;
            return (input == 0, false);
        }
        self.count = self.count.saturating_add(1);
        if self.count == BURNOUT {
            self.burnt_at = now;
        }
        (self.count < BURNOUT && !lit, self.count >= BURNOUT)
    }
}

#[derive(Default)]
pub(super) struct RepeaterState {
    /// Evaluations until it flips; 0 when it waits for none.
    countdown: u8,
}

impl RepeaterState {
    pub(super) fn is_waiting(&self) -> bool {
        self.countdown > 0
    }

    /// Whether the repeater is lit after this evaluation. It flips `delay` evaluations after its
    /// input first differs from it, whatever the input does meanwhile; an input that differs
    /// again as it flips is followed `delay + 1` later, so a fast clock comes out slowed.
    pub(super) fn next(&mut self, lit: bool, fed: bool, delay: u8, locked: bool) -> bool {
        if locked {
            self.countdown = 0;
            return lit;
        }
        match self.countdown {
            0 if fed == lit => lit,
            0 if delay == 0 => fed,
            0 => {
                self.countdown = delay;
                lit
            }
            1 => {
                self.countdown = if fed == lit { delay + 1 } else { 0 };
                !lit
            }
            _ => {
                self.countdown -= 1;
                lit
            }
        }
    }
}

/// What a comparator gives out: what is behind it unless a side is stronger, or, subtracting,
/// what is behind it less its stronger side.
pub(super) fn compared(behind: u8, side: u8, subtracts: bool) -> u8 {
    match subtracts {
        true => behind.saturating_sub(side),
        false if side > behind => 0,
        false => behind,
    }
}

/// The signal a comparator reads off a container whose slots hold `filled` (a count and the most
/// the slot takes of it, for each stack): 0 when empty, else 1 and 14 by how full it is.
pub(super) fn fullness(filled: impl Iterator<Item = (u8, u8)>, slots: usize) -> u8 {
    let parts: Vec<f32> = filled.map(|(count, most)| f32::from(count) / f32::from(most.max(1))).collect();
    if parts.is_empty() || slots == 0 {
        return 0;
    }
    (1.0 + parts.iter().sum::<f32>() / slots as f32 * 14.0).floor() as u8
}

/// The signal a comparator reads off a block by its state alone (research/captures/bds-redstone-reads.txt).
pub(super) fn state_signal(name: &str, properties: &str) -> Option<u8> {
    let value = |key: &str| properties.split(',').find_map(|pair| pair.strip_prefix(key)?.strip_prefix('=')?.parse::<u8>().ok());
    match name.strip_prefix("minecraft:").unwrap_or(name) {
        "cake" => Some(14 - 2 * value("bite_counter")?.min(7)),
        "cauldron" => Some(value("fill_level")?.div_ceil(2)),
        "composter" => value("composter_fill_level"),
        "respawn_anchor" => Some((4 * value("respawn_anchor_charge")?).saturating_sub(1)),
        "beehive" | "bee_nest" => value("honey_level"),
        "end_portal_frame" => Some(15 * value("end_portal_eye_bit")?.min(1)),
        bulb if bulb.ends_with("copper_bulb") => Some(15 * value("lit")?.min(1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comparator_reads_a_chest_and_compares_or_subtracts_as_on_bds() {
        // research/captures/bds-redstone-comparators.txt: a chest filled a stack at a time; 11 behind, 13 or 9 at a side.
        let read: Vec<u8> = (0..=27).map(|stacks| fullness((0..stacks).map(|_| (64, 64)), 27)).collect();
        assert_eq!(read, [0, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 15]);
        assert_eq!([compared(11, 13, false), compared(11, 9, false), compared(11, 13, true), compared(11, 9, true)], [0, 11, 0, 2]);
    }

    #[test]
    fn a_comparator_reads_cake_cauldron_composter_anchor_hive_and_portal_frame_as_on_bds() {
        let read = |name: &str, key: &str, values: std::ops::RangeInclusive<u8>| values.map(|value| state_signal(name, &format!("other=3,{key}={value}")).unwrap()).collect::<Vec<_>>();
        assert_eq!(read("minecraft:cake", "bite_counter", 0..=6), [14, 12, 10, 8, 6, 4, 2]);
        assert_eq!(read("cauldron", "fill_level", 0..=6), [0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(read("composter", "composter_fill_level", 0..=8), (0..=8).collect::<Vec<_>>());
        assert_eq!(read("respawn_anchor", "respawn_anchor_charge", 0..=4), [0, 3, 7, 11, 15]);
        assert_eq!(read("beehive", "honey_level", 0..=5), (0..=5).collect::<Vec<_>>());
        assert_eq!((read("end_portal_frame", "end_portal_eye_bit", 0..=1), state_signal("stone", "")), (vec![0, 15], None));
    }

    /// The evaluations at which a repeater flips, fed a clock that is on at every even one.
    fn flips(delay: u8, evaluations: usize) -> Vec<usize> {
        let (mut state, mut lit, mut flips) = (RepeaterState::default(), false, Vec::new());
        for at in 0..evaluations {
            let next = state.next(lit, at % 2 == 0 && at < 15, delay, false);
            if next != lit {
                flips.push(at);
            }
            lit = next;
        }
        flips
    }

    #[test]
    fn a_repeater_slows_a_clock_faster_than_its_delay_as_on_bds() {
        // research/captures/bds-redstone-repeaters.txt, the rigs at y 5: a torch flipping at every evaluation.
        assert_eq!(flips(0, 18), (0..16).collect::<Vec<_>>());
        assert_eq!(flips(1, 18), [1, 3, 5, 7, 9, 11, 13, 15]);
        assert_eq!(flips(2, 20), [2, 5, 8, 11, 14, 17]);
        assert_eq!(flips(3, 20), [3, 7, 11, 15]);
    }
}
