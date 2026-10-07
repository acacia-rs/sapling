//! Scatter parameters, shared by feature rules and `minecraft:scatter_feature`: the chance gate,
//! the iteration count and one position per iteration, drawing from the positions generator.

use super::molang::Molang;
use super::spec::{Axis, Chance, Coordinate, Distribution, Expr};
use crate::random::Mt19937;

/// The generators a scatter draws from: positions and gates from `positions`, `math.random` from
/// `molang` (a rule's chunk generator) or, without one, from `positions` too.
pub struct Draws<'a> {
    pub positions: &'a mut Mt19937,
    pub molang: Option<&'a mut Mt19937>,
}

impl Draws<'_> {
    fn eval(&mut self, scope: &mut Molang<'_>, expr: Expr) -> f32 {
        let random = match &mut self.molang {
            Some(molang) => &mut **molang,
            None => &mut *self.positions,
        };
        scope.eval(expr, random)
    }
}

/// The number of iterations to run: zero if the chance gate fails.
pub fn iterations(dist: &Distribution, draws: &mut Draws<'_>, scope: &mut Molang<'_>) -> u32 {
    let scatter = match dist.chance {
        Chance::Fraction(numerator, denominator) if numerator >= 1 && denominator >= 1 => {
            numerator == denominator || draws.positions.next_int(denominator) < numerator
        }
        Chance::Fraction(..) => true,
        Chance::Percent(percent) => {
            let percent = draws.eval(scope, percent);
            if percent >= 100.0 {
                true
            } else if percent <= 0.0 {
                false
            } else {
                draws.positions.next_float() * 100.0 < percent
            }
        }
    };
    if !scatter {
        return 0;
    }
    let count = draws.eval(scope, dist.iterations).round();
    if count > 0.0 { count as u32 } else { 0 }
}

/// The position of the iteration `index` (counting down from `iterations - 1`), axes in the
/// distribution's order; each axis's absolute value is visible to the later ones as
/// `variable.world{x,y,z}`.
pub fn position(dist: &Distribution, index: u32, origin: [i32; 3], draws: &mut Draws<'_>, scope: &mut Molang<'_>) -> [i32; 3] {
    let mut grid = index;
    let mut at = origin;
    for axis in dist.order {
        let (coordinate, slot) = match axis {
            Axis::X => (&dist.x, 0),
            Axis::Y => (&dist.y, 1),
            Axis::Z => (&dist.z, 2),
        };
        at[slot] = origin[slot] + offset(coordinate, &mut grid, draws, scope);
        scope.set_world(axis, at[slot]);
    }
    at
}

fn bound(draws: &mut Draws<'_>, scope: &mut Molang<'_>, expr: Expr) -> i32 {
    draws.eval(scope, expr).round() as i32
}

fn offset(coordinate: &Coordinate, grid: &mut u32, draws: &mut Draws<'_>, scope: &mut Molang<'_>) -> i32 {
    let range = match coordinate {
        Coordinate::Fixed(value) => return bound(draws, scope, *value),
        Coordinate::Uniform(range) | Coordinate::Gaussian(range) | Coordinate::InverseGaussian(range) | Coordinate::Triangle(range) => range,
        Coordinate::FixedGrid { extent, .. } | Coordinate::JitteredGrid { extent, .. } => extent,
    };
    let (min, max) = (bound(draws, scope, range[0]), bound(draws, scope, range[1]));
    let random = &mut *draws.positions;
    match coordinate {
        Coordinate::Fixed(_) => unreachable!("returned above"),
        Coordinate::Uniform(_) => {
            if max > min { min + random.next_int(max - min) } else { min }
        }
        Coordinate::Gaussian(_) => {
            let half = (max - min + i32::from(max < min)) >> 1;
            let a = random.next_int(half);
            min + half + a - random.next_int(half)
        }
        Coordinate::InverseGaussian(_) => {
            let half = (max - min) / 2;
            let a = random.next_int(half);
            let difference = a - random.next_int(half);
            if difference < 0 {
                min - difference
            } else if difference > 0 {
                max - difference
            } else if random.next_boolean() {
                max
            } else {
                min
            }
        }
        Coordinate::Triangle(_) => {
            let half = (max - min + i32::from(max < min)) >> 1;
            let a = inclusive(random, half);
            min + a + inclusive(random, max - min - half)
        }
        Coordinate::FixedGrid { step, offset, .. } | Coordinate::JitteredGrid { step, offset, .. } => {
            let modulus = max - min + 1;
            let mut cell = offset + min + *grid as i32 * step;
            if matches!(coordinate, Coordinate::JitteredGrid { .. }) && *step >= 2 {
                cell += random.next_int(*step);
            }
            // The index carried to the next axis has no min term and divides unsigned.
            let carried = (*grid).wrapping_mul(*step as u32).wrapping_add(*offset as u32);
            *grid = if modulus == 0 { 0 } else { carried / modulus as u32 };
            if modulus == 0 { 0 } else { cell % modulus }
        }
    }
}

/// `nextInt(0, max)` inclusive: no draw when max < 0.
fn inclusive(random: &mut Mt19937, max: i32) -> i32 {
    if max < 0 { 0 } else { random.next_int(max + 1) }
}
