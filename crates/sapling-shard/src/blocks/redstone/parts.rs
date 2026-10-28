//! What a block state is to a circuit (docs/research/redstone.md 2).

use sapling_api::Face;
use sapling_world::BlockRegistry;

pub(super) const FULL: u8 = 15;
pub(super) const SIGNAL: &str = "redstone_signal";

/// Blocks a signal opens and shuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Opens {
    Door,
    Trapdoor,
    Gate,
}

/// What uses a signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum User {
    Lamp { lit: bool },
    Opens(Opens),
    Hopper,
    /// A powered or activator rail, on while powered or within reach of one along its line
    /// (rail_line.rs, vehicles/rails.rs); a plain rail, which switches at a junction (rails.rs).
    Rail,
    Tnt,
    /// Pushes out of its `front`, where it takes no power.
    Piston { front: Face },
    /// A dispenser, dropper or crafter, `triggered` by the rise of its last signal.
    Fires { triggered: bool },
    /// Plays on a rise of its signal.
    Note,
    /// Rings on a rise of its signal (bells.rs).
    Bell,
    /// A copper bulb, which turns on and off at each rise of its signal.
    Bulb { powered: bool },
    /// A command block of any kind (command_blocks.rs).
    Command,
    /// A structure block (structure_blocks.rs).
    Structure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// A lever, button, plate or block of redstone giving `strength`, strongly into the block on
    /// `strong` (the one it is attached to; under a plate).
    Producer { strength: u8, strong: Option<Face> },
    Dust { signal: u8 },
    /// Attached on `support`; powers the block above strongly.
    Torch { lit: bool, support: Face },
    /// Takes power on its `input` side and gives it out of the other, `delay` evaluations later.
    Repeater { lit: bool, input: Face, delay: u8 },
    /// Watches the cell on its `watch` side and, `lit`, powers what is on the other.
    Observer { lit: bool, watch: Face },
    /// Takes power on its `input` side and its two sides, and gives `output` out of the fourth.
    Comparator { lit: bool, input: Face, subtracts: bool, output: u8 },
    User(User),
    /// A block powered by what points into it, and passing that on. Cuts a step of dust under it.
    Conductor,
    /// Passes no power but cuts a step of dust under it.
    Cutter,
    /// Passes no power, and dust on it does not step down its sides.
    Ledge,
    Other,
}

impl Part {
    pub(super) fn of(registry: &BlockRegistry, id: u32) -> Part {
        let Some(state) = registry.get(id) else { return Part::Other };
        let (name, properties) = (state.name.strip_prefix("minecraft:").unwrap_or(state.name), state.properties);
        let get = |key: &str| properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map(|(_, value)| value);
        let on = |key: &str| if get(key) == Some("1") { FULL } else { 0 };
        match name {
            "redstone_block" => Part::Producer { strength: FULL, strong: None },
            "lever" => Part::Producer { strength: on("open_bit"), strong: get("lever_direction").and_then(lever_support) },
            _ if name.ends_with("_button") => {
                let support = get("facing_direction").and_then(|face| face.parse().ok()).and_then(Face::from_index).map(opposite);
                Part::Producer { strength: on("button_pressed_bit"), strong: support }
            }
            _ if name.ends_with("pressure_plate") => {
                let stored: u8 = get(SIGNAL).and_then(|signal| signal.parse().ok()).unwrap_or(0);
                let strength = if name.contains("weighted") { stored } else if stored > 0 { FULL } else { 0 };
                Part::Producer { strength, strong: Some(Face::Down) }
            }
            // What it reads of the sky (daylight.rs) is in its state.
            "daylight_detector" | "daylight_detector_inverted" => Part::Producer { strength: get(SIGNAL).and_then(|signal| signal.parse().ok()).unwrap_or(0), strong: None },
            "redstone_wire" => Part::Dust { signal: get(SIGNAL).and_then(|signal| signal.parse().ok()).unwrap_or(0) },
            "redstone_torch" | "unlit_redstone_torch" => Part::Torch { lit: name == "redstone_torch", support: torch_support(get("torch_facing_direction")) },
            "unpowered_repeater" | "powered_repeater" => {
                let delay = get("repeater_delay").and_then(|delay| delay.parse().ok()).unwrap_or(0);
                Part::Repeater { lit: name == "powered_repeater", input: side(get("minecraft:cardinal_direction")), delay }
            }
            "unpowered_comparator" | "powered_comparator" => {
                let (lit, input) = (name == "powered_comparator", side(get("minecraft:cardinal_direction")));
                Part::Comparator { lit, input, subtracts: get("output_subtract_bit") == Some("1"), output: if lit { FULL } else { 0 } }
            }
            "observer" => Part::Observer { lit: get("powered_bit") == Some("1"), watch: side(get("minecraft:facing_direction")) },
            "redstone_lamp" | "lit_redstone_lamp" => Part::User(User::Lamp { lit: name == "lit_redstone_lamp" }),
            "hopper" => Part::User(User::Hopper),
            "dispenser" | "dropper" | "crafter" => Part::User(User::Fires { triggered: get("triggered_bit") == Some("1") }),
            "noteblock" => Part::User(User::Note),
            "bell" => Part::User(User::Bell),
            "command_block" | "repeating_command_block" | "chain_command_block" => Part::User(User::Command),
            "structure_block" => Part::User(User::Structure),
            _ if name.ends_with("copper_bulb") => Part::User(User::Bulb { powered: get("powered_bit") == Some("1") }),
            "tripwire_hook" => Part::Producer { strength: on("powered_bit"), strong: get("direction").and_then(hook_support) },
            _ if name.ends_with("lightning_rod") => {
                let support = get("facing_direction").and_then(|face| face.parse().ok()).and_then(Face::from_index).map(opposite);
                Part::Producer { strength: on("powered_bit"), strong: support }
            }
            // Their strength is held by the circuit (`redstone::emit`), not their block.
            "target" | "jukebox" => Part::Producer { strength: 0, strong: None },
            "sculk_sensor" | "calibrated_sculk_sensor" => Part::Producer { strength: 0, strong: Some(Face::Down) },
            "golden_rail" | "activator_rail" | "rail" => Part::User(User::Rail),
            // A minecart on it turns it on (vehicles/rails.rs).
            "detector_rail" => Part::Producer { strength: on("rail_data_bit"), strong: Some(Face::Down) },
            "tnt" => Part::User(User::Tnt),
            "piston" | "sticky_piston" => match get("facing_direction").and_then(crate::blocks::pistons::pushes) {
                Some(front) => Part::User(User::Piston { front }),
                None => Part::Other,
            },
            _ if name.ends_with("piston_arm_collision") || name == "moving_block" => Part::Other,
            _ if name.ends_with("trapdoor") => Part::User(User::Opens(Opens::Trapdoor)),
            _ if name.ends_with("door") => Part::User(User::Opens(Opens::Door)),
            _ if name.ends_with("fence_gate") => Part::User(User::Opens(Opens::Gate)),
            // Java's: a powered lectern gives 15 around and strongly down.
            "lectern" if on("powered_bit") > 0 => Part::Producer { strength: 15, strong: Some(Face::Down) },
            "slime" | "honey_block" | "mob_spawner" | "lectern" => Part::Cutter,
            "glowstone" => Part::Ledge,
            _ if name.ends_with("_slab") && !name.ends_with("double_slab") && get("minecraft:vertical_half") == Some("top") => Part::Ledge,
            _ if conducts(name, state.is_full_cube()) => Part::Conductor,
            _ => Part::Other,
        }
    }

    /// Whether dust under it cannot step up to dust beside it.
    pub(super) fn cuts(self) -> bool {
        matches!(self, Part::Conductor | Part::Cutter)
    }

    /// Whether it takes power from the cell on its `face` side: all but a piston at its front.
    pub(super) fn takes_from(self, face: Face) -> bool {
        !matches!(self, Part::User(User::Piston { front }) if front == face)
    }

    /// Whether the circuit is searched from it.
    pub(super) fn is_source(self) -> bool {
        matches!(self, Part::Producer { .. } | Part::Torch { .. } | Part::Repeater { .. } | Part::Comparator { .. } | Part::Observer { .. })
    }

    /// Whether a change of signal is written into it.
    pub(super) fn is_written(self) -> bool {
        matches!(self, Part::Dust { .. } | Part::User(_) | Part::Torch { .. } | Part::Repeater { .. } | Part::Comparator { .. })
    }
}

/// Whether BDS passes power through a block (research/captures/bds-conductors.txt): the full cubes
/// but for these, and two that are lower.
// TODO: ice and TNT are taken as conductors undrilled.
fn conducts(name: &str, is_full_cube: bool) -> bool {
    const CLEAR: [&str; 5] = ["sea_lantern", "observer", "jukebox", "beacon", "scaffolding"];
    match name {
        "soul_sand" | "mud" => true,
        _ if name.ends_with("glass") || name.ends_with("leaves") || name.ends_with("copper_grate") || CLEAR.contains(&name) => false,
        _ => is_full_cube,
    }
}

pub(super) use crate::blocks::opposite;

/// The side a lever's block is on, by its `lever_direction` (updates.rs `LEVER_ON`).
fn lever_support(direction: &str) -> Option<Face> {
    Some(match direction {
        _ if direction.starts_with("up_") => Face::Down,
        _ if direction.starts_with("down_") => Face::Up,
        "north" => Face::South,
        "south" => Face::North,
        "west" => Face::East,
        "east" => Face::West,
        _ => return None,
    })
}

/// The side a tripwire hook's block is on, behind the way its `direction` faces (0 south, 1 west,
/// 2 north, 3 east).
fn hook_support(direction: &str) -> Option<Face> {
    Some(match direction {
        "0" => Face::North,
        "1" => Face::East,
        "2" => Face::South,
        "3" => Face::West,
        _ => return None,
    })
}

/// The side a torch's block is on: `torch_facing_direction` names it, `top` and `unknown` below.
fn torch_support(facing: Option<&str>) -> Face {
    side(facing)
}

/// The side a direction names, below for any other; the `minecraft:cardinal_direction` of a repeater
/// or comparator is the side it takes power on (research/captures/bds-redstone-repeaters.txt), an
/// observer's `minecraft:facing_direction` the side it watches.
fn side(direction: Option<&str>) -> Face {
    match direction {
        Some("up") => Face::Up,
        Some("north") => Face::North,
        Some("south") => Face::South,
        Some("west") => Face::West,
        Some("east") => Face::East,
        _ => Face::Down,
    }
}
