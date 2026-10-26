//! Who a selector names: the candidates its base takes, filtered, put in order and counted.

use std::hash::{BuildHasher, RandomState};

use super::matching::{Candidate, passes};
use super::{Base, Filters, Selector, gather};
use crate::{EntityInfo, Player, Sender, ShardCtx};

/// A player or an entity a selector names.
#[derive(Debug, Clone, PartialEq)]
pub enum Selected {
    Player(Player),
    Entity(EntityInfo),
}

impl ShardCtx<'_> {
    /// Where `~` and a selector's distances count from for `sender`: its feet, or the spawn for
    /// the console.
    pub fn origin(&self, sender: &Sender) -> [f32; 3] {
        sender.player().and_then(|player| self.position(player.id)).map_or_else(|| self.spawn(), |position| position.feet)
    }

    /// The players and entities `selector` names for `sender`, in vanilla's order: the nearest
    /// first for `@p`, `@n` and any `c`, at random for `@r`, players then entities, oldest first,
    /// otherwise. A name is the online player of that name.
    pub fn select(&mut self, sender: &Sender, selector: &Selector) -> Vec<Selected> {
        let here = self.origin(sender);
        self.select_from(sender, selector, here)
    }

    /// [`ShardCtx::select`] with distances counted from `here` (an `/execute at`'s place).
    pub fn select_from(&mut self, sender: &Sender, selector: &Selector, here: [f32; 3]) -> Vec<Selected> {
        let (base, filters) = match selector {
            Selector::Named(name) => return self.players().filter(|player| player.name.eq_ignore_ascii_case(name)).map(Selected::Player).collect(),
            Selector::Variable(base, filters) => (*base, filters),
        };
        let origin = std::array::from_fn(|axis| filters.origin[axis].map_or(here[axis], |at| at.from(here[axis])));
        let candidates = gather::candidates(self, sender, base, filters);
        let seed = RandomState::new().hash_one(self.tick());
        pick(base, filters, origin, candidates, seed).into_iter().map(|candidate| candidate.target).collect()
    }
}

/// Those of `candidates` that `base` and `filters` name, in order; `seed` shuffles `@r`.
pub(super) fn pick(base: Base, filters: &Filters, origin: [f32; 3], candidates: Vec<Candidate>, seed: u64) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = candidates.into_iter().filter(|candidate| passes(filters, candidate, origin)).collect();
    let one = matches!(base, Base::NearestPlayer | Base::NearestEntity | Base::RandomPlayer).then_some(1);
    let limit = filters.count.or(one);
    match base {
        Base::RandomPlayer => shuffle(&mut found, seed),
        _ if limit.is_some() => found.sort_by(|a, b| a.distance(origin).total_cmp(&b.distance(origin))),
        _ => {}
    }
    if let Some(limit) = limit {
        if limit < 0 && base != Base::RandomPlayer {
            found.reverse();
        }
        found.truncate(limit.unsigned_abs() as usize);
    }
    found
}

/// Fisher-Yates, drawn from splitmix64.
fn shuffle<T>(items: &mut [T], mut seed: u64) {
    let mut next = || {
        seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    for i in (1..items.len()).rev() {
        items.swap(i, (next() % (i as u64 + 1)) as usize);
    }
}
