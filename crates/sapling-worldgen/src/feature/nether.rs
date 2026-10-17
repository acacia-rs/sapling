//! The Nether's hardcoded decoration (`NetherGenerator::decorationPostProcessChunk` 1481ed1f0), run before its
//! biomes' feature rules on the chunk's post-process generator.

use super::Inputs;
use super::view::View;
use crate::random::Mt19937;

// TODO: the legacy hell features (springs, fire, glowstone, mushrooms, ores) in BDS's order.
pub fn decorate(_random: &mut Mt19937, _source: (i32, i32), _inputs: Inputs, _view: &mut View) {}
