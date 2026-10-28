//! Maps filled, drawn and asked for (maps.rs; research/captures/bds-maps.txt).

use crate::common;
use acacia_proto::RawPacket;
use acacia_proto::nbt::Value as Nbt;
use acacia_proto::packets::{ClientboundMapItemData, InventoryContent, LevelSoundEvent, MapInfoRequest};
use acacia_proto::types::{ItemV4Extra, MapDecorationType, WindowIDVarint};
use common::all;
use common::shooting::{click_air, ready};
use sapling_api::GameMode;

/// The tag of the item in Alex's first slot, as the last inventory in `sent` showed it.
fn first_tag(sent: &[RawPacket]) -> Vec<(String, Nbt)> {
    let windows = all::<InventoryContent>(sent);
    let inventory = windows.iter().rev().find(|window| window.window_id == WindowIDVarint::Inventory).expect("the inventory");
    let ItemV4Extra::Default(Some(extra)) = &inventory.input[0].extra else { return Vec::new() };
    match extra.nbt.as_ref().map(|tag| &tag.nbt.value) {
        Some(Nbt::Compound(fields)) => fields.iter().map(|(key, value)| (key.to_string(), value.clone())).collect(),
        _ => Vec::new(),
    }
}

/// The pixel at `(x, z)` of a map packet carrying the whole map.
fn pixel(packet: &ClientboundMapItemData, x: usize, z: usize) -> u32 {
    packet.pixels.as_ref().expect("pixels")[z * 128 + x] as u32
}

#[test]
fn an_empty_map_fills_and_draws_the_platform_with_both_players_on_it() {
    let mut h = ready(GameMode::Survival, &[("minecraft:empty_map", 1, 0)], 0.0);
    h.receive(1, &click_air());
    let made = h.tick_all();
    let (to_alex, to_steve) = (made.to(1).to_vec(), made.to(2).to_vec());
    let created = all::<ClientboundMapItemData>(&to_alex).pop().expect("the map's first packet");
    assert_eq!(all::<ClientboundMapItemData>(&to_steve).len(), 1, "everyone in the world is told");
    let id = created.map_id;
    assert_eq!(created.included_in, Some((id..id + 5).collect()));
    assert_eq!((created.scale, created.origin.x, created.origin.z, created.tracked_objects.is_none()), (Some(0), 0, 0, true));
    assert_eq!(first_tag(&to_alex), [("map_name_index".to_owned(), Nbt::Int(1)), ("map_uuid".to_owned(), Nbt::Long(id))]);

    let blank = h.tick_all().to(1).to_vec();
    let whole = all::<ClientboundMapItemData>(&blank).pop().expect("the blank map");
    assert_eq!((whole.width, whole.height, whole.decorations.as_deref().map(<[_]>::len)), (Some(128), Some(128), Some(0)));
    assert!(whole.pixels.as_ref().unwrap().iter().all(|pixel| *pixel == 0));
    let sounds: Vec<String> = all::<LevelSoundEvent>(&blank).into_iter().map(|sound| sound.sound_id).collect();
    assert_eq!(sounds, ["ui.cartography_table.take_result"]);

    let marked = all::<ClientboundMapItemData>(h.tick_all().to(1)).pop().expect("the players on it");
    let marks: Vec<(MapDecorationType, u8, u8, i32)> = marked.decorations.unwrap().iter().map(|mark| (mark.r#type.clone(), mark.x, mark.y, mark.color_abgr)).collect();
    // Steve joined last, so he comes first; Alex holds the map.
    assert_eq!(marks, [(MapDecorationType::MarkerWhite, 242, 242, -11_206_657), (MapDecorationType::MarkerWhite, 2, 2, -1)]);
    assert!(all::<ClientboundMapItemData>(h.tick_all().to(1)).is_empty(), "markers go every fifth tick");

    let drawn: Vec<ClientboundMapItemData> = (0..25).flat_map(|_| all::<ClientboundMapItemData>(h.tick_all().to(1))).filter(|packet| packet.pixels.is_some()).collect();
    let drawn = drawn.first().expect("a drawing within 21 ticks");
    let [x0, z0] = [drawn.x_offset.unwrap() as usize, drawn.y_offset.unwrap() as usize];
    let at = |x: usize, z: usize| drawn.pixels.as_ref().unwrap()[(z - z0) * drawn.width.unwrap() as usize + x - x0] as u32;
    assert_eq!(at(64, 64), 0xFF60_6060, "stone at the normal shade");

    h.receive(1, &MapInfoRequest { map_id: id, client_pixels: Vec::new() });
    let answer = all::<ClientboundMapItemData>(h.tick_all().to(1));
    assert!(answer[0].included_in.is_some() && answer[1].width == Some(128));
    assert_eq!(pixel(&answer[1], 64, 64), 0xFF60_6060);
}
