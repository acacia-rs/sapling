//! The map drills of bds_command_probe: maps made, explored, asked for (MapInfoRequest) and worked
//! at a cartography table (`maps`), and every block's map colour read off a map of a grid the
//! tools/map-colours pack lays (`mapgrid`, BDS only). A map's pixels are printed whole, row by row.

use acacia_bot::Bot;
use acacia_bot::items::SlotRef;
use acacia_bot::proto::nbt::Value;
use acacia_bot::proto::packets::{ClientboundMapItemData, MapInfoRequest};
use acacia_bot::proto::{Packet, RawPacket};

pub const MAPS_SHOWN: &[&str] = &[
    "ClientboundMapItemData", "MapPixels", "InventorySlot", "InventoryContent", "PlayerHotbar", "ItemStackResponse", "LevelSoundEvent", "ContainerOpen", "ContainerClose",
];

/// A map packet's pixels as `MapPixels` lines, one per row: runs of `count*rrggbbaa` (the wire's
/// little-endian ABGR read as bytes). `None` for other packets and for map data with no pixels.
pub fn pixels(packet: &RawPacket) -> Option<Vec<String>> {
    let data = (packet.id == ClientboundMapItemData::ID).then(|| packet.decode::<ClientboundMapItemData>().ok()).flatten()?;
    let (pixels, width) = (data.pixels.as_ref().filter(|pixels| !pixels.is_empty())?, usize::try_from(data.width?).ok()?.max(1));
    let head = format!("MapPixels map={} x={} y={} w={} h={}", data.map_id, data.x_offset?, data.y_offset?, width, data.height?);
    let rows = pixels.chunks(width).enumerate().map(|(row, line)| {
        let mut runs: Vec<(usize, i32)> = Vec::new();
        for &pixel in line {
            match runs.last_mut() {
                Some((count, last)) if *last == pixel => *count += 1,
                _ => runs.push((1, pixel)),
            }
        }
        let runs: Vec<String> = runs.iter().map(|(count, pixel)| format!("{count}*{:08x}", pixel.swap_bytes())).collect();
        format!("MapPixels row {row} {}", runs.join(" "))
    });
    Some(std::iter::once(head).chain(rows).collect())
}

/// The `map_uuid` of every filled map the actor holds in its main inventory.
fn held_maps(actor: &Bot) -> Vec<i64> {
    (0..36u8)
        .filter_map(|slot| SlotRef::Main(slot).stack(actor.state()))
        .filter_map(|stack| match &stack.nbt.as_ref()?.value {
            Value::Compound(fields) => fields.iter().find_map(|(key, value)| match (key.as_str(), value) {
                ("map_uuid", Value::Long(id)) => Some(*id),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

/// Does a map drill's `!` line: `!mapinfo` asks for every held map (MapInfoRequest with no
/// pixels), `!mapinfo1` for the one in hotbar 0 with one client pixel, `!carto x y z [name]`
/// works the map in hotbar 0 with what lies in hotbar 1 (or alone, renamed). False else.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match word {
        "!mapinfo" => {
            for map_id in held_maps(actor) {
                println!("## asked: {map_id} {}", actor.client().send(&MapInfoRequest { map_id, client_pixels: Vec::new() }));
            }
        }
        "!mapinfo1" => {
            let pixel = acacia_bot::proto::packets::MapInfoRequestClientPixelsItem { rgba: 0x11223344, index: 0 };
            if let Some(&map_id) = held_maps(actor).first() {
                println!("## asked: {map_id} {}", actor.client().send(&MapInfoRequest { map_id, client_pixels: vec![pixel] }));
            }
        }
        "!tint" => tint(actor, rest).await,
        "!carto" => {
            let words: Vec<&str> = rest.split(' ').collect();
            let offset: Vec<i32> = words.iter().take(3).filter_map(|word| word.parse().ok()).collect();
            let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
            let (Some([x, y, z]), &[dx, dy, dz]) = (feet, &offset[..]) else { return true };
            let at = [x + dx, y + dy, z + dz];
            let name = words.get(3).copied();
            let additional = SlotRef::Main(1).stack(actor.state()).filter(|stack| !stack.is_empty()).map(|_| SlotRef::Main(1));
            println!("## carto: {:?}", actor.cartography(at, SlotRef::Main(0), additional, name, 1).await);
        }
        _ => return false,
    }
    true
}

/// The first CommandOutput the actor gets within `wait`, as its parameters.
async fn command_output(actor: &mut Bot, wait: std::time::Duration) -> Option<Vec<String>> {
    let end = tokio::time::Instant::now() + wait;
    while let Ok(Some(acacia_bot::BotEvent::Packet(packet))) = tokio::time::timeout_at(end, actor.next()).await {
        if packet.id != acacia_bot::proto::packets::CommandOutput::ID {
            continue;
        }
        if let Ok(output) = packet.decode::<acacia_bot::proto::packets::CommandOutput>() {
            return Some(output.output.iter().flat_map(|line| line.parameters.clone()).collect());
        }
    }
    None
}

/// `!tint biome`: /locate the biome, go there and have the map-colours pack set its tint samples.
async fn tint(actor: &mut Bot, biome: &str) {
    drop(actor.client().command(&format!("locate biome minecraft:{biome}")));
    let found = command_output(actor, std::time::Duration::from_secs(20)).await.unwrap_or_default();
    println!("## locate {biome}: {found:?}");
    let numbers: Vec<&String> = found.iter().filter(|word| word.parse::<f32>().is_ok() || word.as_str() == "~").collect();
    let [x, y, z] = match numbers[..] {
        [x, y, z, ..] => [x.as_str(), y.as_str(), z.as_str()],
        _ => return,
    };
    drop(actor.client().command(&format!("tp @s {x} {} {z}", if y == "~" { "100" } else { y })));
    let _ = command_output(actor, std::time::Duration::from_secs(4)).await;
    let _ = command_output(actor, std::time::Duration::from_secs(3)).await;
    drop(actor.client().command(&format!("scriptevent mapcol:tint {biome}")));
    let _ = command_output(actor, std::time::Duration::from_secs(1)).await;
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn blank(lines: &mut Vec<String>, ticks: usize) {
    lines.extend(std::iter::repeat_n(String::new(), ticks));
}

/// The actor at the world's middle in survival, looking east; the watcher three blocks south.
fn start(tp: &str) -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "time set noon", "weather clear", "gamemode survival @s", "clear @s", "effect @s clear"]);
    lines.extend([tp.to_owned(), "tp @a[name=CmdWatcher] @s".to_owned(), "tp @a[name=CmdWatcher] ~ ~ ~3".to_owned(), String::new()]);
    lines
}

/// Ground of known shape: a stone yard with steps east (one and two up), a ridge to the south,
/// planks, a pool of rising depth and glass; a plain map made there and a locator map; the actor
/// walks off east, out of the map and back; a map made far away; maps asked for; zoomed, locked,
/// cloned and renamed at a cartography table.
pub fn maps() -> Vec<String> {
    let mut lines = start("tp @s 0 ~ 0 -90 30");
    lines.extend(owned([
        "fill ~-8 ~-1 ~-8 ~12 ~-1 ~8 stone", "fill ~-8 ~ ~-8 ~12 ~4 ~8 air", "fill ~3 ~ ~-3 ~3 ~ ~3 stone", "fill ~5 ~ ~-3 ~5 ~1 ~3 stone",
        "fill ~-3 ~ ~4 ~2 ~ ~4 planks", "fill ~-3 ~ ~5 ~2 ~1 ~5 planks", "fill ~7 ~-1 ~-3 ~7 ~-1 ~3 water", "fill ~8 ~-2 ~-3 ~8 ~-1 ~3 water",
        "fill ~9 ~-4 ~-3 ~9 ~-1 ~3 water", "fill ~10 ~-8 ~-3 ~10 ~-1 ~3 water", "fill ~-6 ~ ~-6 ~-4 ~ ~-4 glass", "setblock ~-5 ~ ~-2 oak_leaves",
        "replaceitem entity @s slot.hotbar 0 empty_map 1", "",
    ]));
    lines.extend(owned(["!map", "", "", "", "", "!mapinfo", "", "", "replaceitem entity @s slot.hotbar 0 empty_map 1 2", "", "!map", "", "", ""]));
    lines.extend(owned(["setblock ~1 ~ ~-2 gold_block", "", "", "tp @s ~30 ~ ~", "", "", "", "tp @s ~80 ~ ~", "", "", "", "tp @s ~-110 ~ ~", "", ""]));
    lines.extend(owned(["tp @s 200 ~ -300", "", "", "replaceitem entity @s slot.hotbar 0 empty_map 1", "", "!map", "", "", "", "!mapinfo1", "", "", "tp @s 0 ~ 0", "", ""]));
    lines.extend(owned(["setblock ~2 ~ ~ cartography_table", "replaceitem entity @s slot.hotbar 1 paper 1", "", "!carto 2 0 0", "", "", "!mapinfo", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.hotbar 1 glass_pane 1", "", "!carto 2 0 0", "", "", "replaceitem entity @s slot.hotbar 1 empty_map 1", "", "!carto 2 0 0", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.hotbar 1 air", "", "!carto 2 0 0 Named", "", "", "!mapinfo", "", "", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// The map-colours pack lays every block type on a grid about the actor (BDS only), on the
/// ground's level or one higher (`raised`); a map of it is made and shown whole.
pub fn mapgrid(raised: bool) -> Vec<String> {
    let mut lines = start("tp @s 0 ~ 0 -90 30");
    lines.push(format!("scriptevent mapcol:grid{}", if raised { " raised" } else { "" }));
    blank(&mut lines, 15);
    lines.extend(owned(["replaceitem entity @s slot.hotbar 0 empty_map 1", "", "!map"]));
    blank(&mut lines, 8);
    lines
}

/// Water 1 to 16 deep, two columns a depth, its surface level with a stone platform's; then the
/// actor stands about the map at odd offsets and yaws, to its edge and far off it.
pub fn mapwater() -> Vec<String> {
    let mut lines = start("tp @s 0 ~ 0 -90 30");
    lines.push("fill ~-20 ~ ~8 ~40 ~14 ~20 stone".to_owned());
    lines.extend((1..=16).map(|depth| format!("fill ~{x} ~{bottom} ~10 ~{x1} ~14 ~18 water", x = -18 + 3 * (depth - 1), x1 = -17 + 3 * (depth - 1), bottom = 15 - depth)));
    lines.extend(owned(["", "replaceitem entity @s slot.hotbar 0 empty_map 1", "", "!map"]));
    blank(&mut lines, 3);
    for tp in ["10.3 ~ -7.8 37 0", "-20.7 ~ 15.2 170 0", "-0.2 ~ 0.7 -2 0", "62.6 ~ 0 -170 0", "63.4 ~ -63.6 10 0", "63.6 ~ 0 100 0", "-64.4 ~ 0 -100 0", "330 ~ 0 0 0", "0.5 ~ 0.5 -90 0"] {
        lines.push(format!("tp @s {tp}"));
        blank(&mut lines, 2);
    }
    lines.extend(owned(["!mapinfo", "", "", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// The overworld's biomes, as the vanilla packs name them.
const BIOMES: &str = "bamboo_jungle bamboo_jungle_hills beach birch_forest birch_forest_hills birch_forest_hills_mutated birch_forest_mutated cherry_grove \
cold_beach cold_ocean cold_taiga cold_taiga_hills cold_taiga_mutated dappled_forest deep_cold_ocean deep_dark deep_frozen_ocean deep_lukewarm_ocean deep_ocean \
deep_warm_ocean desert desert_hills desert_mutated dripstone_caves extreme_hills extreme_hills_edge extreme_hills_mutated extreme_hills_plus_trees \
extreme_hills_plus_trees_mutated flower_forest forest forest_hills frozen_ocean frozen_peaks frozen_river grove ice_mountains ice_plains ice_plains_spikes \
jagged_peaks jungle jungle_edge jungle_edge_mutated jungle_hills jungle_mutated legacy_frozen_ocean lukewarm_ocean lush_caves mangrove_swamp meadow mega_taiga \
mega_taiga_hills mesa mesa_bryce mesa_plateau mesa_plateau_mutated mesa_plateau_stone mesa_plateau_stone_mutated mushroom_island mushroom_island_shore ocean \
pale_garden plains redwood_taiga_hills_mutated redwood_taiga_mutated river roofed_forest roofed_forest_mutated savanna savanna_mutated savanna_plateau \
savanna_plateau_mutated snowy_slopes stone_beach stony_peaks sulfur_caves sunflower_plains swampland swampland_mutated taiga taiga_hills taiga_mutated warm_ocean";

/// Every overworld biome visited (a spectator, on a generated world) and its tints printed by the
/// map-colours pack.
pub fn maptints() -> Vec<String> {
    let mut lines = owned(["gamemode spectator @s", ""]);
    lines.extend(BIOMES.split_whitespace().map(|biome| format!("!tint {biome}")));
    lines.push("gamemode creative @s".to_owned());
    lines
}

/// The lines and shown packets of the map drill `name`, if it is one.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "maps" => (maps(), MAPS_SHOWN),
        "mapwater" => (mapwater(), MAPS_SHOWN),
        "maptints" => (maptints(), &["CommandOutput"]),
        "mapgrid" => (mapgrid(false), MAPS_SHOWN),
        "mapgrid-raised" => (mapgrid(true), MAPS_SHOWN),
        _ => return None,
    })
}
