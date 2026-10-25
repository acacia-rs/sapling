//! Jukeboxes: a click puts the disc in the hand in, with its title told, and the next gives it
//! back (docs/research/block-uses.md, "Jukebox"). The disc is kept in the block's entity
//! (`RecordItem`), so it outlasts a restart and a broken jukebox leaves it; while one is in, the
//! jukebox powers what is around it (docs/research/redstone-blocks.md, "Jukebox").
// TODO: no music is told, as BDS told none; hoppers neither fill nor empty one; the signal lasts
// as long as the disc is in, not as long as its song.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentJukeboxPopup, TextType};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_world::ItemStack;

use super::{fields, level, redstone};
use crate::components::{Mode, Outgoing};
use crate::{drops, inventory};

const JUKEBOX: &str = "minecraft:jukebox";
const ENTITY: &str = "Jukebox";
pub(super) const RECORD: &str = "RecordItem";
const DISC: &str = "minecraft:music_disc_";
/// Above the jukebox's floor, where BDS let the disc go.
const OUT: f32 = 1.71;
/// What a jukebox playing gives the blocks around it.
const PLAYING: u8 = 15;

/// A disc's title, as BDS told it of `13`, and what a comparator reads of a jukebox playing it:
/// measured for thirteen (research/captures/bds-discs.txt), the Java game's for the rest.
const DISCS: &[(&str, &str, u8)] = &[
    ("13", "C418 - 13", 1), ("cat", "C418 - cat", 2), ("blocks", "C418 - blocks", 3), ("chirp", "C418 - chirp", 4), ("far", "C418 - far", 5), ("mall", "C418 - mall", 6),
    ("mellohi", "C418 - mellohi", 7), ("stal", "C418 - stal", 8), ("strad", "C418 - strad", 9), ("ward", "C418 - ward", 10), ("11", "C418 - 11", 11), ("wait", "C418 - wait", 12),
    ("otherside", "Lena Raine - otherside", 14), ("5", "Samuel Åberg - 5", 15), ("pigstep", "Lena Raine - Pigstep", 13), ("relic", "Aaron Cherof - Relic", 14),
    ("creator", "Lena Raine - Creator", 12), ("creator_music_box", "Lena Raine - Creator (Music Box)", 11), ("precipice", "Aaron Cherof - Precipice", 13),
    ("tears", "Amos Roddy - Tears", 10), ("lava_chicken", "Hyper Potions - Lava Chicken", 9),
];

fn disc(item: &str) -> Option<(&'static str, u8)> {
    let short = item.strip_prefix(DISC)?;
    DISCS.iter().find(|(name, ..)| *name == short).map(|(_, title, signal)| (*title, *signal))
}

fn title(item: &str) -> Option<&'static str> {
    disc(item).map(|(title, _)| title)
}

/// The disc in the jukebox at `pos`.
fn held(world: &World, layer: Entity, pos: BlockPos) -> Option<ItemStack> {
    ItemStack::parse(&fields::get(world, layer, pos, RECORD)?)
}

/// What a comparator reads of the block at `pos`: `None` for what is no jukebox.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == JUKEBOX)?;
    Some(held(world, layer, pos).and_then(|held| disc(&held.name)).map_or(0, |(_, signal)| signal))
}

/// What the jukebox at `pos` gives the blocks around it: `PLAYING` while a disc is in.
pub(super) fn power(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    if signal(world, layer, pos).is_some_and(|signal| signal > 0) { PLAYING } else { 0 }
}

fn now_playing(title: &str) -> Text {
    let content = TextContent::JukeboxPopup(TextContentJukeboxPopup { message: "record.nowPlaying".to_owned(), parameters: vec![format!("%{title}")] });
    Text { needs_translation: true, category: TextCategory::Parameters, r#type: TextType::JukeboxPopup, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

/// Puts `disc` in the jukebox at `pos`, or takes what it holds out (`None`).
fn keep(world: &mut World, layer: Entity, pos: BlockPos, disc: Option<&ItemStack>) {
    let fields = disc.zip(level(world, layer)).map(|(disc, level)| Value::Compound(disc.fields(level.registry())));
    fields::set(world, layer, pos, ENTITY, RECORD, fields);
    redstone::emit(world, layer, pos, if disc.is_some() { PLAYING } else { 0 });
    super::container_changed(world, layer, pos);
}

/// A click on the block `name` at `pos`: false when it is no jukebox.
pub(super) fn use_jukebox(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if name != JUKEBOX {
        return false;
    }
    if let Some(disc) = held(world, layer, pos) {
        keep(world, layer, pos, None);
        let [x, y, z] = pos.map(|c| c as f32);
        drops::toss(world, layer, disc, [x + 0.5, y + OUT, z + 0.5]);
        return true;
    }
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some((title, disc)) = held.and_then(|stack| Some((title(&stack.name)?, stack))) else { return true };
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
    }
    keep(world, layer, pos, Some(&ItemStack { count: 1, ..disc }));
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(sapling_net::encode(&now_playing(title)));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_has_its_title_and_nothing_else_has_one() {
        assert_eq!([title("minecraft:music_disc_13"), title("minecraft:music_disc_pigstep"), title("minecraft:stone")], [Some("C418 - 13"), Some("Lena Raine - Pigstep"), None]);
        assert_eq!([disc("minecraft:music_disc_5"), disc("minecraft:music_disc_lava_chicken")].map(|disc| disc.map(|(_, signal)| signal)), [Some(15), Some(9)]);
    }
}
