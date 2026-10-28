//! Where a portal is made for a traveller none waits for, and its blocks: BDS's
//! `PortalForcer::force` (144783640), a port of the Java game's `Teleporter.makePortal`
//! (docs/research/nether-dimension.md).

use std::ops::Range;

use sapling_api::BlockPos;

use super::shape::Axis;

/// How far from the traveller a site is looked for, on x and on z.
const REACH: i32 = 16;
/// The lowest and the gap under the top a portal is put at where no site is.
const FLOOR: i32 = 70;
const UNDER_TOP: i32 = 10;

/// What a cell is to a portal site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ground {
    /// Air, or what a portal may replace.
    Empty,
    Solid,
    Other,
}

/// A portal to make: the frame's origin (the inside's lowest cell is beside it), and the
/// direction its inside runs from there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Site {
    pub origin: BlockPos,
    direction: i32,
    /// No site was found: a platform is made to stand it on.
    pub platform: bool,
}

/// `(l6, i3)`: the step along the inside and the step across it, of a direction 0 to 3.
fn steps(direction: i32) -> (i32, i32) {
    let (along, across) = (direction % 2, 1 - direction % 2);
    if direction % 4 >= 2 { (-along, -across) } else { (along, across) }
}

impl Site {
    pub fn axis(&self) -> Axis {
        if steps(self.direction).0 == 0 { Axis::Z } else { Axis::X }
    }

    fn cell(&self, along: i32, across: i32, up: i32) -> BlockPos {
        let (l6, i3) = steps(self.direction);
        let [x, y, z] = self.origin;
        [x + along * l6 + across * i3, y + up, z + along * i3 - across * l6]
    }

    /// The platform's cells under and over the floor, as obsidian (true) or air.
    pub fn platform_cells(&self) -> Vec<(BlockPos, bool)> {
        let mut cells = Vec::new();
        for across in -1..=1 {
            for along in 1..3 {
                for up in -1..3 {
                    cells.push((self.cell(along - 1, across, up), up < 0));
                }
            }
        }
        cells
    }

    /// The netherrack round the platform, under the floor: where BDS puts it if the cell is air,
    /// in its order, a row across at a time.
    pub fn ring_cells(&self) -> Vec<BlockPos> {
        let alongs = |across: i32| if across.abs() == 2 { [0, 1] } else { [-1, 2] };
        (-2..=2).flat_map(|across| alongs(across).map(|along| self.cell(along, across, -1))).collect()
    }

    /// The frame (true) and the inside, in the order BDS lists them: along the frame, each
    /// column from the bottom.
    pub fn frame_cells(&self) -> Vec<(BlockPos, bool)> {
        let mut cells = Vec::new();
        for along in 0..4 {
            for up in -1..4 {
                cells.push((self.cell(along - 1, 0, up), along == 0 || along == 3 || up == -1 || up == 3));
            }
        }
        cells
    }

    /// The inside's lowest cell at its start: the record's corner.
    pub fn corner(&self) -> BlockPos {
        let low = [self.cell(0, 0, 0), self.cell(1, 0, 0)];
        if low[0] <= low[1] { low[0] } else { low[1] }
    }
}

/// The site nearest `at` (the traveller where it arrives) within reach: first one with room for a
/// 3 x 4 footing, then for the frame alone; a platform at a clamped height where neither is.
/// `start` is the direction tried first (BDS draws it, 0 to 3).
pub(crate) fn site(at: [f32; 3], heights: Range<i32>, start: i32, ground: &dyn Fn(BlockPos) -> Ground) -> Site {
    let [px, py, pz] = at;
    let (x0, y0, z0) = (px.floor() as i32, py.floor() as i32, pz.floor() as i32);
    let distance = |x: i32, y: i32, z: i32| (x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2) + (z as f32 + 0.5 - pz).powi(2);
    let mut best: Option<(f32, Site)> = None;
    for wide in [true, false] {
        for x in x0 - REACH..=x0 + REACH {
            for z in z0 - REACH..=z0 + REACH {
                let mut y = heights.end - 1;
                while y >= heights.start {
                    if ground([x, y, z]) == Ground::Empty {
                        while y > heights.start && ground([x, y - 1, z]) == Ground::Empty {
                            y -= 1;
                        }
                        let directions = if wide { start..start + 4 } else { start..start + 2 };
                        for direction in directions {
                            let candidate = Site { origin: [x, y, z], direction: if wide { direction % 4 } else { direction % 2 }, platform: false };
                            if !fits(&candidate, wide, ground) {
                                break;
                            }
                            let d = distance(x, y, z);
                            if best.as_ref().is_none_or(|(least, _)| d < *least) {
                                best = Some((d, candidate));
                            }
                        }
                    }
                    y -= 1;
                }
            }
        }
        if let Some((_, site)) = best {
            return site;
        }
    }
    // The direction is not drawn here: the Java game's starts at 0 and only a site sets it.
    let y = y0.clamp(FLOOR, heights.end - UNDER_TOP);
    Site { origin: [x0, y, z0], direction: 0, platform: true }
}

/// Whether a direction's footing is solid and the room over it empty: three across for `wide`.
fn fits(site: &Site, wide: bool, ground: &dyn Fn(BlockPos) -> Ground) -> bool {
    let across = if wide { 0..3 } else { 0..1 };
    across.flat_map(|across| (0..4).flat_map(move |along| (-1..4).map(move |up| (along, across, up)))).all(|(along, across, up)| {
        let at = if wide { site.cell(along - 1, across, up) } else { narrow(site, along - 1, up) };
        match up < 0 {
            true => ground(at) == Ground::Solid,
            false => ground(at) == Ground::Empty,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_at(y: i32) -> impl Fn(BlockPos) -> Ground {
        move |[_, at, _]| if at == y { Ground::Solid } else if at > y { Ground::Empty } else { Ground::Other }
    }

    #[test]
    fn the_nearest_open_floor_with_the_first_drawn_direction() {
        let site = site([0.0, 66.62, 0.0], 0..128, 0, &floor_at(40));
        assert_eq!((site.origin, site.axis(), site.platform, site.corner()), ([-1, 41, -1], Axis::Z, false, [-1, 41, -1]));
        let frame: Vec<BlockPos> = site.frame_cells().into_iter().map(|(at, _)| at).collect();
        assert_eq!((frame[0], frame[19], frame.len()), ([-1, 40, -2], [-1, 44, 1], 20), "along z, each column from the bottom");
        let turned = super::site([0.0, 66.62, 0.0], 0..128, 1, &floor_at(40));
        assert_eq!((turned.axis(), turned.corner()), (Axis::X, [-1, 41, -1]));
    }

    #[test]
    fn nowhere_to_stand_makes_a_platform_at_70_or_more() {
        let site = site([3.0, 12.0, -4.0], 0..128, 2, &|_| Ground::Empty);
        assert_eq!((site.origin, site.platform, site.axis()), ([3, 70, -4], true, Axis::Z));
        let solid: Vec<BlockPos> = site.platform_cells().into_iter().filter(|(_, solid)| *solid).map(|(at, _)| at).collect();
        assert_eq!(solid.len(), 6);
        assert!(solid.iter().all(|[_, y, _]| *y == 69));
        let ring = site.ring_cells();
        assert_eq!(ring.len(), 10);
        assert!(ring.iter().all(|at| at[1] == 69 && !solid.contains(at)), "round the platform, not on it");
    }
}

/// The narrow pass's cells: along by `(j8, j9)` with no turn; its direction is 0 or 1.
fn narrow(site: &Site, along: i32, up: i32) -> BlockPos {
    let (j8, j9) = (site.direction % 2, 1 - site.direction % 2);
    let [x, y, z] = site.origin;
    [x + along * j8, y + up, z + along * j9]
}
