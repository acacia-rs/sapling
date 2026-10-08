//! Structure block rules. BDS's hardcoded pools (`StructurePoolBlockRule`, applied in
//! `placeInWorld` after the wood remap): every rule whose predicates pass replaces the block and
//! later rules test the replacement (no first-match stop), drawing from the placement's generator,
//! random matches only between 0 and 1 exclusive (`IRandom` vt+0x68 `f < p`, vt+0x70 `f <= p`).
//! The v2 rule processor (145944ea0) is Java's: the first rule that passes wins, drawing from
//! a `java.util.Random` per block (`rule_seed`).

use crate::block::Block;
use crate::random::{JavaRandom, Mt19937};

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
    fn test(&self, block: Block, float: &mut dyn FnMut() -> f32) -> bool {
        match self {
            Predicate::Always => true,
            Predicate::Block(name) => block.name() == name,
            Predicate::RandomBlock(name, chance) => block.name() == name && chance_of(*chance, || float() < *chance),
            Predicate::RandomInList(names, chance) => names.iter().any(|n| n == block.name()) && chance_of(*chance, || float() <= *chance),
            Predicate::NotInList(names) => !names.iter().any(|n| n == block.name()),
        }
    }
}

fn chance_of(chance: f32, draw: impl FnOnce() -> bool) -> bool {
    chance > 0.0 && (chance >= 1.0 || draw())
}

/// `block` after `rules`, `world` being what the world holds at its position.
pub fn apply(rules: &[Rule], mut block: Block, world: Block, random: &mut Mt19937) -> Block {
    let mut float = || random.next_float();
    for rule in rules {
        if rule.input.test(block, &mut float) && rule.location.test(world, &mut float) {
            block = rule.output;
        }
    }
    block
}

/// The v2 rule processor's output for `block` at `at`, if a rule passes.
pub fn apply_first(rules: &[Rule], block: Block, world: Block, at: (i32, i32, i32)) -> Option<Block> {
    let mut random = JavaRandom::new(rule_seed(at));
    let mut float = || random.next_float();
    rules.iter().find(|r| r.input.test(block, &mut float) && r.location.test(world, &mut float)).map(|r| r.output)
}

/// Java's `Mth.getSeed`, but with x multiplied in 64 bits (145944ea0).
fn rule_seed((x, y, z): (i32, i32, i32)) -> i64 {
    let seed = (x as i64).wrapping_mul(3_129_871) ^ (z as i64).wrapping_mul(116_129_781) ^ y as i64;
    seed.wrapping_mul(seed).wrapping_mul(42_317_861).wrapping_add(seed.wrapping_mul(11)) >> 16
}
