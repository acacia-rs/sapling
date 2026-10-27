//! What a map tells besides its pixels: the players on it and where they face, and the packets
//! carrying them (docs/research/maps.md "Markers" and "Packets").

use acacia_proto::packets::ClientboundMapItemData;
use acacia_proto::types::{MapDecoration, MapDecorationType, TrackedObject, TrackedObjectType, Vec3i};

use super::MapData;
use super::render::SIZE;

/// The holder's own marker is white, everyone else's this blue (ABGR, as BDS sent them).
const HOLDER: i32 = -1;
const OTHER: i32 = -11_206_657;
/// Past this many pixels from the middle a player is no longer marked.
const FAR: i32 = 319;
/// Markers inside this many pixels of the middle sit where the player is; past it, on the edge.
const INSIDE: i32 = 62;

/// A marker's place on one axis, in half pixels from the middle: `None` past [`FAR`].
fn place(at: f32, centre: i32, scale: u8) -> Option<(i8, bool)> {
    let pixels = ((at - centre as f32) / (1 << scale) as f32).round() as i32;
    let on_map = (-SIZE / 2..SIZE / 2).contains(&pixels);
    let half = match pixels {
        p if p > INSIDE => 127,
        p if p < -INSIDE => -128,
        p => (2 * p) as i8,
    };
    (pixels.abs() <= FAR).then_some((half, on_map))
}

/// Sixteenths of a turn, as BDS rounds a yaw: away from zero by half a sixteenth, then cut.
fn rotation(yaw: f32) -> u8 {
    ((yaw + if yaw < 0.0 { -8.0 } else { 8.0 }) * 16.0 / 360.0) as i32 as i8 as u8
}

/// A player the map may mark: its unique id, feet, yaw, and whether it holds the map.
pub(super) struct Seen {
    pub id: i64,
    pub feet: [f32; 3],
    pub yaw: f32,
    pub holder: bool,
}

/// The tracked players and their markers, newest player first (BDS's order changed between captures).
pub(super) fn mark(map: &MapData, mut seen: Vec<Seen>) -> (Vec<TrackedObject>, Vec<MapDecoration>) {
    seen.sort_by_key(|player| std::cmp::Reverse(player.id));
    let mut tracked = Vec::new();
    let mut marks = Vec::new();
    for player in seen {
        let (Some((x, x_on)), Some((z, z_on))) = (place(player.feet[0], map.centre[0], map.scale), place(player.feet[2], map.centre[1], map.scale)) else { continue };
        tracked.push(TrackedObject { r#type: TrackedObjectType::Entity, entity_unique_id: Some(player.id), block_position: None });
        let r#type = if x_on && z_on { MapDecorationType::MarkerWhite } else { MapDecorationType::SquareWhite };
        let color_abgr = if player.holder { HOLDER } else { OTHER };
        marks.push(MapDecoration { r#type, rotation: rotation(player.yaw), x: x as u8, y: z as u8, label: String::new(), color_abgr });
    }
    (tracked, marks)
}

/// What every map packet carries.
fn base(map: &MapData) -> ClientboundMapItemData {
    ClientboundMapItemData {
        map_id: map.id,
        dimension: map.dimension as u8,
        locked: map.locked,
        origin: Vec3i { x: map.centre[0], y: 0, z: map.centre[1] },
        included_in: None,
        scale: Some(map.scale as i8),
        tracked_objects: None,
        decorations: None,
        width: None,
        height: None,
        x_offset: None,
        y_offset: None,
        pixels: None,
    }
}

/// The map's first packet, naming the ids of its zoom levels.
pub(super) fn creation(map: &MapData) -> ClientboundMapItemData {
    ClientboundMapItemData { included_in: Some((0..5).map(|level| map.id + level).collect()), ..base(map) }
}

/// The last told markers, with the pixels of `area` (`[x, z, width, height]`) when there is one.
pub(super) fn update(map: &MapData, area: Option<[usize; 4]>) -> ClientboundMapItemData {
    let mut packet = ClientboundMapItemData { tracked_objects: Some(map.tracked.clone()), decorations: Some(map.marks.clone()), ..base(map) };
    if let Some([x, z, width, height]) = area {
        let rows = (z..z + height).flat_map(|row| map.pixels[row * SIZE as usize + x..][..width].iter().map(|&pixel| pixel as i32));
        packet.pixels = Some(rows.collect());
        (packet.width, packet.height, packet.x_offset, packet.y_offset) = (Some(width as i32), Some(height as i32), Some(x as i32), Some(z as i32));
    }
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_round_and_clamp_as_bds_placed_them() {
        assert_eq!(place(0.5, 0, 0), Some((2, true)));
        assert_eq!(place(3.5, 0, 0), Some((8, true)));
        assert_eq!(place(63.0, 0, 0), Some((127, true)));
        assert_eq!(place(-64.0, 0, 0), Some((-128, true)));
        assert_eq!(place(100.0, 0, 0), Some((127, false)));
        assert_eq!(place(330.0, 0, 0), None);
        assert_eq!(place(200.0, 256, 0), Some((-112, true)));
    }

    #[test]
    fn yaws_turn_into_sixteenths() {
        let turned: Vec<u8> = [-90.0, 37.0, 170.0, -170.0, 10.0, 100.0, -2.0].into_iter().map(rotation).collect();
        assert_eq!(turned, [252, 2, 7, 249, 0, 4, 0]);
    }
}
