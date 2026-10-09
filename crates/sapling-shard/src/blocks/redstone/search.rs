//! Where the power of a circuit's sources reaches, by BDS's scene-graph search (docs/research/
//! redstone.md 3 and 4): strongest first, a strength per cell and what it is for.
// Dust powers a side block by its shape (no connection across it), not by the path the power came
// by as BDS does: the two agree on every rig of research/captures/bds-redstone-shapes.txt (C8).

use std::collections::{BinaryHeap, HashMap, HashSet};

use sapling_api::{BlockPos, Face};

use super::parts::{FULL, Part, opposite};

const ACROSS: [Face; 4] = [Face::North, Face::South, Face::West, Face::East];

/// How a cell is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Via {
    Dust,
    /// A block a source points into: it powers dust too.
    Strong,
    /// A block dust points into or lies on.
    Weak,
    User,
    /// Into a torch from the block it is on, or into the back of a repeater.
    Input,
    /// Into the side of a repeater from one that is lit.
    Lock,
    /// Into the side of a comparator.
    Side,
}

/// What a search found: the strongest signal each cell gets, what each torch, repeater and
/// comparator is given and from which source, the repeaters another holds as they are, and the
/// strongest signal at a comparator's sides.
#[derive(Default, Debug)]
pub(super) struct Power {
    pub(super) at: HashMap<BlockPos, u8>,
    pub(super) inputs: HashMap<BlockPos, (u8, BlockPos)>,
    pub(super) locked: HashSet<BlockPos>,
    pub(super) sides: HashMap<BlockPos, u8>,
}

struct Search<'a, F: FnMut(BlockPos) -> Part> {
    part: &'a mut F,
    best: HashMap<(Via, BlockPos), u8>,
    queue: BinaryHeap<(u8, Via, BlockPos, BlockPos)>,
    power: Power,
}

impl<F: FnMut(BlockPos) -> Part> Search<'_, F> {
    fn offer(&mut self, via: Via, pos: BlockPos, strength: u8, origin: BlockPos) {
        let best = self.best.entry((via, pos)).or_insert(0);
        if strength > *best {
            *best = strength;
            self.queue.push((strength, via, pos, origin));
        }
    }

    /// Offers `strength` to what `pos` holds, reached from `from` on its `face` side: dust and
    /// users always, a solid block when `strong` points into it, a torch standing on `from`, a
    /// repeater whose back `from` is at, or whose side when `from` is a repeater or comparator
    /// that `locks`, a comparator's back or side.
    fn reach(&mut self, from: BlockPos, face: Face, strength: u8, origin: BlockPos, strong: bool, to_dust: bool, locks: bool) {
        let pos = face.neighbour(from);
        match (self.part)(pos) {
            Part::Repeater { input, .. } if face == opposite(input) => self.offer(Via::Input, pos, strength, origin),
            Part::Repeater { input, .. } if locks && ACROSS.contains(&face) && face != input => self.offer(Via::Lock, pos, strength, origin),
            Part::Comparator { input, .. } if face == opposite(input) => self.offer(Via::Input, pos, strength, origin),
            Part::Comparator { input, .. } if ACROSS.contains(&face) && face != input => self.offer(Via::Side, pos, strength, origin),
            Part::Dust { .. } if to_dust => self.offer(Via::Dust, pos, strength, origin),
            Part::User(_) => self.offer(Via::User, pos, strength, origin),
            Part::Conductor if strong => self.offer(Via::Strong, pos, strength, origin),
            Part::Torch { support, .. } if support.neighbour(pos) == from => self.offer(Via::Input, pos, strength, origin),
            _ => {}
        }
    }

    fn from_source(&mut self, pos: BlockPos, part: Part) {
        match part {
            Part::Producer { strength, strong } if strength > 0 => {
                for face in Face::ALL {
                    self.reach(pos, face, strength, pos, strong == Some(face), true, false);
                }
            }
            Part::Torch { lit: true, support } => {
                for face in Face::ALL.into_iter().filter(|face| *face != support) {
                    self.reach(pos, face, FULL, pos, face == Face::Up, true, false);
                }
            }
            Part::Repeater { lit: true, input, .. } => self.reach(pos, opposite(input), FULL, pos, true, true, true),
            Part::Observer { lit: true, watch } => self.reach(pos, opposite(watch), FULL, pos, true, true, false),
            Part::Comparator { input, output, .. } if output > 0 => self.reach(pos, opposite(input), output, pos, true, true, true),
            _ => {}
        }
    }

    /// Whether dust at `dust` joins what is on its `face` side: dust there, a step up or down, or a
    /// source. A block over the lower dust that [`Part::cuts`] stops a step, a [`Part::Ledge`] a step down.
    fn connects(&mut self, dust: BlockPos, face: Face) -> bool {
        let beside = face.neighbour(dust);
        let is_dust = |part: Part| matches!(part, Part::Dust { .. });
        let beside_part = (self.part)(beside);
        let up = !(self.part)(Face::Up.neighbour(dust)).cuts() && is_dust((self.part)(Face::Up.neighbour(beside)));
        let down = !beside_part.cuts() && (self.part)(Face::Down.neighbour(dust)) != Part::Ledge && is_dust((self.part)(Face::Down.neighbour(beside)));
        let joins_repeater = matches!(beside_part, Part::Repeater { input, .. } | Part::Comparator { input, .. } if input == face || input == opposite(face));
        let joins_observer = matches!(beside_part, Part::Observer { watch, .. } if watch == face);
        matches!(beside_part, Part::Dust { .. } | Part::Producer { .. } | Part::Torch { .. }) || joins_repeater || joins_observer || up || down
    }

    fn from_dust(&mut self, dust: BlockPos, strength: u8, origin: BlockPos) {
        let below = Face::Down.neighbour(dust);
        let below_part = (self.part)(below);
        match below_part {
            Part::Conductor => self.offer(Via::Weak, below, strength, origin),
            Part::User(_) => self.offer(Via::User, below, strength, origin),
            _ => {}
        }
        let up_free = !(self.part)(Face::Up.neighbour(dust)).cuts();
        let connected = ACROSS.map(|face| self.connects(dust, face));
        for (index, face) in ACROSS.into_iter().enumerate() {
            let beside = face.neighbour(dust);
            let beside_part = (self.part)(beside);
            let next = strength - 1;
            match beside_part {
                Part::Dust { .. } => self.offer(Via::Dust, beside, next, origin),
                Part::Repeater { input, .. } | Part::Comparator { input, .. } if input == opposite(face) => self.offer(Via::Input, beside, strength, origin),
                Part::Comparator { input, .. } if input != face => self.offer(Via::Side, beside, strength, origin),
                _ => {}
            }
            let (above, under) = (Face::Up.neighbour(beside), Face::Down.neighbour(beside));
            if up_free && matches!((self.part)(above), Part::Dust { .. }) {
                self.offer(Via::Dust, above, next, origin);
            }
            if !beside_part.cuts() && below_part != Part::Ledge && matches!((self.part)(under), Part::Dust { .. }) {
                self.offer(Via::Dust, under, next, origin);
            }
            // North and south are 0 and 1, west and east 2 and 3: the other pair lies across.
            let across = if index < 2 { [2, 3] } else { [0, 1] };
            if across.iter().all(|side| !connected[*side]) {
                match beside_part {
                    Part::Conductor => self.offer(Via::Weak, beside, strength, origin),
                    Part::User(_) => self.offer(Via::User, beside, strength, origin),
                    _ => {}
                }
            }
        }
    }

    fn from_block(&mut self, block: BlockPos, strength: u8, origin: BlockPos, strong: bool) {
        for face in Face::ALL {
            self.reach(block, face, strength, origin, false, strong, false);
        }
    }

    fn run(mut self, sources: &[(BlockPos, Part)]) -> Power {
        for (pos, part) in sources {
            self.from_source(*pos, *part);
        }
        while let Some((strength, via, pos, origin)) = self.queue.pop() {
            if self.best.get(&(via, pos)) != Some(&strength) {
                continue;
            }
            let at = self.power.at.entry(pos).or_insert(0);
            *at = (*at).max(strength);
            match via {
                Via::Dust => self.from_dust(pos, strength, origin),
                Via::Strong => self.from_block(pos, strength, origin, true),
                Via::Weak => self.from_block(pos, strength, origin, false),
                Via::Lock => drop(self.power.locked.insert(pos)),
                Via::Side => {
                    let side = self.power.sides.entry(pos).or_insert(0);
                    *side = (*side).max(strength);
                }
                Via::Input => {
                    let input = self.power.inputs.entry(pos).or_insert((0, origin));
                    if strength > input.0 {
                        *input = (strength, origin);
                    }
                }
                Via::User => {}
            }
        }
        self.power
    }
}

/// Searches out from `sources` (a torch as it is about to be), reading cells by `part`.
pub(super) fn search(sources: &[(BlockPos, Part)], part: &mut impl FnMut(BlockPos) -> Part) -> Power {
    Search { part, best: HashMap::new(), queue: BinaryHeap::new(), power: Power::default() }.run(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(cells: &[(BlockPos, Part)]) -> impl FnMut(BlockPos) -> Part + '_ {
        move |pos| cells.iter().find(|(at, _)| *at == pos).map_or(Part::Other, |(_, part)| *part)
    }

    const LAMP: Part = Part::User(super::super::parts::User::Lamp { lit: false });
    const DUST: Part = Part::Dust { signal: 0 };

    #[test]
    fn dust_fades_by_one_a_block_and_reaches_a_lamp_15_away_not_16() {
        // As research/captures/bds-redstone.txt: a block of redstone at x 2, dust from x 3.
        let block = ([2, 0, 0], Part::Producer { strength: FULL, strong: None });
        for (length, lit) in [(15, true), (16, false)] {
            let mut cells: Vec<(BlockPos, Part)> = (3..3 + length).map(|x| ([x, 0, 0], DUST)).collect();
            cells.extend([block, ([3 + length, 0, 0], LAMP)]);
            let power = search(&[block], &mut world(&cells));
            assert_eq!(power.at.get(&[3, 0, 0]), Some(&15));
            assert_eq!(power.at.get(&[3 + length, 0, 0]).is_some_and(|s| *s > 0), lit, "{length} dust");
        }
    }

    #[test]
    fn a_lever_powers_through_its_block_but_a_redstone_block_does_not() {
        let lever = ([0, 1, 0], Part::Producer { strength: FULL, strong: Some(Face::Down) });
        let cells = [lever, ([0, 0, 0], Part::Conductor), ([1, 0, 0], LAMP), ([5, 0, 0], Part::Conductor), ([6, 0, 0], LAMP)];
        let block = ([4, 0, 0], Part::Producer { strength: FULL, strong: None });
        let mut cells = cells.to_vec();
        cells.push(block);
        let power = search(&[lever, block], &mut world(&cells));
        assert_eq!((power.at.get(&[1, 0, 0]), power.at.get(&[6, 0, 0])), (Some(&15), None));
    }

    #[test]
    fn a_torch_reads_the_block_it_is_on_and_its_own_power_back_names_it() {
        let torch = ([0, 1, 0], Part::Torch { lit: true, support: Face::Down });
        let lever = ([1, 0, 0], Part::Producer { strength: FULL, strong: Some(Face::West) });
        let cells = [torch, lever, ([0, 0, 0], Part::Conductor)];
        let power = search(&[torch, lever], &mut world(&cells));
        assert_eq!(power.inputs.get(&[0, 1, 0]), Some(&(15, [1, 0, 0])));
    }

    #[test]
    fn dust_steps_up_past_glass_but_not_slime_and_down_stone_but_not_glowstone() {
        // As research/captures/bds-conductors.txt: `over` above the dust by the block of redstone, `under` the dust a step up.
        let block = ([0, 0, 0], Part::Producer { strength: FULL, strong: None });
        let stepped = |over: Part, under: Part, from_top: bool| {
            let (low, high) = ([1, 0, 0], [2, 1, 0]);
            let source = if from_top { ([3, 1, 0], block.1) } else { block };
            let cells = [source, (low, DUST), ([1, 1, 0], over), ([2, 0, 0], under), (high, DUST)];
            let power = search(&[source], &mut world(&cells));
            power.at.get(if from_top { &low } else { &high }).copied()
        };
        assert_eq!(stepped(Part::Other, Part::Conductor, false), Some(14));
        assert_eq!(stepped(Part::Ledge, Part::Conductor, false), Some(14), "glass and glowstone cut nothing");
        assert_eq!(stepped(Part::Cutter, Part::Conductor, false), None);
        assert_eq!(stepped(Part::Conductor, Part::Conductor, false), None);
        assert_eq!(stepped(Part::Other, Part::Conductor, true), Some(14));
        assert_eq!(stepped(Part::Other, Part::Ledge, true), None);
    }
}
