//! A panda's gene, rolled as its pack's genetics roll it: two alleles of 0 to 15; a main allele
//! of 0 to 3 makes it lazy, worried, playful or aggressive, both 4 to 7 weak, both 8 or 9 brown,
//! and the rest normal. What each does when hit, as BDS's did (research/captures/bds-pandas-findings.md):
//! a normal or brown one hits back once, an aggressive one for 6 until it is calm, the rest run. A
//! lazy one walks at its own pace ([`pace`]), tempted too (bds-x2panda.txt: 0.0165 a tick to a normal
//! one's 0.0759), and lies down (husbandry/lying.rs), a worried one keeps away from other kinds
//! (husbandry/avoiding.rs) and a playful one rolls (husbandry/rolls.rs). An aggressive one's anger
//! reached no other: three aggressive bystanders 6 to 9 off took no target (bds-g14panda.txt), its
//! pack's `broadcast_anger` notwithstanding.
// TODO: a lazy one courting or followed by its baby is not drilled at its pace (BDS's fed pandas only ate).

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
const STANDS: u64 = 48;
/// A panda's `movement`, a lazy one's, and a lazy one's panic multiplier.
const MOVEMENT: f32 = 0.15;
const LAZY_MOVEMENT: f32 = 0.07;
const LAZY_PANIC: f32 = 2.5;

/// The gene a mob shows as its `Variant`.
#[derive(Component, Clone, Copy)]
pub(super) struct Gene(pub(super) i32);

/// Kinds whose `Variant` never changes: husk's pack sets 2, parched's BDS shows 4 (bds-h3herds.txt).
const FIXED: [(&str, i32); 2] = [("minecraft:husk", 2), ("minecraft:parched", 4)];

fn fixed(kind: &str) -> Option<Gene> {
    FIXED.iter().find(|(fixed, _)| *fixed == kind).map(|&(_, variant)| Gene(variant))
}

/// A new mob of `kind`'s gene, if its kind has one, or its size (slimes/sizes.rs).
pub(super) fn roll(world: &mut World, kind: &str) -> Option<Gene> {
    if let Some(gene) = fixed(kind) {
        return Some(gene);
    }
    if kind != KIND {
        return super::slimes::sizes::roll(world, kind);
    }
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    Some(of_alleles(dice.roll(ALLELES) as i32, dice.roll(ALLELES) as i32))
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

/// The gene a spawn `event` gives a mob of `kind`, as BDS's `/summon panda .. minecraft:panda_lazy`
/// did (research/captures/bds-pandas.txt).
pub(super) fn of_event(kind: &str, event: &str) -> Option<Gene> {
    if kind != KIND {
        return super::slimes::sizes::of_event(kind, event);
    }
    let gene = event.strip_prefix("minecraft:panda_")?;
    let names = [("lazy", MAIN[0]), ("worried", MAIN[1]), ("playful", MAIN[2]), ("aggressive", AGGRESSIVE), ("weak", WEAK), ("brown", BROWN), ("normal", NORMAL)];
    names.iter().find(|(name, _)| *name == gene).map(|&(_, variant)| Gene(variant))
}

/// The `Variant` a mob of `kind` shows with `gene`, or without one.
pub(super) fn variant(kind: &str, gene: Option<Gene>) -> i32 {
    gene.map_or_else(|| super::slimes::sizes::base_variant(kind), |gene| gene.0)
}

/// A stored mob of `kind`'s gene, by the `Variant` it was stored with.
pub(super) fn stored(kind: &str, variant: i32) -> Option<Gene> {
    if kind != KIND {
        return fixed(kind).or_else(|| super::slimes::sizes::stored(kind, variant));
    }
    Some(Gene(variant))
}

fn is_panda(world: &World, entity: Entity) -> bool {
    world.get::<crate::entities::Kind>(entity).is_some_and(|kind| kind.0 == KIND)
}

fn has_gene(world: &World, entity: Entity, gene: i32) -> bool {
    is_panda(world, entity) && world.get::<Gene>(entity).is_some_and(|kept| kept.0 == gene)
}

pub(super) fn is_worried(world: &World, entity: Entity) -> bool {
    has_gene(world, entity, MAIN[1])
}

pub(super) fn is_playful(world: &World, entity: Entity) -> bool {
    has_gene(world, entity, MAIN[2])
}

pub(super) fn is_lazy(world: &World, entity: Entity) -> bool {
    has_gene(world, entity, MAIN[0])
}

/// The share of its kind's pace the mob walks at: a lazy panda's `movement` is 0.07 to the kind's
/// 0.15 (research/captures/bds-g14panda.txt; bds-pandas-findings.md fitted its walk to it).
pub(super) fn pace(world: &World, entity: Entity) -> f32 {
    if has_gene(world, entity, MAIN[0]) { LAZY_MOVEMENT / MOVEMENT } else { 1.0 }
}

/// The mob's stroll and panic speeds, its kind's being `strolls` and `panics`: a lazy panda's at its
/// pace, its panic at its own 2.5 times it (`panda_lazy`'s `behavior.panic`; BDS's ran 0.0654 to 0.066,
/// bds-pandas-findings.md).
pub(super) fn paces(world: &World, entity: Entity, strolls: Option<f32>, panics: Option<f32>) -> (Option<f32>, Option<f32>) {
    match has_gene(world, entity, MAIN[0]) {
        true => (strolls.map(|speed| speed * LAZY_MOVEMENT / MOVEMENT), panics.map(|_| LAZY_MOVEMENT * LAZY_PANIC)),
        false => (strolls, panics),
    }
}

/// A lazy panda is announced with its own `movement`.
pub(super) fn dress(added: &mut acacia_proto::packets::AddEntity, kind: &str, gene: Option<Gene>) {
    if kind != KIND || gene.is_none_or(|gene| gene.0 != MAIN[0]) {
        return;
    }
    for attribute in added.attributes.iter_mut().filter(|attribute| attribute.name == "minecraft:movement") {
        attribute.value = LAZY_MOVEMENT;
    }
}

/// Whether a hit makes the mob turn on its hitter: every mob but a panda that runs.
pub(super) fn fights(world: &World, entity: Entity) -> bool {
    !is_panda(world, entity) || world.get::<Gene>(entity).is_none_or(|gene| [NORMAL, BROWN, AGGRESSIVE].contains(&gene.0))
}

/// How the mob hunts, by its kind's `hunts` and its gene.
pub(super) fn hunts(world: &World, entity: Entity, hunts: Hunts) -> Hunts {
    match world.get::<Gene>(entity).filter(|_| is_panda(world, entity)) {
        Some(Gene(AGGRESSIVE)) => Hunts { attack: AGGRESSIVE_ATTACK, once: false, ..hunts },
        _ => hunts,
    }
}

/// Whether a panda that took its prey stands yet: BDS's stood about 48 ticks before walking up
/// (research/captures/bds-provoke3-findings.md).
pub(super) fn stands(world: &World, entity: Entity, now: u64) -> bool {
    is_panda(world, entity) && super::hunting::quarry(world, entity).is_some_and(|(_, since)| now < since + STANDS)
}

/// The full health of a mob of `kind` and `gene`, its kind's being `max`.
pub(super) fn health(kind: &str, gene: Option<Gene>, max: f32) -> f32 {
    if let Some(health) = super::slimes::sizes::health(kind, gene) {
        return health;
    }
    if kind == KIND && gene.is_some_and(|gene| gene.0 == WEAK) { WEAK_HEALTH } else { max }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_main_allele_under_4_decides_and_else_both_must_be_weak_or_brown() {
        let genes = [(0, 15), (3, 0), (5, 7), (5, 8), (9, 8), (9, 10), (15, 15)].map(|(main, hidden)| of_alleles(main, hidden).0);
        assert_eq!(genes, [1, AGGRESSIVE, WEAK, NORMAL, BROWN, NORMAL, NORMAL]);
    }

    #[test]
    fn a_spawn_event_names_bds_s_variant_of_its_gene() {
        let genes = ["lazy", "worried", "playful", "aggressive", "weak"].map(|gene| of_event(KIND, &format!("minecraft:panda_{gene}")).map(|gene| gene.0));
        assert_eq!(genes, [1, 2, 3, 6, 5].map(Some));
        assert!(of_event("minecraft:cow", "minecraft:panda_lazy").is_none() && of_event(KIND, "minecraft:entity_born").is_none());
    }

    #[test]
    fn a_husk_and_a_parched_keep_bds_s_variant_stored_or_not() {
        assert_eq!(["minecraft:husk", "minecraft:parched"].map(|kind| stored(kind, 0).map(|gene| gene.0)), [Some(2), Some(4)]);
        assert!(stored("minecraft:zombie", 0).is_none());
    }
}
