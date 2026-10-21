//! A gateway a kill opens, as BDS wrote the first (research/captures/bds-dragon-fight.txt): Java's
//! shape round its cell, x then y then z, bedrock above and below in a cross, a cell of bedrock
//! two away, air round the gateway itself.

use sapling_api::BlockPos;

use super::podium::BEDROCK;

pub(super) const GATEWAY: (&str, &str) = ("minecraft:end_gateway", "");
const AIR: (&str, &str) = ("minecraft:air", "");

/// The gateway's writes round `centre`, in BDS's order.
pub(super) fn cells([cx, cy, cz]: BlockPos) -> Vec<(BlockPos, (&'static str, &'static str))> {
    let mut cells = Vec::new();
    for x in cx - 1..=cx + 1 {
        for y in cy - 2..=cy + 2 {
            for z in cz - 1..=cz + 1 {
                let (on_x, on_y, on_z, far) = (x == cx, y == cy, z == cz, (y - cy).abs() == 2);
                let block = match () {
                    _ if on_x && on_y && on_z => GATEWAY,
                    _ if on_y => AIR,
                    _ if far && on_x && on_z => BEDROCK,
                    _ if (on_x || on_z) && !far => BEDROCK,
                    _ => AIR,
                };
                cells.push(([x, y, z], block));
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_gateway_is_wrapped_as_bds_wrapped_the_first() {
        let cells = super::cells([0, 75, -96]);
        let bedrock: Vec<_> = cells.iter().filter(|(_, (name, _))| name.ends_with("bedrock")).map(|(pos, _)| *pos).collect();
        assert_eq!(bedrock.len(), 12);
        assert_eq!(&bedrock[..3], &[[-1, 74, -96], [-1, 76, -96], [0, 73, -96]]);
        assert!(cells.iter().any(|(pos, (name, _))| *pos == [0, 75, -96] && name.ends_with("end_gateway")));
    }
}
