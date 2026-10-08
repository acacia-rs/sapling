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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// A lever, button, plate or block of redstone giving `strength`, strongly into the block on
    /// `strong` (the one it is attached to; under a plate).
    Producer { strength: u8, strong: Option<Face> },
    Dust { signal: u8 },
    /// Attached on `support`; powers the block above strongly.
    Torch { lit: bool, support: Face },
    User(User),
    /// A solid block: powered by what points into it, and passing that on.
    Conductor,
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
            "redstone_wire" => Part::Dust { signal: get(SIGNAL).and_then(|signal| signal.parse().ok()).unwrap_or(0) },
            "redstone_torch" | "unlit_redstone_torch" => Part::Torch { lit: name == "redstone_torch", support: torch_support(get("torch_facing_direction")) },
            "redstone_lamp" | "lit_redstone_lamp" => Part::User(User::Lamp { lit: name == "lit_redstone_lamp" }),
            "hopper" => Part::User(User::Hopper),
            _ if name.ends_with("trapdoor") => Part::User(User::Opens(Opens::Trapdoor)),
            _ if name.ends_with("door") => Part::User(User::Opens(Opens::Door)),
            _ if name.ends_with("fence_gate") => Part::User(User::Opens(Opens::Gate)),
            // TODO: BDS asks each block's `redstone_conductivity` (research C5); this takes every solid one.
            _ if state.is_solid() => Part::Conductor,
            _ => Part::Other,
        }
    }

    /// Whether the circuit is searched from it.
    pub(super) fn is_source(self) -> bool {
        matches!(self, Part::Producer { .. } | Part::Torch { .. })
    }

    /// Whether a change of signal is written into it.
    pub(super) fn is_written(self) -> bool {
        matches!(self, Part::Dust { .. } | Part::User(_) | Part::Torch { .. })
    }
}

pub(super) fn opposite(face: Face) -> Face {
    match face {
        Face::Down => Face::Up,
        Face::Up => Face::Down,
        Face::North => Face::South,
        Face::South => Face::North,
        Face::West => Face::East,
        Face::East => Face::West,
    }
}

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

/// The side a torch's block is on: `torch_facing_direction` names it, `top` and `unknown` below.
fn torch_support(facing: Option<&str>) -> Face {
    match facing {
        Some("north") => Face::North,
        Some("south") => Face::South,
        Some("west") => Face::West,
        Some("east") => Face::East,
        _ => Face::Down,
    }
}
