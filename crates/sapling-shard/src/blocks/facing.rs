//! The state a placed block takes from how it was placed: which face was clicked, how high on
//! it, and which way the placer looked.
// TODO: only the properties below; rails, skulls and the rest keep the item's own value (a
// door's hinge, which its neighbours decide, is doors.rs's). Each value's evidence is in docs/research/block-rules.md: collision
// boxes per state, the Java game's placement code, and for pistons only other servers' code.

use sapling_api::Face;
use sapling_world::BlockRegistry;

/// How far over a placer's feet the game takes its eyes to be when it turns a piston by them.
pub(super) const EYES: f32 = 1.82;

/// How a block was placed.
#[derive(Debug, Clone, Copy)]
pub(super) struct Click {
    pub face: Face,
    /// How high on the clicked block, 0 to 1.
    pub height: f32,
    /// The placer's yaw: 0 looks south (+z), 90 west.
    pub yaw: f32,
    /// How far over the bottom of the cell the placer's eyes are, where it stands within two
    /// blocks of it across; the game takes the eyes as 1.82 over the feet here.
    pub eyes_over: Option<f32>,
}

impl Click {
    /// The quarter the placer looks into: 0 south, 1 west, 2 north, 3 east.
    pub(super) fn quarter(&self) -> usize {
        // Half-way turns go up, as the game rounds them: `round()` would send -45 the other way.
        if self.yaw.is_finite() { (self.yaw / 90.0 + 0.5).floor().rem_euclid(4.0) as usize % 4 } else { 0 }
    }

    /// Whether the block goes into the upper half of its cell.
    fn is_top(&self) -> bool {
        match self.face {
            Face::Down => true,
            Face::Up => false,
            _ => self.height > 0.5,
        }
    }

    /// The sixteenth of a turn a standing sign shows its face to, towards the placer.
    fn sixteenth(&self) -> &'static str {
        const TURNS: [&str; 16] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"];
        let yaw = if self.yaw.is_finite() { self.yaw } else { 0.0 };
        TURNS[((yaw + 180.0) * 16.0 / 360.0 + 0.5).floor().rem_euclid(16.0) as usize % 16]
    }

    /// A hanging sign's `facing_direction`: under a ceiling it faces its placer; on a wall its
    /// board sticks out, facing away from the way along the wall the placer looks (Java
    /// `CeilingHangingSignBlock`, `WallHangingSignBlock`).
    fn hanging_sign_facing(&self) -> &'static str {
        let yaw = if self.yaw.is_finite() { self.yaw.to_radians() } else { 0.0 };
        match self.face {
            Face::North | Face::South => if -yaw.sin() < 0.0 { "5" } else { "4" },
            Face::West | Face::East => if yaw.cos() > 0.0 { "2" } else { "3" },
            Face::Down | Face::Up => ["2", "5", "3", "4"][self.quarter()],
        }
    }

    fn value(&self, block: &str, property: &str) -> Option<&'static str> {
        let ends = |list: &[&str]| list.iter().any(|end| block.ends_with(end));
        let on_a_face = ends(&["ladder", "button", "wall_sign", "wall_banner", "_skull", "_head"]);
        // A dispenser, a piston and an observer turn up or down by where their placer stands, not
        // by its look: one put a block east of the placer's feet under a look 58 degrees down
        // faced it level (research/captures/bds-placing.txt). The bounds are the old game's.
        let (looks_down, looks_up) = (self.eyes_over.is_some_and(|over| over > 2.0), self.eyes_over.is_some_and(|over| over < 0.0));
        Some(match property {
            // Dispensers and the like face their placer, up or down under a steep look.
            "facing_direction" if ends(&["dispenser", "dropper", "barrel"]) && looks_down => "1",
            "facing_direction" if ends(&["dispenser", "dropper", "barrel"]) && looks_up => "0",
            "facing_direction" if ends(&["dispenser", "dropper", "barrel", "glazed_terracotta"]) => ["2", "5", "3", "4"][self.quarter()],
            // A piston's head faces the placer too, but its sideways values name the other end.
            "facing_direction" if block.ends_with("piston") => match (looks_down, looks_up) {
                (true, _) => "1",
                (_, true) => "0",
                _ => ["3", "4", "2", "5"][self.quarter()],
            },
            // An observer watches the way its placer looks.
            "minecraft:facing_direction" if block.ends_with("observer") => match (looks_down, looks_up) {
                (true, _) => "down",
                (_, true) => "up",
                _ => ["south", "west", "north", "east"][self.quarter()],
            },
            // A hopper's spout points into the clicked block.
            "facing_direction" if block.ends_with("hopper") => ["0", "0", "3", "2", "5", "4"][self.face as usize],
            "facing_direction" if ends(&["end_rod", "lightning_rod"]) => ["0", "1", "2", "3", "4", "5"][self.face as usize],
            // A bed's head lies the way its placer looks.
            "direction" if block.ends_with(":bed") => ["0", "1", "2", "3"][self.quarter()],
            "direction" if ends(&["loom", "beehive", "bee_nest", ":bell", "grindstone", "decorated_pot", "chiseled_bookshelf"]) => ["2", "3", "0", "1"][self.quarter()],
            // A crafter's front faces its placer, its top up (bds-placing2.txt; one that looks up
            // or down was not tried).
            "orientation" if block.ends_with(":crafter") => ["north_up", "east_up", "south_up", "west_up"][self.quarter()],
            // What clings names the side it clings to: the block clicked.
            "multi_face_direction_bits" => ["2", "1", "4", "16", "32", "8"][self.face as usize],
            // A cocoa pod names the side its log is on: the clicked one.
            "direction" if block.ends_with(":cocoa") => ["0", "0", "0", "2", "1", "3"][self.face as usize],
            // A rail starts north and south whatever the look; rails.rs then joins it to its neighbours.
            "rail_direction" => "0",
            // A door stands across the look and an anvil the other way across; a gate, as a
            // furnace, faces its placer (bds-placing.txt).
            "minecraft:cardinal_direction" if block.ends_with("_door") && !block.ends_with("trapdoor") => ["west", "north", "east", "south"][self.quarter()],
            "minecraft:cardinal_direction" if block.ends_with("anvil") => ["east", "south", "west", "north"][self.quarter()],
            "minecraft:cardinal_direction" if ends(&["campfire", "calibrated_sculk_sensor"]) => ["south", "west", "north", "east"][self.quarter()],
            // An amethyst cluster and an item frame name the face they were put on.
            "minecraft:block_face" => ["down", "up", "north", "south", "west", "east"][self.face as usize],
            "facing_direction" if ends(&[":frame", ":glow_frame"]) => ["0", "1", "2", "3", "4", "5"][self.face as usize],
            "facing_direction" if block.ends_with("_hanging_sign") => self.hanging_sign_facing(),
            "lever_direction" => match (self.face, self.quarter() % 2 == 1) {
                (Face::Up, false) => "up_north_south",
                (Face::Up, true) => "up_east_west",
                (Face::Down, false) => "down_north_south",
                (Face::Down, true) => "down_east_west",
                (Face::North, _) => "north",
                (Face::South, _) => "south",
                (Face::West, _) => "west",
                (Face::East, _) => "east",
            },
            "ground_sign_direction" => self.sixteenth(),
            // Named for the side the torch leans away from: the block it hangs on.
            "torch_facing_direction" => ["top", "top", "south", "north", "east", "west"][self.face as usize],
            "facing_direction" if on_a_face => ["0", "1", "2", "3", "4", "5"][self.face as usize],
            "direction" if block.ends_with("trapdoor") => ["3", "0", "2", "1"][self.quarter()],
            "pillar_axis" => match self.face {
                Face::Down | Face::Up => "y",
                Face::North | Face::South => "z",
                Face::West | Face::East => "x",
            },
            // A furnace faces its placer; stairs rise the way the placer looks.
            "minecraft:cardinal_direction" => ["north", "east", "south", "west"][self.quarter()],
            "weirdo_direction" => ["2", "1", "3", "0"][self.quarter()],
            "upside_down_bit" => ["0", "1"][usize::from(self.is_top())],
            "minecraft:vertical_half" => ["bottom", "top"][usize::from(self.is_top())],
            _ => return None,
        })
    }
}

/// The state of `id`'s block that `click` places; `id` itself for a block that does not turn.
pub(super) fn oriented(registry: &BlockRegistry, id: u32, click: &Click) -> u32 {
    let Some(state) = registry.get(id).filter(|state| !state.properties.is_empty()) else { return id };
    let turned = |pair: &str| match pair.split_once('=') {
        Some((key, value)) => format!("{key}={}", click.value(state.name, key).unwrap_or(value)),
        None => pair.to_owned(),
    };
    let properties: Vec<String> = state.properties.split(',').map(turned).collect();
    registry.find(state.name, &properties.join(",")).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(block: &str, face: Face, height: f32, yaw: f32) -> String {
        let registry = BlockRegistry::vanilla();
        let id = registry.states_of(block).next().expect("a vanilla block").0;
        registry.get(oriented(&registry, id, &Click { face, height, yaw, eyes_over: None })).expect("a state").properties.to_owned()
    }

    #[test]
    fn a_placed_block_turns_to_the_click_and_the_placer() {
        assert_eq!([placed("minecraft:oak_log", Face::Up, 1.0, 0.0), placed("minecraft:oak_log", Face::East, 0.5, 0.0)], ["pillar_axis=y", "pillar_axis=x"]);
        assert!(placed("minecraft:oak_stairs", Face::Up, 1.0, -90.0).ends_with("upside_down_bit=0,weirdo_direction=0"), "looking east");
        assert!(placed("minecraft:oak_stairs", Face::North, 0.8, 170.0).ends_with("upside_down_bit=1,weirdo_direction=3"), "high on a side, looking north");
        assert!(placed("minecraft:furnace", Face::Up, 1.0, 0.0).contains("minecraft:cardinal_direction=north"), "faces a placer who looks south");
        assert!(placed("minecraft:stone_brick_slab", Face::Down, 0.0, 0.0).contains("minecraft:vertical_half=top"));
        assert_eq!(placed("minecraft:stone", Face::Up, 1.0, 0.0), "");
        assert_eq!([placed("minecraft:torch", Face::Up, 1.0, 0.0), placed("minecraft:torch", Face::East, 0.5, 0.0)], ["torch_facing_direction=top", "torch_facing_direction=west"]);
        assert!(placed("minecraft:ladder", Face::North, 0.5, 0.0).contains("facing_direction=2"));
        assert!(placed("minecraft:lever", Face::Up, 1.0, 90.0).contains("lever_direction=up_east_west") && placed("minecraft:lever", Face::South, 0.5, 0.0).contains("lever_direction=south"));
        assert!(placed("minecraft:dispenser", Face::Up, 1.0, 0.0).contains("facing_direction=2"), "faces a placer who looks south");
        assert!(placed("minecraft:piston", Face::Up, 1.0, 0.0).contains("facing_direction=3") && placed("minecraft:observer", Face::Up, 1.0, 0.0).contains("minecraft:facing_direction=south"));
        assert!(placed("minecraft:spruce_door", Face::Up, 1.0, 0.0).contains("minecraft:cardinal_direction=west"), "across the look");
        assert!(placed("minecraft:oak_stairs", Face::Up, 1.0, -45.0).ends_with("weirdo_direction=2"), "half-way between east and south is south");
        assert!(placed("minecraft:standing_sign", Face::Up, 1.0, 0.0).contains("ground_sign_direction=8"));
        assert!(placed("minecraft:bed", Face::Up, 1.0, 90.0).contains("direction=1"), "head to the west");
        // research/captures/bds-placing.txt, a placer looking east.
        assert!(placed("minecraft:fence_gate", Face::Up, 1.0, -90.0).contains("minecraft:cardinal_direction=west") && placed("minecraft:anvil", Face::Up, 1.0, -90.0).contains("minecraft:cardinal_direction=north"));
        assert!(placed("minecraft:bell", Face::Up, 1.0, -90.0).contains("direction=1") && placed("minecraft:grindstone", Face::Up, 1.0, -90.0).contains("direction=1"));
        assert!(placed("minecraft:amethyst_cluster", Face::Up, 1.0, -90.0).contains("block_face=up") && placed("minecraft:frame", Face::Up, 1.0, -90.0).contains("facing_direction=1"));
        assert_eq!(placed("minecraft:rail", Face::Up, 1.0, -90.0), "rail_direction=0");
        // research/captures/bds-placing2.txt, the same placer.
        assert!(placed("minecraft:crafter", Face::Up, 1.0, -90.0).contains("orientation=west_up") && placed("minecraft:decorated_pot", Face::Up, 1.0, -90.0).contains("direction=1"));
        assert!(placed("minecraft:chiseled_bookshelf", Face::Up, 1.0, -90.0).contains("direction=1") && placed("minecraft:calibrated_sculk_sensor", Face::Up, 1.0, -90.0).contains("cardinal_direction=east"));
        assert_eq!([placed("minecraft:glow_lichen", Face::Up, 1.0, -90.0), placed("minecraft:sculk_vein", Face::North, 0.5, 0.0)], ["multi_face_direction_bits=1", "multi_face_direction_bits=4"]);
        let registry = BlockRegistry::vanilla();
        let piston = registry.states_of("minecraft:piston").next().unwrap().0;
        let facing = |eyes_over| registry.get(oriented(&registry, piston, &Click { face: Face::Up, height: 1.0, yaw: -90.0, eyes_over })).unwrap().properties.to_owned();
        assert_eq!([facing(Some(1.82)), facing(Some(2.82)), facing(Some(-0.2)), facing(None)], ["facing_direction=5", "facing_direction=1", "facing_direction=0", "facing_direction=5"]);
    }
}
