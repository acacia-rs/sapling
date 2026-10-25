//! A structure turned and mirrored as `/structure load` turns it: the mirror first, then the
//! clockwise turn, each cell kept inside the box and each block's facing states turned with it
//! (research/captures/bds-structures.txt: a chest, stairs and a log at every turn and mirror).
// TODO: the turns of `coral_direction`, `huge_mushroom_bits` and the other states BDS turns that
// no drill showed are left as saved.

use sapling_api::{Mirror, Rotation};

/// Compass quarters in the order a clockwise turn walks them.
const COMPASS: [&str; 4] = ["south", "west", "north", "east"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Turn {
    /// Quarter turns clockwise, seen from above.
    pub quarters: u8,
    /// `x`: z runs the other way.
    pub flip_z: bool,
    /// `z`: x runs the other way.
    pub flip_x: bool,
}

impl Turn {
    pub(crate) fn of(rotation: Rotation, mirror: Mirror) -> Self {
        let quarters = match rotation {
            Rotation::None => 0,
            Rotation::Clockwise90 => 1,
            Rotation::Clockwise180 => 2,
            Rotation::Clockwise270 => 3,
        };
        Self { quarters, flip_z: matches!(mirror, Mirror::X | Mirror::XZ), flip_x: matches!(mirror, Mirror::Z | Mirror::XZ) }
    }

    /// Where the cell at `offset` of a box of `size` lands, from the turned box's lowest corner.
    pub(crate) fn cell(&self, [x, y, z]: [i32; 3], [sx, _, sz]: [i32; 3]) -> [i32; 3] {
        let (x, z) = self.plane([x as f32, z as f32], [sx as f32 - 1.0, sz as f32 - 1.0]);
        [x as i32, y, z as i32]
    }

    /// Where a point at `offset` (an entity's feet) lands.
    pub(crate) fn point(&self, [x, y, z]: [f32; 3], [sx, _, sz]: [i32; 3]) -> [f32; 3] {
        let (x, z) = self.plane([x, z], [sx as f32, sz as f32]);
        [x, y, z]
    }

    fn plane(&self, [mut x, mut z]: [f32; 2], [far_x, far_z]: [f32; 2]) -> (f32, f32) {
        if self.flip_z {
            z = far_z - z;
        }
        if self.flip_x {
            x = far_x - x;
        }
        match self.quarters % 4 {
            1 => (far_z - z, x),
            2 => (far_x - x, far_z - z),
            3 => (z, far_x - x),
            _ => (x, z),
        }
    }

    /// An entity's yaw. BDS mirrors a yaw the other way from its place: `x` leaves 0 as it was and
    /// `z` makes it -180 (bds-structures.txt); the result runs from -180 up to 180.
    pub(crate) fn yaw(&self, yaw: f32) -> f32 {
        let mut yaw = yaw;
        if self.flip_z {
            yaw = -yaw;
        }
        if self.flip_x {
            yaw = 180.0 - yaw;
        }
        (yaw + 90.0 * f32::from(self.quarters) + 180.0).rem_euclid(360.0) - 180.0
    }

    /// A compass quarter (0 south, 1 west, 2 north, 3 east) turned.
    fn quarter(&self, quarter: usize) -> usize {
        let mut quarter = quarter % 4;
        if self.flip_z && quarter % 2 == 0 {
            quarter = 2 - quarter;
        }
        if self.flip_x && quarter % 2 == 1 {
            quarter = 4 - quarter;
        }
        (quarter + usize::from(self.quarters)) % 4
    }

    fn name(&self, name: &str) -> Option<&'static str> {
        Some(COMPASS[self.quarter(COMPASS.iter().position(|side| *side == name)?)])
    }

    /// The value in `order` (four values in compass order) that `value` turns to.
    fn in_order(&self, value: &str, order: [&'static str; 4]) -> Option<&'static str> {
        Some(order[self.quarter(order.iter().position(|v| *v == value)?)])
    }

    /// A block's states (`key=value,...`, sorted) turned, sorted again.
    pub(crate) fn states(&self, block: &str, states: &str) -> String {
        if *self == Self::default() || states.is_empty() {
            return states.to_owned();
        }
        let mut pairs: Vec<String> = states.split(',').map(|pair| match pair.split_once('=') {
            Some((key, value)) => {
                let (key, value) = self.state(block, key, value);
                format!("{key}={value}")
            }
            None => pair.to_owned(),
        }).collect();
        pairs.sort_unstable();
        pairs.join(",")
    }

    fn state(&self, block: &str, key: &str, value: &str) -> (String, String) {
        let odd = self.quarters % 2 == 1;
        let turned: Option<String> = match key {
            "minecraft:cardinal_direction" | "torch_facing_direction" | "minecraft:facing_direction" | "minecraft:block_face" => self.name(value).map(str::to_owned),
            // 0 down, 1 up, 2 north, 3 south, 4 west, 5 east.
            "facing_direction" => self.in_order(value, ["3", "4", "2", "5"]).map(str::to_owned),
            "weirdo_direction" => self.in_order(value, ["2", "1", "3", "0"]).map(str::to_owned),
            "direction" if block.ends_with("trapdoor") => self.in_order(value, ["2", "1", "3", "0"]).map(str::to_owned),
            "direction" => self.in_order(value, ["0", "1", "2", "3"]).map(str::to_owned),
            "pillar_axis" | "portal_axis" if odd => Some(match value { "x" => "z", "z" => "x", other => other }.to_owned()),
            "ground_sign_direction" => value.parse::<i32>().ok().map(|sixteenth| self.sixteenth(sixteenth).to_string()),
            "lever_direction" => Some(self.lever(value)),
            "rail_direction" => value.parse::<usize>().ok().map(|rail| self.rail(rail).to_string()),
            "vine_direction_bits" => value.parse::<u32>().ok().map(|bits| self.bits(bits, [1, 2, 4, 8]).to_string()),
            "multi_face_direction_bits" => value.parse::<u32>().ok().map(|bits| self.bits(bits, [4, 8, 16, 32]).to_string()),
            _ => None,
        };
        let key = match key.rsplit_once('_') {
            Some((stem, side)) if COMPASS.contains(&side) => format!("{stem}_{}", self.name(side).expect("a compass side")),
            _ => key.to_owned(),
        };
        (key, turned.unwrap_or_else(|| value.to_owned()))
    }

    /// A standing sign's sixteenth of a turn: 0 faces south, 4 west.
    fn sixteenth(&self, sixteenth: i32) -> i32 {
        let mut turn = sixteenth.rem_euclid(16);
        if self.flip_z {
            turn = (8 - turn).rem_euclid(16);
        }
        if self.flip_x {
            turn = (16 - turn) % 16;
        }
        (turn + 4 * i32::from(self.quarters)) % 16
    }

    fn lever(&self, value: &str) -> String {
        match value.split_once('_') {
            Some((end, axis)) if self.quarters % 2 == 1 => format!("{end}_{}", if axis == "north_south" { "east_west" } else { "north_south" }),
            Some(_) => value.to_owned(),
            None => self.name(value).unwrap_or(value).to_owned(),
        }
    }

    /// 0 north-south, 1 east-west, 2..=5 rising east, west, north, south, 6..=9 the curves
    /// south-east, south-west, north-west, north-east.
    fn rail(&self, rail: usize) -> usize {
        const RISING: [usize; 4] = [5, 3, 4, 2];
        match rail {
            0 | 1 => rail ^ usize::from(self.quarters % 2 == 1),
            2..=5 => RISING[self.quarter(RISING.iter().position(|r| *r == rail).expect("a rising rail"))],
            6..=9 => {
                let corners = [[0, 3], [0, 1], [2, 1], [2, 3]];
                let mut turned = corners[rail - 6].map(|quarter| self.quarter(quarter));
                turned.sort_unstable();
                6 + corners.iter().position(|corner| {
                    let mut corner = *corner;
                    corner.sort_unstable();
                    corner == turned
                }).expect("a corner turns to a corner")
            }
            _ => rail,
        }
    }

    /// Bits named for the compass quarters in `order`, each moved with its quarter.
    fn bits(&self, bits: u32, order: [u32; 4]) -> u32 {
        let others = bits & !order.iter().sum::<u32>();
        order.iter().enumerate().filter(|(_, bit)| bits & **bit != 0).fold(others, |turned, (quarter, _)| turned | order[self.quarter(quarter)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(quarters: u8, flip_z: bool, flip_x: bool) -> Turn {
        Turn { quarters, flip_z, flip_x }
    }

    /// bds-structures.txt: a 3x3x3 box with a chest facing east at (1, 1, 0), stairs facing east at
    /// (2, 1, 0) and a log along x at (1, 1, 2).
    #[test]
    fn cells_and_states_turn_as_on_bds() {
        let size = [3, 3, 3];
        let cases = [
            (turn(1, false, false), [2, 1, 1], "south", "2", "z"),
            (turn(2, false, false), [1, 1, 2], "west", "1", "x"),
            (turn(3, false, false), [0, 1, 1], "north", "3", "z"),
            (turn(0, true, false), [1, 1, 2], "east", "0", "x"),
            (turn(0, false, true), [1, 1, 0], "west", "1", "x"),
            (turn(1, true, true), [0, 1, 1], "north", "3", "z"),
        ];
        for (turn, chest, facing, stairs, axis) in cases {
            assert_eq!(turn.cell([1, 1, 0], size), chest, "{turn:?}");
            assert_eq!(turn.states("minecraft:chest", "minecraft:cardinal_direction=east"), format!("minecraft:cardinal_direction={facing}"));
            assert_eq!(turn.states("minecraft:oak_stairs", "upside_down_bit=0,weirdo_direction=0"), format!("upside_down_bit=0,weirdo_direction={stairs}"));
            assert_eq!(turn.states("minecraft:oak_log", "pillar_axis=x"), format!("pillar_axis={axis}"));
        }
    }

    #[test]
    fn yaws_turn_as_on_bds() {
        let yaws = [(turn(1, false, false), 90.0), (turn(2, false, false), -180.0), (turn(3, false, false), -90.0), (turn(0, true, false), 0.0), (turn(0, false, true), -180.0), (turn(1, true, true), -90.0)];
        for (turn, yaw) in yaws {
            assert_eq!(turn.yaw(0.0), yaw, "{turn:?}");
        }
        assert_eq!(turn(1, false, false).point([1.5, 1.0, 1.5], [3, 3, 3]), [1.5, 1.0, 1.5]);
        assert_eq!(turn(1, false, false).point([0.5, 1.0, 0.5], [3, 1, 2]), [1.5, 1.0, 0.5]);
    }

    #[test]
    fn rails_signs_levers_and_sides_turn() {
        let quarter = turn(1, false, false);
        assert_eq!(quarter.states("minecraft:rail", "rail_direction=0"), "rail_direction=1");
        assert_eq!(quarter.states("minecraft:rail", "rail_direction=2"), "rail_direction=5", "rising east turns to rising south");
        assert_eq!(quarter.states("minecraft:rail", "rail_direction=6"), "rail_direction=7", "south-east turns to south-west");
        assert_eq!(quarter.states("minecraft:standing_sign", "ground_sign_direction=15"), "ground_sign_direction=3");
        assert_eq!(quarter.states("minecraft:lever", "lever_direction=up_north_south,open_bit=0"), "lever_direction=up_east_west,open_bit=0");
        assert_eq!(quarter.states("minecraft:vine", "vine_direction_bits=1"), "vine_direction_bits=2");
        assert_eq!(quarter.states("minecraft:cobblestone_wall", "wall_connection_type_east=short,wall_connection_type_north=none"), "wall_connection_type_east=none,wall_connection_type_south=short");
        assert_eq!(turn(0, true, false).states("minecraft:standing_sign", "ground_sign_direction=1"), "ground_sign_direction=7");
    }
}
