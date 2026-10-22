//! The dragon's last 64 yaws and heights (`aiStep` step 7, research/dragon-ai-decompile.md 3), which
//! bend its neck and tail: filled with the first on first use, then one a tick.

const LENGTH: usize = 64;

#[derive(Clone, Debug, Default)]
pub(crate) struct Trail {
    ring: Vec<[f32; 2]>,
    index: usize,
}

impl Trail {
    pub fn record(&mut self, yaw: f32, y: f32) {
        if self.ring.is_empty() {
            self.ring = vec![[yaw, y]; LENGTH];
        }
        self.index = (self.index + 1) % LENGTH;
        self.ring[self.index] = [yaw, y];
    }

    /// Its yaw and height `n` ticks back; now's before the first record.
    pub fn back(&self, n: usize, now: [f32; 2]) -> [f32; 2] {
        self.ring.get((self.index + LENGTH - n) % LENGTH).copied().unwrap_or(now)
    }
}
