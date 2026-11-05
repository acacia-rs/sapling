//! Which of a custom block's permutations hold for one of its states: each condition is Molang over
//! `query.block_state('<name>')`, the state's value (a boolean as 1 or 0, a string as a name).

use acacia_molang::{Compiler, Env, Host, Query, Scratch, Structs, Symbol, Value, Variables};
use acacia_proto::nbt::Value as Nbt;
use sapling_api::{BlockBox, CustomBlock};

struct State {
    values: Vec<(Symbol, Value)>,
    block_state: Query,
}

impl Host for State {
    fn query(&self, query: Query, args: &[Value], _structs: &mut Structs<'_>) -> Value {
        match args.first() {
            Some(Value::Str(asked)) if query == self.block_state => self.values.iter().find(|(name, _)| name == asked).map_or(Value::ZERO, |(_, value)| *value),
            _ => Value::ZERO,
        }
    }
}

/// Whether `condition` holds for the state `states` (its compound, as `block_states` makes it). A
/// condition that does not compile holds for none.
fn holds(condition: &str, states: &Nbt) -> bool {
    let mut compiler = Compiler::new();
    let (Ok(program), Nbt::Compound(fields)) = (compiler.compile(condition), states) else { return false };
    let mut values = Vec::new();
    for (name, value) in fields {
        let value = match value {
            Nbt::Byte(number) => Value::Num(f32::from(*number)),
            Nbt::Int(number) => Value::Num(*number as f32),
            Nbt::String(text) => Value::Str(compiler.symbol(text.as_str())),
            _ => continue,
        };
        values.push((compiler.symbol(name.as_str()), value));
    }
    let host = State { values, block_state: compiler.query("block_state") };
    program.eval(&mut Env { host: &host, variables: &mut Variables::new(), scratch: &mut Scratch::new(), this: 0.0 }).truthy()
}

/// The box `block` stops entities with in the state `states`: its own, or that of the last
/// permutation that holds and names one (BDS applies them in order, a later one over an earlier).
pub fn collision_in(block: &CustomBlock, states: &Nbt) -> Option<BlockBox> {
    let named = block.permutations.iter().filter(|permutation| permutation.collision.is_some());
    named.filter(|permutation| holds(&permutation.condition, states)).fold(block.collision, |own, permutation| permutation.collision.unwrap_or(own))
}

#[cfg(test)]
mod tests {
    use acacia_proto::nbt::{Str, Value as Nbt};
    use sapling_api::{BlockBox, CustomBlock, Permutation};

    use super::collision_in;

    #[test]
    fn a_permutation_whose_condition_holds_gives_its_box() {
        let low = BlockBox { min: [0.0; 3], max: [16.0, 8.0, 16.0] };
        let mut block = CustomBlock::new("test:lamp", "Lamp", Vec::new());
        block.permutations.push(Permutation { collision: Some(Some(low)), ..Permutation::when("q.block_state('test:lit') == true") });
        block.permutations.push(Permutation { collision: Some(None), ..Permutation::when("q.block_state('test:colour') == 'red'") });
        let state = |lit: i8, colour: &str| Nbt::Compound(vec![(Str::from("test:lit"), Nbt::Byte(lit)), (Str::from("test:colour"), Nbt::String(Str::from(colour)))]);
        assert_eq!(collision_in(&block, &state(0, "blue")), block.collision);
        assert_eq!(collision_in(&block, &state(1, "blue")), Some(low));
        assert_eq!(collision_in(&block, &state(1, "red")), None);
    }
}
