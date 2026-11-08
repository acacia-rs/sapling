//! What a map tells besides its pixels: the players on it and where they face, and the packets
//! carrying them (docs/research/maps.md "Markers" and "Packets").

use acacia_proto::packets::ClientboundMapItemData;
use acacia_proto::types::{BlockCoordinates, MapDecoration, MapDecorationType, TrackedObject, TrackedObjectType, Vec3i};
use sapling_world::MapPin;

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

/// An explorer map's structure: a block tracked at `(x, 64, z)` and its marker, as BDS pinned them
/// (bds-at1maps.txt: taiga village (-984, 136) on the map at (-832, 192) drawn at (180, 228)).
pub(crate) fn pin(map: &MapData, r#type: MapDecorationType, [x, z]: [i32; 2]) -> (TrackedObject, MapDecoration) {
    let half = |at: i32, centre: i32| place(at as f32, centre, map.scale).map_or(0, |(half, _)| half) as u8;
    let tracked = TrackedObject { r#type: TrackedObjectType::Block, entity_unique_id: None, block_position: Some(BlockCoordinates { x, y: PIN_Y, z }) };
    let mark = MapDecoration { r#type, rotation: PIN_ROTATION, x: half(x, map.centre[0]), y: half(z, map.centre[1]), label: String::new(), color_abgr: HOLDER };
    (tracked, mark)
}

/// A pin as the map's record keeps it; `None` for one tracking no block.
pub(super) fn kept_pin((tracked, mark): &(TrackedObject, MapDecoration)) -> Option<MapPin> {
    let at = tracked.block_position.as_ref()?;
    Some(MapPin { kind: mark.r#type.clone().to_raw() as i32, rotation: mark.rotation.into(), x: (mark.x as i8).into(), y: (mark.y as i8).into(), block: [at.x, at.y, at.z] })
}

/// The pin a map's record kept.
pub(super) fn restored_pin(kept: &MapPin) -> (TrackedObject, MapDecoration) {
    let [x, y, z] = kept.block;
    let tracked = TrackedObject { r#type: TrackedObjectType::Block, entity_unique_id: None, block_position: Some(BlockCoordinates { x, y, z }) };
    let mark = MapDecoration { r#type: MapDecorationType::from_raw(kept.kind.into()), rotation: kept.rotation as u8, x: kept.x as u8, y: kept.y as u8, label: String::new(), color_abgr: HOLDER };
    (tracked, mark)
}

const PIN_Y: i32 = 64;
const PIN_ROTATION: u8 = 8;

/// The pin, then the tracked players and their markers, newest player first (BDS's order changed
/// between captures). A pinned map marks a player past [`FAR`] small on its edge, as BDS's explorer
/// maps did 560 pixels off.
pub(super) fn mark(map: &MapData, mut seen: Vec<Seen>) -> (Vec<TrackedObject>, Vec<MapDecoration>) {
    seen.sort_by_key(|player| std::cmp::Reverse(player.id));
    let (mut tracked, mut marks): (Vec<_>, Vec<_>) = map.pin.iter().cloned().unzip();
    let pinned = map.pin.is_some();
    for player in seen {
        let axis = |at: f32, centre: i32| place(at, centre, map.scale).map(|(half, on)| (half, on, false)).or_else(|| pinned.then(|| (if at < centre as f32 { -128 } else { 127 }, false, true)));
        let (Some((x, x_on, x_far)), Some((z, z_on, z_far))) = (axis(player.feet[0], map.centre[0]), axis(player.feet[2], map.centre[1])) else { continue };
        tracked.push(TrackedObject { r#type: TrackedObjectType::Entity, entity_unique_id: Some(player.id), block_position: None });
        let r#type = match (x_on && z_on, x_far || z_far) {
            (true, _) => MapDecorationType::MarkerWhite,
            (false, false) => MapDecorationType::SquareWhite,
            (false, true) => MapDecorationType::SmallSquareWhite,
        };
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

    #[test]
    fn a_pin_is_kept_as_bds_saved_it() {
        // bds-ax1maps.txt's world: the taiga village map's one decoration.
        let kept = MapPin { kind: 21, rotation: 8, x: -76, y: -28, block: [-984, 64, 136] };
        let pin = restored_pin(&kept);
        assert_eq!((pin.1.r#type.clone(), pin.1.x, pin.1.y), (MapDecorationType::VillageTaiga, 180, 228));
        assert_eq!(kept_pin(&pin), Some(kept));
    }
}
