//! The Molang fields of feature data, compiled once per expression and evaluated with the scatter's
//! `variable.origin{x,y,z}` and `variable.world{x,y,z}`. `math.random` draws `nextFloat` from the
//! generator `ScatterParams::initMolangParams` captured: the chunk's own one for rules, the placing
//! one inside `scatter_feature`.

use std::cell::RefCell;
use std::collections::HashMap;

use acacia_molang::{Compiler, Env, Host, Program, Query, Scratch, Structs, Value, Variable, Variables};

use super::spec::{Axis, Expr};
use super::view::View;
use crate::noise::SimplexNoise;
use crate::random::Mt19937;

/// The world-generation queries a feature field may ask, answered from a source chunk's view.
pub struct World<'a, 'b> {
    pub view: &'a View<'b>,
    pub queries: Queries,
}

#[derive(Clone, Copy)]
pub struct Queries {
    above_top_solid: Query,
    heightmap: Query,
    noise: Query,
}

/// `query.noise` (lambda 1411a24c0) and the legacy flower features' noise: the biome info noise.
pub fn noise() -> &'static SimplexNoise {
    crate::surface::info_noise()
}

impl Host for World<'_, '_> {
    fn query(&self, query: Query, args: &[Value], _structs: &mut Structs<'_>) -> Value {
        let num = |i: usize| args.get(i).map_or(0.0, |v| v.num());
        let arg = |i: usize| num(i).round() as i32;
        if query == self.queries.above_top_solid {
            return Value::Num(self.view.above_top_solid(arg(0), arg(1)) as f32);
        }
        if query == self.queries.heightmap {
            return Value::Num(self.view.heightmap(arg(0), arg(1)) as f32);
        }
        if query == self.queries.noise {
            return Value::Num(noise().value(num(0), num(1)));
        }
        Value::ZERO
    }
}

/// A host whose `math.random` draws from `random`.
struct Drawing<'a> {
    host: &'a dyn Host,
    random: RefCell<&'a mut Mt19937>,
}

impl Host for Drawing<'_> {
    fn query(&self, query: Query, args: &[Value], structs: &mut Structs<'_>) -> Value {
        self.host.query(query, args, structs)
    }

    fn random(&self) -> f32 {
        self.random.borrow_mut().next_float()
    }
}

/// The scatter variables of one scope, saved around a nested scatter, which BDS runs on a copy.
pub struct Saved([Option<Value>; 6]);

pub struct Scope {
    pub queries: Queries,
    compiler: Compiler,
    programs: HashMap<&'static str, Program>,
    variables: Variables,
    scratch: Scratch,
    origin: [Variable; 3],
    world: [Variable; 3],
    /// The biome `query.has_biome_tag` asks about: a scatter's origin's.
    pub biome: Option<u32>,
    /// Whether a scatter of this rule has run yet: see `Scatter::place`.
    pub world_bound: bool,
}

impl Scope {
    pub fn new() -> Self {
        let mut compiler = Compiler::new();
        let origin = ["originx", "originy", "originz"].map(|name| compiler.variable(name));
        let world = ["worldx", "worldy", "worldz"].map(|name| compiler.variable(name));
        let queries =
            Queries { above_top_solid: compiler.query("above_top_solid"), heightmap: compiler.query("heightmap"), noise: compiler.query("noise") };
        Self { queries, compiler, programs: HashMap::new(), variables: Variables::new(), scratch: Scratch::new(), origin, world, biome: None, world_bound: false }
    }

    pub fn set_origin(&mut self, origin: [i32; 3]) {
        for (variable, value) in self.origin.into_iter().zip(origin) {
            self.variables.set(variable, value as f32);
        }
    }

    pub fn set_world(&mut self, axis: Axis, value: i32) {
        let slot = match axis {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        };
        self.variables.set(self.world[slot], value as f32);
    }

    pub fn save(&self) -> Saved {
        let mut all = self.origin.into_iter().chain(self.world);
        Saved(std::array::from_fn(|_| self.variables.get(all.next().expect("six variables"))))
    }

    pub fn restore(&mut self, saved: Saved) {
        for (variable, value) in self.origin.into_iter().chain(self.world).zip(saved.0) {
            self.variables.set(variable, value.unwrap_or(Value::ZERO));
        }
    }

    /// An expression that fails to compile is 0, as BDS loads such a field and evaluates it to 0.
    pub fn eval(&mut self, expr: Expr, host: &dyn Host, random: &mut Mt19937) -> f32 {
        let source = match expr {
            Expr::Const(value) => return value,
            Expr::Molang(source) => source,
            Expr::ByBiome(table) => {
                return self.biome.and_then(|biome| table.iter().find(|(id, _)| *id == biome)).map_or(0.0, |&(_, value)| value);
            }
        };
        if !self.programs.contains_key(source) {
            let program = self.compiler.compile(source).unwrap_or_else(|_| Compiler::constant(0.0));
            self.programs.insert(source, program);
        }
        let host = Drawing { host, random: RefCell::new(random) };
        let env = &mut Env { host: &host, variables: &mut self.variables, scratch: &mut self.scratch, this: 0.0 };
        self.programs[source].eval(env).num()
    }
}

/// A scope with the host its queries ask.
pub struct Molang<'a> {
    pub scope: &'a mut Scope,
    pub host: &'a dyn Host,
}

impl Molang<'_> {
    pub fn eval(&mut self, expr: Expr, random: &mut Mt19937) -> f32 {
        self.scope.eval(expr, self.host, random)
    }

    pub fn set_world(&mut self, axis: Axis, value: i32) {
        self.scope.set_world(axis, value);
    }
}
