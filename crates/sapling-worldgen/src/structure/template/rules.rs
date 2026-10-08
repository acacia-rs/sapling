//! BDS's structure pool block rules (`StructurePoolBlockRule`, applied in `placeInWorld` after the
//! wood remap): every rule whose input, location and position predicates pass replaces the block,
//! and later rules test the replacement (no first-match stop). Predicates draw from the placement's
//! generator: random matches only between 0 and 1 exclusive (`IRandom` vt+0x68 `f < p`, vt+0x70 `f <= p`).

use crate::block::Block;
use crate::random::Mt19937;

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Always,
    /// The block's type, any states.
    Block(String),
    /// The block's type, then `next_float() < chance`.
    RandomBlock(String, f32),
    /// Any of the types, then `next_float() <= chance`.
    RandomInList(Vec<String>, f32),
    /// None of the types (then a chance of 1, which never draws).
    NotInList(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub input: Predicate,
    pub location: Predicate,
    /// The output type's default block.
    pub output: Block,
}

impl Predicate {
    fn test(&self, block: Block, random: &mut Mt19937) -> bool {
        match self {
            Predicate::Always => true,
            Predicate::Block(name) => block.name() == name,
            Predicate::RandomBlock(name, chance) => block.name() == name && less(random, *chance),
            Predicate::RandomInList(names, chance) => names.iter().any(|n| n == block.name()) && at_most(random, *chance),
            Predicate::NotInList(names) => !names.iter().any(|n| n == block.name()),
        }
    }
}

fn less(random: &mut Mt19937, chance: f32) -> bool {
    chance > 0.0 && (chance >= 1.0 || random.next_float() < chance)
}

fn at_most(random: &mut Mt19937, chance: f32) -> bool {
    chance > 0.0 && (chance >= 1.0 || random.next_float() <= chance)
}

/// `block` after `rules`, `world` being what the world holds at its position.
pub fn apply(rules: &[Rule], mut block: Block, world: Block, random: &mut Mt19937) -> Block {
    for rule in rules {
        if rule.input.test(block, random) && rule.location.test(world, random) {
            block = rule.output;
        }
    }
    block
}
