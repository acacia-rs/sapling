//! How often BDS tells a viewer of an entity's moves (research/bds-actor-move-sync.md): by its
//! kind's `minecraft:conditional_bandwidth_optimization`, a viewer as far as its `max_optimized_distance`
//! may go `max_dropped_ticks` without one, a nearer one as much less as it is nearer. A move that
//! jumps, lands or teleports goes to every viewer at once.
// TODO: a moving boat's or minecart's `is_moving` entry (none of its moves dropped).

use bevy_ecs::prelude::Entity;

/// How far away a viewer is told least often, and how many ticks it then goes without a move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Limits {
    far: f32,
    dropped: u32,
}

/// What a kind with an empty component, or none in its pack (items), takes.
const DEFAULT: Limits = Limits { far: 80.0, dropped: 10 };

/// The kinds' own, from the 1.21 vanilla behaviour packs; those without the component at all
/// (`None`) tell every viewer of every move.
const KINDS: [(&str, Option<Limits>); 31] = [
    ("arrow", Some(Limits { far: 80.0, dropped: 7 })),
    ("breeze_wind_charge_projectile", Some(Limits { far: 80.0, dropped: 7 })),
    ("dragon_fireball", Some(Limits { far: 80.0, dropped: 7 })),
    ("egg", Some(Limits { far: 80.0, dropped: 7 })),
    ("ender_pearl", Some(Limits { far: 80.0, dropped: 7 })),
    ("fireball", Some(Limits { far: 80.0, dropped: 7 })),
    ("fishing_hook", Some(Limits { far: 80.0, dropped: 7 })),
    ("llama_spit", Some(Limits { far: 80.0, dropped: 7 })),
    ("shulker_bullet", Some(Limits { far: 80.0, dropped: 7 })),
    ("small_fireball", Some(Limits { far: 80.0, dropped: 7 })),
    ("thrown_trident", Some(Limits { far: 80.0, dropped: 7 })),
    ("wind_charge_projectile", Some(Limits { far: 80.0, dropped: 7 })),
    ("snowball", Some(Limits { far: 100.0, dropped: 7 })),
    ("lingering_potion", Some(Limits { far: 80.0, dropped: 5 })),
    ("splash_potion", Some(Limits { far: 80.0, dropped: 5 })),
    ("tnt", Some(Limits { far: 80.0, dropped: 5 })),
    ("xp_bottle", Some(Limits { far: 80.0, dropped: 5 })),
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
    ("boat", Some(Limits { far: 60.0, dropped: 20 })),
];

/// The limits of `kind`, `None` where every move goes to every viewer.
pub(crate) fn limits(kind: &str) -> Option<Limits> {
    let name = kind.strip_prefix("minecraft:").unwrap_or(kind);
    let name = if name.ends_with("minecart") || name == "chest_boat" { "boat" } else { name };
    KINDS.iter().find(|(known, _)| *known == name).map_or(Some(DEFAULT), |(_, limits)| *limits)
}

/// The ticks a viewer at `eye` may go without a move of an entity at `at`, which BDS sends in the
/// move's `ticks`.
pub(crate) fn dropped(limits: Option<Limits>, eye: [f32; 3], at: [f32; 3]) -> u64 {
    let Some(Limits { far, dropped }) = limits else { return 0 };
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
        assert_eq!([7.9, 15.9, 16.1, 68.0, 200.0].map(|far| dropped(slime, [0.0; 3], [far, 0.0, 0.0])), [0, 1, 2, 8, 10]);
        assert_eq!(dropped(limits("minecraft:arrow"), [0.0; 3], [200.0, 0.0, 0.0]), 7);
        assert_eq!(dropped(limits("minecraft:goat"), [0.0; 3], [200.0, 0.0, 0.0]), 0);
        assert_eq!(limits("minecraft:hopper_minecart"), limits("minecraft:boat"));
        let (mut paced, viewer) = (Paced::default(), bevy_ecs::world::World::new().spawn_empty().id());
        assert_eq!([10, 11, 17, 18, 19].map(|now| paced.due(viewer, now, 8)), [true, false, false, true, false]);
    }
}
