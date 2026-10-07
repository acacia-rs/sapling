//! The Molang fields of feature data, compiled once per expression and evaluated with the scatter's
//! `variable.origin{x,y,z}` and `variable.world{x,y,z}`.

use std::collections::HashMap;

use acacia_molang::{Compiler, Env, Host, Program, Scratch, Variable, Variables};

use super::spec::{Axis, Expr};

pub struct Scope {
    compiler: Compiler,
    programs: HashMap<&'static str, Program>,
    variables: Variables,
    scratch: Scratch,
    origin: [Variable; 3],
    world: [Variable; 3],
}

impl Scope {
    pub fn new() -> Self {
        let mut compiler = Compiler::new();
        let origin = ["originx", "originy", "originz"].map(|name| compiler.variable(name));
        let world = ["worldx", "worldy", "worldz"].map(|name| compiler.variable(name));
        Self { compiler, programs: HashMap::new(), variables: Variables::new(), scratch: Scratch::new(), origin, world }
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

    /// An expression that fails to compile is 0, as BDS loads such a field and evaluates it to 0.
    pub fn eval(&mut self, expr: Expr, host: &dyn Host) -> f32 {
        let source = match expr {
            Expr::Const(value) => return value,
            Expr::Molang(source) => source,
        };
        if !self.programs.contains_key(source) {
            let program = self.compiler.compile(source).unwrap_or_else(|_| Compiler::constant(0.0));
            self.programs.insert(source, program);
        }
        let env = &mut Env { host, variables: &mut self.variables, scratch: &mut self.scratch, this: 0.0 };
        self.programs[source].eval(env).num()
    }
}

/// A scope with the host its queries ask.
pub struct Molang<'a> {
    pub scope: &'a mut Scope,
    pub host: &'a dyn Host,
}

impl Molang<'_> {
    pub fn eval(&mut self, expr: Expr) -> f32 {
        self.scope.eval(expr, self.host)
    }

    pub fn set_world(&mut self, axis: Axis, value: i32) {
        self.scope.set_world(axis, value);
    }
}
