//! How often BDS tells a viewer of an entity's moves (research/bds-actor-move-sync.md): by its
//! kind's `minecraft:conditional_bandwidth_optimization`, a viewer as far as its `max_optimized_distance`
//! may go `max_dropped_ticks` without one, a nearer one as much less as it is nearer. A move that
//! jumps, lands or teleports goes to every viewer at once.

use bevy_ecs::prelude::Entity;

/// How far away a viewer is told least often, and how many ticks it then goes without a move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Limits {
    far: f32,
    dropped: u32,
    /// Only while it stands still (a boat's or minecart's `is_moving` entry): moving, every move goes.
    still: bool,
}

const fn paced(far: f32, dropped: u32) -> Option<Limits> {
    Some(Limits { far, dropped, still: false })
}

/// What a kind with an empty component, or none in its pack (items), takes.
const DEFAULT: Option<Limits> = paced(80.0, 10);

/// The kinds' own, from the 1.21 vanilla behaviour packs; those without the component at all
/// (`None`) tell every viewer of every move.
const KINDS: [(&str, Option<Limits>); 31] = [
    ("arrow", paced(80.0, 7)),
    ("breeze_wind_charge_projectile", paced(80.0, 7)),
    ("dragon_fireball", paced(80.0, 7)),
    ("egg", paced(80.0, 7)),
    ("ender_pearl", paced(80.0, 7)),
    ("fireball", paced(80.0, 7)),
    ("fishing_hook", paced(80.0, 7)),
    ("llama_spit", paced(80.0, 7)),
    ("shulker_bullet", paced(80.0, 7)),
    ("small_fireball", paced(80.0, 7)),
    ("thrown_trident", paced(80.0, 7)),
    ("wind_charge_projectile", paced(80.0, 7)),
    ("snowball", paced(100.0, 7)),
    ("lingering_potion", paced(80.0, 5)),
    ("splash_potion", paced(80.0, 5)),
    ("tnt", paced(80.0, 5)),
    ("xp_bottle", paced(80.0, 5)),
    ("axolotl", None),
    ("creaking", None),
    ("ender_dragon", None),
    ("falling_block", None),
    ("frog", None),
    ("glow_squid", None),
    ("goat", None),
    ("nautilus", None),
    ("npc", None),
    ("ominous_item_spawner", None),
    ("tadpole", None),
    ("warden", None),
    ("zombie_nautilus", None),
    ("boat", Some(Limits { far: 60.0, dropped: 20, still: true })),
];

/// The limits of `kind`, `None` where every move goes to every viewer.
pub(crate) fn limits(kind: &str) -> Option<Limits> {
    let name = kind.strip_prefix("minecraft:").unwrap_or(kind);
    let name = if name.ends_with("minecart") || name == "chest_boat" { "boat" } else { name };
    KINDS.iter().find(|(known, _)| *known == name).map_or(DEFAULT, |(_, limits)| *limits)
}

/// The ticks a viewer at `eye` may go without a move of an entity at `at`, `moving` or not, which
/// BDS sends in the move's `ticks`.
pub(crate) fn dropped(limits: Option<Limits>, eye: [f32; 3], at: [f32; 3], moving: bool) -> u64 {
    let Some(Limits { far, dropped, .. }) = limits.filter(|limits| !(limits.still && moving)) else { return 0 };
    let distance = (0..3).map(|axis| (eye[axis] - at[axis]).powi(2)).sum::<f32>().sqrt();
    (dropped as f32 * (distance / far).min(1.0)) as u64
}

/// When each viewer was last told of a move.
#[derive(Clone, Debug, Default)]
pub(crate) struct Paced(Vec<(Entity, u64)>);

impl Paced {
    /// Whether `viewer` is due a move at `now`, `dropped` ticks after its last; it is then told.
    pub(crate) fn due(&mut self, viewer: Entity, now: u64, dropped: u64) -> bool {
        let at = self.0.iter().position(|(known, _)| *known == viewer);
        if at.is_some_and(|at| now < self.0[at].1 + dropped) {
            return false;
        }
        match at {
            Some(at) => self.0[at].1 = now,
            None => self.0.push((viewer, now)),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_far_viewer_goes_ticks_without_a_move_by_its_distance_as_on_bds() {
        // bds-kinds2.txt: the drowned's watcher went from 1 to 2 as it passed 16 blocks; the enderman, past 80, 10.
        let slime = limits("minecraft:slime");
        assert_eq!([7.9, 15.9, 16.1, 68.0, 200.0].map(|far| dropped(slime, [0.0; 3], [far, 0.0, 0.0], true)), [0, 1, 2, 8, 10]);
        assert_eq!(dropped(limits("minecraft:arrow"), [0.0; 3], [200.0, 0.0, 0.0], true), 7);
        assert_eq!(dropped(limits("minecraft:goat"), [0.0; 3], [200.0, 0.0, 0.0], true), 0);
        let cart = limits("minecraft:hopper_minecart");
        assert_eq!(cart, limits("minecraft:boat"));
        assert_eq!([true, false].map(|moving| dropped(cart, [0.0; 3], [30.0, 0.0, 0.0], moving)), [0, 10]);
        let (mut paced, viewer) = (Paced::default(), bevy_ecs::world::World::new().spawn_empty().id());
        assert_eq!([10, 11, 17, 18, 19].map(|now| paced.due(viewer, now, 8)), [true, false, false, true, false]);
    }
}
