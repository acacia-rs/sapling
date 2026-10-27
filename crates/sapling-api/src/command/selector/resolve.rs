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
    /// Where `~` and a selector's distances count from for `sender`: its feet, where an `/execute`
    /// put it, or the spawn for the console.
    pub fn origin(&self, sender: &Sender) -> [f32; 3] {
        if let Sender::Executed(executed) = sender {
            return executed.position;
        }
        sender.player().and_then(|player| self.position(player.id)).map_or_else(|| self.spawn(), |position| position.feet)
    }

    /// The point `[left, up, forward]` from `sender`'s anchor along where it faces: `^ ^ ^`.
    /// A player counts from its feet along its look (BDS's `tp @s ^ ^ ^1`).
    pub(crate) fn local_point(&self, sender: &Sender, [left, up, forward]: [f32; 3]) -> [f32; 3] {
        let ([x, y, z], [yaw, pitch]) = match sender {
            Sender::Executed(executed) => {
                let [x, y, z] = executed.position;
                ([x, y + executed.anchor, z], executed.rotation)
            }
            _ => match sender.player().and_then(|player| self.position(player.id)) {
                Some(position) => (position.feet, [position.yaw, position.pitch]),
                None => (self.spawn(), [0.0, 0.0]),
            },
        };
        // Java's LocalCoordinates: forward along the look, up square to it, left their cross.
        let (yaw, pitch) = ((yaw + 90.0).to_radians(), (-pitch).to_radians());
        let lift = pitch + std::f32::consts::FRAC_PI_2;
        let ahead = [yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()];
        let above = [yaw.cos() * lift.cos(), lift.sin(), yaw.sin() * lift.cos()];
        let side = [-(ahead[1] * above[2] - ahead[2] * above[1]), -(ahead[2] * above[0] - ahead[0] * above[2]), -(ahead[0] * above[1] - ahead[1] * above[0])];
        let base = [x, y, z];
        std::array::from_fn(|axis| base[axis] + ahead[axis] * forward + above[axis] * up + side[axis] * left)
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
