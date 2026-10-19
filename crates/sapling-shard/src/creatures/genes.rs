//! A panda's gene, rolled as its pack's genetics roll it: two alleles of 0 to 15; a main allele
//! of 0 to 3 makes it lazy, worried, playful or aggressive, both 4 to 7 weak, both 8 or 9 brown,
//! and the rest normal. What each does when hit, as BDS's did (research/captures/bds-pandas-findings.md):
//! a normal or brown one hits back once, an aggressive one for 6 until it is calm, the rest run.
// TODO: a lazy one walks and runs at its own slow pace, a worried one keeps away from other kinds,
// a playful one rolls, and an aggressive one's anger is not passed to others of its gene.

use bevy_ecs::prelude::*;
use sapling_data::Hunts;

use crate::blocks::Dice;

const KIND: &str = "minecraft:panda";
const ALLELES: u32 = 16;
/// Each gene's `Variant`, and the main alleles that make the first four.
const NORMAL: i32 = 0;
const BROWN: i32 = 4;
const WEAK: i32 = 5;
const AGGRESSIVE: i32 = 6;
const MAIN: [i32; 4] = [1, 2, 3, AGGRESSIVE];
/// A weak one's health, and an aggressive one's hit.
const WEAK_HEALTH: f32 = 10.0;
const AGGRESSIVE_ATTACK: f32 = 6.0;

/// The gene a mob shows as its `Variant`.
#[derive(Component, Clone, Copy)]
pub(super) struct Gene(pub(super) i32);

/// A new mob of `kind`'s gene, if its kind has one.
pub(super) fn roll(world: &mut World, kind: &str) -> Option<Gene> {
    (kind == KIND).then(|| {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        of_alleles(dice.roll(ALLELES) as i32, dice.roll(ALLELES) as i32)
    })
}

fn of_alleles(main: i32, hidden: i32) -> Gene {
    let both = |low: i32, high: i32| (low..=high).contains(&main) && (low..=high).contains(&hidden);
    Gene(match main {
        0..=3 => MAIN[main as usize],
        _ if both(4, 7) => WEAK,
        _ if both(8, 9) => BROWN,
        _ => NORMAL,
    })
}

/// A stored mob of `kind`'s gene, by the `Variant` it was stored with.
pub(super) fn stored(kind: &str, variant: i32) -> Option<Gene> {
    (kind == KIND).then_some(Gene(variant))
}

/// Whether a hit makes the mob turn on its hitter: every mob but a panda that runs.
pub(super) fn fights(world: &World, entity: Entity) -> bool {
    world.get::<Gene>(entity).is_none_or(|gene| [NORMAL, BROWN, AGGRESSIVE].contains(&gene.0))
}

/// How the mob hunts, by its kind's `hunts` and its gene.
pub(super) fn hunts(world: &World, entity: Entity, hunts: Hunts) -> Hunts {
    match world.get::<Gene>(entity) {
        Some(Gene(AGGRESSIVE)) => Hunts { attack: AGGRESSIVE_ATTACK, once: false, ..hunts },
        _ => hunts,
    }
}

/// The full health of a mob of `gene`, its kind's being `max`.
pub(super) fn health(gene: Option<Gene>, max: f32) -> f32 {
    if gene.is_some_and(|gene| gene.0 == WEAK) { WEAK_HEALTH } else { max }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_main_allele_under_4_decides_and_else_both_must_be_weak_or_brown() {
        let genes = [(0, 15), (3, 0), (5, 7), (5, 8), (9, 8), (9, 10), (15, 15)].map(|(main, hidden)| of_alleles(main, hidden).0);
        assert_eq!(genes, [1, AGGRESSIVE, WEAK, NORMAL, BROWN, NORMAL, NORMAL]);
    }
}
