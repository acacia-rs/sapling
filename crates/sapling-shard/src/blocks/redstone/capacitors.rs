//! Torches and repeaters: what an evaluation flips by the power the search brought it, for dust and
//! one another to see at the next (docs/research/redstone.md, C10 and "Repeaters").

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

#[cfg(test)]
mod tests {
    use super::*;

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
