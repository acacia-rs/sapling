//! `addInlandBiomes`: the weirdness slices (mid 1481f9850, high 1482009c0, peaks 148206880, low
//! 14820bae0, valleys inline), each looping temperature then humidity, and the biome pickers.

use super::tables::*;
use super::{Builder, Range, join};

fn variant(table: &[[u32; 5]; 5], t: usize, h: usize) -> Option<u32> {
    Some(table[t][h]).filter(|&b| b != 0)
}

fn middle(t: usize, h: usize, w: Range) -> u32 {
    if w.1 < 0 { MIDDLE[t][h] } else { variant(&MIDDLE_VARIANT, t, h).unwrap_or(MIDDLE[t][h]) }
}

fn badlands(h: usize, w: Range) -> u32 {
    if h < 2 {
        if w.1 < 0 { BADLANDS } else { ERODED_BADLANDS }
    } else if h < 3 {
        BADLANDS
    } else {
        WOODED_BADLANDS
    }
}

fn middle_or_badlands(t: usize, h: usize, w: Range) -> u32 {
    if t == 4 { badlands(h, w) } else { middle(t, h, w) }
}

fn middle_or_badlands_or_slope(t: usize, h: usize, w: Range) -> u32 {
    if t == 0 { slope(t, h, w) } else { middle_or_badlands(t, h, w) }
}

fn windswept_savanna_or(t: usize, h: usize, w: Range, underlying: u32) -> u32 {
    if t > 1 && h < 4 && w.1 >= 0 { WINDSWEPT_SAVANNA } else { underlying }
}

fn beach(t: usize) -> u32 {
    match t {
        0 => SNOWY_BEACH,
        4 => DESERT,
        _ => BEACH,
    }
}

fn shattered_coast(t: usize, h: usize, w: Range) -> u32 {
    let underlying = if w.1 >= 0 { middle(t, h, w) } else { beach(t) };
    windswept_savanna_or(t, h, w, underlying)
}

fn plateau(t: usize, h: usize, w: Range) -> u32 {
    let variant = if w.1 >= 0 { variant(&PLATEAU_VARIANT, t, h) } else { None };
    variant.unwrap_or(PLATEAU[t][h])
}

fn peak(t: usize, h: usize, w: Range) -> u32 {
    match t {
        0..=2 => {
            if w.1 < 0 { JAGGED_PEAKS } else { FROZEN_PEAKS }
        }
        3 => STONY_PEAKS,
        _ => badlands(h, w),
    }
}

fn slope(t: usize, h: usize, w: Range) -> u32 {
    if t >= 3 {
        plateau(t, h, w)
    } else if h <= 1 {
        SNOWY_SLOPES
    } else {
        GROVE
    }
}

fn shattered(t: usize, h: usize, w: Range) -> u32 {
    variant(&SHATTERED, t, h).unwrap_or_else(|| middle(t, h, w))
}

impl Builder {
    pub(super) fn inland(&mut self) {
        for (w, slice) in weirdness_slices() {
            match slice {
                Slice::Mid => self.mid_slice(w),
                Slice::High => self.high_slice(w),
                Slice::Peaks => self.peaks(w),
                Slice::Low => self.low_slice(w),
                Slice::Valleys => self.valleys(w),
            }
        }
    }

    /// The swamps and stony shores every mid and low slice starts with.
    fn shores_and_swamps(&mut self, w: Range) {
        let (full, e, c, t) = (self.full, self.e, &self.c, self.t);
        let (coast, inland) = (c.coast, join(c.near, c.far));
        self.surface(full, full, coast, join(e[0], e[2]), w, STONY_SHORE);
        self.surface(join(t[1], t[2]), full, inland, e[6], w, SWAMP);
        self.surface(join(t[3], t[4]), full, inland, e[6], w, MANGROVE_SWAMP);
    }

    fn peaks(&mut self, w: Range) {
        let (e, c) = (self.e, &self.c);
        let (coast_far, coast_near, mid_far) = (join(c.coast, c.far), join(c.coast, c.near), join(c.mid, c.far));
        let (mid, far) = (c.mid, c.far);
        for ti in 0..5 {
            for hi in 0..5 {
                let (t, h) = (self.t[ti], self.h[hi]);
                let m = middle(ti, hi, w);
                let mb = middle_or_badlands(ti, hi, w);
                let mbs = middle_or_badlands_or_slope(ti, hi, w);
                let p = plateau(ti, hi, w);
                let s = shattered(ti, hi, w);
                let ws = windswept_savanna_or(ti, hi, w, s);
                let pk = peak(ti, hi, w);
                self.surface(t, h, coast_far, e[0], w, pk);
                self.surface(t, h, coast_near, e[1], w, mbs);
                self.surface(t, h, mid_far, e[1], w, pk);
                self.surface(t, h, coast_near, join(e[2], e[3]), w, m);
                self.surface(t, h, mid_far, e[2], w, p);
                self.surface(t, h, mid, e[3], w, mb);
                self.surface(t, h, far, e[3], w, p);
                self.surface(t, h, coast_far, e[4], w, m);
                self.surface(t, h, coast_near, e[5], w, ws);
                self.surface(t, h, mid_far, e[5], w, s);
                self.surface(t, h, coast_far, e[6], w, m);
            }
        }
    }

    fn high_slice(&mut self, w: Range) {
        let (e, c) = (self.e, &self.c);
        let (coast, near, mid, far) = (c.coast, c.near, c.mid, c.far);
        let (coast_far, coast_near, mid_far) = (join(coast, far), join(coast, near), join(mid, far));
        for ti in 0..5 {
            for hi in 0..5 {
                let (t, h) = (self.t[ti], self.h[hi]);
                let m = middle(ti, hi, w);
                let mb = middle_or_badlands(ti, hi, w);
                let mbs = middle_or_badlands_or_slope(ti, hi, w);
                let p = plateau(ti, hi, w);
                let s = shattered(ti, hi, w);
                let ws = windswept_savanna_or(ti, hi, w, m);
                let sl = slope(ti, hi, w);
                let pk = peak(ti, hi, w);
                self.surface(t, h, coast, join(e[0], e[1]), w, m);
                self.surface(t, h, near, e[0], w, sl);
                self.surface(t, h, mid_far, e[0], w, pk);
                self.surface(t, h, near, e[1], w, mbs);
                self.surface(t, h, mid_far, e[1], w, sl);
                self.surface(t, h, coast_near, join(e[2], e[3]), w, m);
                self.surface(t, h, mid_far, e[2], w, p);
                self.surface(t, h, mid, e[3], w, mb);
                self.surface(t, h, far, e[3], w, p);
                self.surface(t, h, coast_far, e[4], w, m);
                self.surface(t, h, coast_near, e[5], w, ws);
                self.surface(t, h, mid_far, e[5], w, s);
                self.surface(t, h, coast_far, e[6], w, m);
            }
        }
    }

    fn mid_slice(&mut self, w: Range) {
        self.shores_and_swamps(w);
        let (e, c) = (self.e, &self.c);
        let (coast, near, mid, far) = (c.coast, c.near, c.mid, c.far);
        let (coast_far, coast_near, mid_far, near_far, near_mid) = (join(coast, far), join(coast, near), join(mid, far), join(near, far), join(near, mid));
        for ti in 0..5 {
            for hi in 0..5 {
                let (t, h) = (self.t[ti], self.h[hi]);
                let m = middle(ti, hi, w);
                let mb = middle_or_badlands(ti, hi, w);
                let mbs = middle_or_badlands_or_slope(ti, hi, w);
                let s = shattered(ti, hi, w);
                let p = plateau(ti, hi, w);
                let b = beach(ti);
                let ws = windswept_savanna_or(ti, hi, w, m);
                let sc = shattered_coast(ti, hi, w);
                let sl = slope(ti, hi, w);
                self.surface(t, h, near_far, e[0], w, sl);
                self.surface(t, h, near_mid, e[1], w, mbs);
                self.surface(t, h, far, e[1], w, if ti == 0 { sl } else { p });
                self.surface(t, h, near, e[2], w, m);
                self.surface(t, h, mid, e[2], w, mb);
                self.surface(t, h, far, e[2], w, p);
                self.surface(t, h, coast_near, e[3], w, m);
                self.surface(t, h, mid_far, e[3], w, mb);
                if w.1 < 0 {
                    self.surface(t, h, coast, e[4], w, b);
                    self.surface(t, h, near_far, e[4], w, m);
                } else {
                    self.surface(t, h, coast_far, e[4], w, m);
                }
                self.surface(t, h, coast, e[5], w, sc);
                self.surface(t, h, near, e[5], w, ws);
                self.surface(t, h, mid_far, e[5], w, s);
                self.surface(t, h, coast, e[6], w, if w.1 < 0 { b } else { m });
                if ti == 0 {
                    self.surface(t, h, near_far, e[6], w, m);
                }
            }
        }
    }

    fn low_slice(&mut self, w: Range) {
        self.shores_and_swamps(w);
        let (e, c) = (self.e, &self.c);
        let (coast, near, mid, far) = (c.coast, c.near, c.mid, c.far);
        let (mid_far, near_far) = (join(mid, far), join(near, far));
        for ti in 0..5 {
            for hi in 0..5 {
                let (t, h) = (self.t[ti], self.h[hi]);
                let m = middle(ti, hi, w);
                let mb = middle_or_badlands(ti, hi, w);
                let mbs = middle_or_badlands_or_slope(ti, hi, w);
                let b = beach(ti);
                let ws = windswept_savanna_or(ti, hi, w, m);
                let sc = shattered_coast(ti, hi, w);
                self.surface(t, h, near, join(e[0], e[1]), w, mb);
                self.surface(t, h, mid_far, join(e[0], e[1]), w, mbs);
                self.surface(t, h, near, join(e[2], e[3]), w, m);
                self.surface(t, h, mid_far, join(e[2], e[3]), w, mb);
                self.surface(t, h, coast, join(e[3], e[4]), w, b);
                self.surface(t, h, near_far, e[4], w, m);
                self.surface(t, h, coast, e[5], w, sc);
                self.surface(t, h, near, e[5], w, ws);
                self.surface(t, h, mid_far, e[5], w, m);
                self.surface(t, h, coast, e[6], w, b);
                if ti == 0 {
                    self.surface(t, h, near_far, e[6], w, m);
                }
            }
        }
    }

    fn valleys(&mut self, w: Range) {
        let (full, e, c, t) = (self.full, self.e, &self.c, self.t);
        let (coast, near, mid, far, inland) = (c.coast, c.near, c.mid, c.far, c.inland);
        let shore = |river: u32| if w.1 < 0 { STONY_SHORE } else { river };
        self.surface(frozen(), full, coast, join(e[0], e[1]), w, shore(FROZEN_RIVER));
        self.surface(unfrozen(), full, coast, join(e[0], e[1]), w, shore(RIVER));
        self.surface(frozen(), full, near, join(e[0], e[1]), w, FROZEN_RIVER);
        self.surface(unfrozen(), full, near, join(e[0], e[1]), w, RIVER);
        self.surface(frozen(), full, join(coast, far), join(e[2], e[5]), w, FROZEN_RIVER);
        self.surface(unfrozen(), full, join(coast, far), join(e[2], e[5]), w, RIVER);
        self.surface(frozen(), full, coast, e[6], w, FROZEN_RIVER);
        self.surface(unfrozen(), full, coast, e[6], w, RIVER);
        self.surface(join(t[1], t[2]), full, join(inland, far), e[6], w, SWAMP);
        self.surface(join(t[3], t[4]), full, join(inland, far), e[6], w, MANGROVE_SWAMP);
        self.surface(frozen(), full, join(inland, far), e[6], w, FROZEN_RIVER);
        for ti in 0..5 {
            for hi in 0..5 {
                self.surface(self.t[ti], self.h[hi], join(mid, far), join(e[0], e[1]), w, middle_or_badlands(ti, hi, w));
            }
        }
    }
}
