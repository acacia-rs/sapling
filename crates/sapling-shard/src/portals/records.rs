//! Finding a portal by its record, as BDS's `PortalForcer::_findPortal` (144786ac0) does: no
//! blocks are looked at.

use sapling_world::PortalRecord;

/// How far a record's cells may be from where a portal is looked for, on x and on z.
const REACH: i32 = 128;

fn cells(record: &PortalRecord) -> impl Iterator<Item = [i32; 3]> + '_ {
    let [x, y, z] = record.corner;
    let (dx, dz) = if record.along_x { (1, 0) } else { (0, 1) };
    (0..i32::from(record.span)).map(move |i| [x + i * dx, y, z + i * dz])
}

/// The record of `dimension` with the cell nearest `at` within reach, and that cell. Distance is
/// counted in whole blocks with y; of equal ones the first kept wins.
pub(crate) fn nearest(portals: &[PortalRecord], dimension: i32, at: [i32; 3]) -> Option<(PortalRecord, [i32; 3])> {
    let mut best: Option<(f32, PortalRecord, [i32; 3])> = None;
    for record in portals.iter().filter(|record| record.dimension == dimension) {
        for cell in cells(record) {
            let [dx, dy, dz] = [cell[0] - at[0], cell[1] - at[1], cell[2] - at[2]];
            if dx.abs() > REACH || dz.abs() > REACH {
                continue;
            }
            let distance = (dx * dx + dy * dy + dz * dz) as f32;
            if best.is_none_or(|(least, ..)| distance < least) {
                best = Some((distance, *record, cell));
            }
        }
    }
    best.map(|(_, record, cell)| (record, cell))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_cell_within_128_on_x_and_z_whatever_the_height() {
        let nether = PortalRecord { dimension: 1, corner: [-15, 34, 7], span: 2, along_x: false };
        let far = PortalRecord { dimension: 1, corner: [200, 34, 7], span: 2, along_x: true };
        let overworld = PortalRecord { dimension: 0, corner: [4, -59, 5], span: 2, along_x: true };
        let portals = [far, nether, overworld];
        // BDS's second overworld portal at x 164 found the Nether's at x -15 (bds-portal-far.txt).
        assert_eq!(nearest(&portals, 1, [20, -58, 0]), Some((nether, [-15, 34, 7])));
        assert_eq!(nearest(&portals, 1, [0, 0, 8]), Some((nether, [-15, 34, 8])));
        assert_eq!(nearest(&portals, 0, [4, 100, 5]), Some((overworld, [4, -59, 5])));
        assert_eq!(nearest(&portals, 1, [-15, 34, 140]), None, "beyond 128 on z");
        assert_eq!(nearest(&portals, 2, [0, 0, 0]), None);
    }
}
