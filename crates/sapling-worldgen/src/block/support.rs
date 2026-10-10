//! Which faces of a block hold another block: `Block::canProvideSupport(face, type)` as BDS inlines
//! it (the `minecraft:support` component 148a897e0 first, else `BlockType` vtable 0x90) and
//! `canProvideMultifaceSupport` (vtable 0x98). Classes come from BDS's registry (support_table.rs);
//! see docs/research/worldgen-bds-caves.md, "Support".

use std::collections::HashMap;
use std::sync::LazyLock;

use super::Block;
use super::support_table::{CLASSES, SOLID_BLOCKING};
use crate::structure::template::palette;

/// A block type's support rule; faces are FacingIDs (down, up, north, south, west, east).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The solid-blocking property: every face.
    Solid,
    Open,
    /// The top face only (property bit 0x20000).
    Top,
    Down,
    DownCentre,
    /// Top and bottom, at the centre (walls, panes, bars).
    EndsCentre,
    Slab,
    Trapdoor,
    Chain,
    Cauldron,
    Composter,
    Up,
    /// The support component's shapes.
    Stair,
    Fence,
    /// No support, but multiface blocks hang on them.
    Leaves,
    Azalea,
}

/// `BlockSupportType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Centre = 0,
    Edge = 1,
    Any = 2,
}

const KINDS: [Kind; 3] = [Kind::Centre, Kind::Edge, Kind::Any];
/// Face masks by `Kind`, then for a multiface block (virtual) and for the base class's rule.
const MULTIFACE: usize = 3;
const PLAIN_MULTIFACE: usize = 4;
const BLOCKING: usize = 5;
type Masks = [u8; 6];

fn state<'a>(states: &'a str, key: &str) -> &'a str {
    states.split(',').filter_map(|pair| pair.split_once('=')).find(|&(k, _)| k.trim_start_matches("minecraft:") == key).map_or("", |(_, value)| value)
}

fn set(states: &str, key: &str) -> bool {
    !matches!(state(states, key), "" | "0" | "false" | "bottom")
}

/// 148a89b20: a stair's full faces are the bottom (or top, upside down) and its back.
fn stair(states: &str, face: usize) -> bool {
    match face {
        0 => !set(states, "upside_down_bit"),
        1 => set(states, "upside_down_bit"),
        _ => face == 5 - state(states, "weirdo_direction").parse::<usize>().unwrap_or(0),
    }
}

fn provides(class: Class, states: &str, face: usize, kind: Kind) -> bool {
    let centred = kind != Kind::Edge;
    match class {
        Class::Solid => true,
        Class::Open | Class::Leaves | Class::Azalea => false,
        Class::Top | Class::Up => face == 1,
        Class::Down => face == 0,
        Class::DownCentre => face == 0 && centred,
        Class::EndsCentre | Class::Fence => face < 2 && centred,
        // 146603600
        Class::Slab => face < 2 && (face == 1) == set(states, "vertical_half"),
        // 1425764f0: closed, the face it lies in; open, the side it stands on (table 0x14a7a2f68).
        Class::Trapdoor => match (set(states, "open_bit"), face) {
            (false, 0 | 1) => (face == 1) == set(states, "upside_down_bit"),
            (true, 2..) => face == [4, 5, 2, 3][state(states, "direction").parse::<usize>().unwrap_or(0) & 3],
            _ => false,
        },
        Class::Chain => face < 2 && centred && matches!(state(states, "pillar_axis"), "" | "y"),
        Class::Cauldron => face == 1 && kind != Kind::Centre,
        Class::Composter => !(face == 1 && kind == Kind::Centre),
        Class::Stair => stair(states, face),
    }
}

/// 141a24f10 and its overrides: a fence never, a stair by its shape, else centre and edge both.
fn provides_multiface(class: Class, states: &str, face: usize, plain: bool) -> bool {
    match class {
        Class::Leaves if !plain => true,
        Class::Azalea if !plain => face == 1,
        Class::Fence => false,
        _ => provides(class, states, face, Kind::Centre) && provides(class, states, face, Kind::Edge),
    }
}

fn masks(name: &str, states: &str, classes: &HashMap<&str, Class>, blocking: &[&str]) -> Masks {
    let class = classes.get(name).copied().unwrap_or(Class::Solid);
    let mask = |test: &dyn Fn(usize) -> bool| (0..6).fold(0u8, |bits, face| bits | u8::from(test(face)) << face);
    let mut out = [0; 6];
    for kind in KINDS {
        out[kind as usize] = mask(&|face| provides(class, states, face, kind));
    }
    out[MULTIFACE] = mask(&|face| provides_multiface(class, states, face, false));
    out[PLAIN_MULTIFACE] = mask(&|face| provides_multiface(class, states, face, true));
    out[BLOCKING] = u8::from(class == Class::Solid || blocking.contains(&name));
    out
}

/// Masks of `Block::ALL`, then of the template palette.
static TABLE: LazyLock<(Vec<Masks>, Vec<Masks>)> = LazyLock::new(|| {
    let classes: HashMap<&str, Class> = CLASSES.iter().flat_map(|&(class, names)| names.split_whitespace().map(move |name| (name, class))).collect();
    let blocking: Vec<&str> = SOLID_BLOCKING.split_whitespace().collect();
    let own = Block::ALL.iter().map(|block| masks(block.name(), block.states(), &classes, &blocking)).collect();
    let templates = palette().iter().map(|&(name, states, ..)| masks(name, states, &classes, &blocking)).collect();
    (own, templates)
});

impl Block {
    fn support_masks(self) -> Masks {
        match self {
            Block::Template(i) => TABLE.1[i as usize],
            _ => TABLE.0[self.index()],
        }
    }

    /// `canProvideSupport`: whether `face` of this block holds a block of the kind.
    pub fn supports(self, face: usize, kind: Kind) -> bool {
        self.support_masks()[kind as usize] >> face & 1 != 0
    }

    /// `canProvideMultifaceSupport`: whether a lichen or a vein hangs on `face`.
    pub fn supports_multiface(self, face: usize) -> bool {
        self.support_masks()[MULTIFACE] >> face & 1 != 0
    }

    /// 141a24e50, the base rule called past the vtable (sculk's diagonal moves and shriekers).
    pub fn supports_multiface_plain(self, face: usize) -> bool {
        self.support_masks()[PLAIN_MULTIFACE] >> face & 1 != 0
    }

    /// The solid-blocking property (`BlockType` +0x130 bit 0x40000).
    pub fn is_solid_blocking(self) -> bool {
        self.support_masks()[BLOCKING] != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_the_registry() {
        let named = |name: &str| Block::ALL.iter().copied().find(|b| b.name() == name).unwrap();
        assert!(named("deepslate").supports(3, Kind::Any) && named("deepslate").supports_multiface(0));
        assert!(!named("oak_leaves").supports(1, Kind::Any) && named("oak_leaves").supports_multiface(4));
        assert!(!named("bamboo").supports(1, Kind::Any) && !named("pointed_dripstone").supports_multiface(0));
        assert!(named("oak_fence").supports(1, Kind::Any) && !named("oak_fence").supports_multiface(1));
        assert!(named("oak_leaves").is_solid_blocking() && !named("glow_lichen").is_solid_blocking());
    }

    #[test]
    fn shaped_blocks_read_their_states() {
        assert!(stair("upside_down_bit=1,weirdo_direction=2", 1) && stair("weirdo_direction=2", 3) && !stair("weirdo_direction=2", 2));
        assert!(provides(Class::Slab, "minecraft:vertical_half=top", 1, Kind::Any) && provides(Class::Slab, "", 0, Kind::Any));
        assert!(!provides(Class::Slab, "minecraft:vertical_half=top", 0, Kind::Any) && !provides(Class::Slab, "", 4, Kind::Any));
        assert!(provides(Class::Trapdoor, "direction=1,open_bit=1", 5, Kind::Any) && provides(Class::Trapdoor, "", 0, Kind::Any));
    }

    /// A block the registry table lacks would pass as solid.
    #[test]
    fn no_open_block_is_left_solid() {
        let all = Block::ALL.iter().copied().chain(Block::templates());
        let mut missing: Vec<&str> = all.filter(|b| b.is_solid_blocking() && !b.blocks_motion()).map(Block::name).collect();
        missing.dedup();
        assert!(missing.is_empty(), "not in the support table: {missing:?}");
    }
}
