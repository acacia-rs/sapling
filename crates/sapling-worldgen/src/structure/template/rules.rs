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
    /// Any of the types in its default state, then `next_float() <= chance`.
    RandomInList(Vec<String>, f32),
    /// None of the types (then a chance of 1, which never draws).
    NotInList(Vec<String>),
}

/// An axis-aligned linear position test (vtable 14a89af30, test 145e161a0): one `next_float` once the
/// input and location pass, kept when at most Java's `clampedLerp(min, max, inverseLerp(d, min_d, max_d))`,
/// `d` the distance along `axis` (0 x, 1 y, 2 z) from the piece's pivot.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearPos {
    pub min: f32,
    pub max: f32,
    pub min_d: i32,
    pub max_d: i32,
    pub axis: usize,
}

impl LinearPos {
    fn test(&self, from_pivot: (i32, i32, i32), float: &mut dyn FnMut() -> f32) -> bool {
        let d = [from_pivot.0, from_pivot.1, from_pivot.2][self.axis].abs();
        let t = (d - self.min_d) as f32 / (self.max_d - self.min_d) as f32;
        let chance = if t < 0.0 { self.min } else if t > 1.0 { self.max } else { self.min + (self.max - self.min) * t };
        float() <= chance
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub input: Predicate,
    pub location: Predicate,
    pub position: Option<LinearPos>,
    /// The output type's default block.
    pub output: Block,
}

impl Predicate {
    fn test(&self, block: Block, float: &mut dyn FnMut() -> f32) -> bool {
        match self {
            Predicate::Always => true,
            Predicate::Block(name) => block.name() == name,
            Predicate::RandomBlock(name, chance) => block.name() == name && chance_of(*chance, || float() < *chance),
            Predicate::RandomInList(names, chance) => names.iter().any(|n| n == block.name()) && default_state(block) && chance_of(*chance, || float() <= *chance),
            Predicate::NotInList(names) => !names.iter().any(|n| n == block.name()),
        }
    }
}

/// Whether the block is its type's default state: the list holds blocks, not types (the oracle's
/// city keeps every turned stair and top slab; docs/research/worldgen-bds-jigsaw.md).
fn default_state(block: Block) -> bool {
    block.states().split(',').filter_map(|pair| pair.split_once('=')).all(|(_, value)| matches!(value, "0" | "false" | "bottom" | "none" | "y"))
}

fn chance_of(chance: f32, draw: impl FnOnce() -> bool) -> bool {
    chance > 0.0 && (chance >= 1.0 || draw())
}

/// `block` after `rules`, `world` being what the world holds at its position, `from_pivot` that
/// position less the piece's pivot.
pub fn apply(rules: &[Rule], mut block: Block, world: Block, from_pivot: (i32, i32, i32), random: &mut Mt19937) -> Block {
    let mut float = || random.next_float();
    for rule in rules {
        if rule.input.test(block, &mut float) && rule.location.test(world, &mut float) && rule.position.as_ref().is_none_or(|p| p.test(from_pivot, &mut float)) {
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
    crate::random::wide_position_seed(x, y, z)
}
