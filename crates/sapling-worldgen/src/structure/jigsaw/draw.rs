//! How the jigsaw placers shuffle: BDS's own way on MT19937 for the hardcoded structures, Java's
//! on `java.util.Random` for the v2 ones.

use crate::random::{JavaRandom, Mt19937};

pub trait JigsawRandom {
    fn shuffle<T>(&mut self, items: &mut [T]);
}

impl JigsawRandom for Mt19937 {
    /// 141ee53d0, 145e23dc0: for i from 1, swap i with `u32 % (i + 1)` when they differ.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in 1..items.len() {
            let j = (self.next_u32() % (i as u32 + 1)) as usize;
            if j != i {
                items.swap(i, j);
            }
        }
    }
}

impl JigsawRandom for JavaRandom {
    /// Inlined in 144cd3360 and 144d11120.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        JavaRandom::shuffle(self, items);
    }
}
