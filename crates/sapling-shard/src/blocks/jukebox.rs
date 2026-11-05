//! Jukeboxes: a click puts the disc in the hand in, with its title told, and the next gives it
//! back (docs/research/block-uses.md, "Jukebox"); a hopper puts a disc into an empty one, and the
//! hopper under one takes the disc once its song is over (research/captures/bds-e8juke.txt). The
//! disc is kept in the block's entity (`RecordItem`), so it outlasts a restart and a broken jukebox
//! leaves it; while its song plays (songs.rs), the jukebox powers what is around it
//! (docs/research/redstone-blocks.md, "Jukebox").
// TODO: no music is told, as BDS told none.

mod songs;

pub(crate) use songs::tick as play_jukeboxes;

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
pub(super) const FINISHED: &str = "FinishedRecording";
const TICKS: &str = "TicksPlaying";
const DISC: &str = "minecraft:music_disc_";
/// Above the jukebox's floor, where BDS let the disc go.
const OUT: f32 = 1.71;
/// What a jukebox playing gives the blocks around it.
const PLAYING: u8 = 15;

/// A disc's title, as BDS told it of `13`, what a comparator reads of a jukebox holding it, and its
/// song's length in seconds: signals and lengths of BDS's RecordItem switch 142ef08b0
/// (redstone-blocks.md B14), thirteen's signal measured too (research/captures/bds-discs.txt).
const DISCS: &[(&str, &str, u8, f32)] = &[
    ("13", "C418 - 13", 1, 178.0), ("cat", "C418 - cat", 2, 185.0), ("blocks", "C418 - blocks", 3, 345.0), ("chirp", "C418 - chirp", 4, 185.0),
    ("far", "C418 - far", 5, 174.0), ("mall", "C418 - mall", 6, 197.0), ("mellohi", "C418 - mellohi", 7, 96.0), ("stal", "C418 - stal", 8, 150.0),
    ("strad", "C418 - strad", 9, 188.0), ("ward", "C418 - ward", 10, 251.0), ("11", "C418 - 11", 11, 71.0), ("wait", "C418 - wait", 12, 238.0),
    ("otherside", "Lena Raine - otherside", 14, 195.0), ("5", "Samuel Åberg - 5", 15, 178.0), ("pigstep", "Lena Raine - Pigstep", 13, 149.0),
    ("relic", "Aaron Cherof - Relic", 14, 218.0), ("creator", "Lena Raine - Creator", 12, 176.0), ("creator_music_box", "Lena Raine - Creator (Music Box)", 11, 73.0),
    ("precipice", "Aaron Cherof - Precipice", 13, 299.0), ("tears", "Amos Roddy - Tears", 10, 175.0), ("lava_chicken", "Hyper Potions - Lava Chicken", 9, 134.0),
];

fn disc(item: &str) -> Option<(&'static str, u8, f32)> {
    let short = item.strip_prefix(DISC)?;
    DISCS.iter().find(|(name, ..)| *name == short).map(|(_, title, signal, seconds)| (*title, *signal, *seconds))
}

fn title(item: &str) -> Option<&'static str> {
    disc(item).map(|(title, ..)| title)
}

/// The disc in the jukebox at `pos`.
fn held(world: &World, layer: Entity, pos: BlockPos) -> Option<ItemStack> {
    ItemStack::parse(&fields::get(world, layer, pos, RECORD)?)
}

fn is_finished(world: &World, layer: Entity, pos: BlockPos) -> bool {
    fields::byte(world, layer, pos, FINISHED) == Some(1)
}

pub(crate) fn is_jukebox(world: &World, layer: Entity, pos: BlockPos) -> bool {
    level(world, layer).filter(|level| level.in_height(pos[1])).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).is_some_and(|state| state.name == JUKEBOX)
}

/// What a comparator reads of the block at `pos`: `None` for what is no jukebox.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    is_jukebox(world, layer, pos).then(|| held(world, layer, pos).and_then(|held| disc(&held.name)).map_or(0, |(_, signal, _)| signal))
}

/// What the jukebox at `pos` gives the blocks around it: `PLAYING` while its song plays.
pub(super) fn power(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    if signal(world, layer, pos).is_some_and(|signal| signal > 0) && !is_finished(world, layer, pos) { PLAYING } else { 0 }
}

/// Whether a hopper may put `item` into the jukebox at `pos`: a disc, into an empty one.
pub(crate) fn takes_disc(world: &World, layer: Entity, pos: BlockPos, item: &ItemStack) -> bool {
    disc(&item.name).is_some() && held(world, layer, pos).is_none()
}

/// Puts a hopper's `disc` in the jukebox at `pos`, its song started; nobody is told its title.
pub(crate) fn put_disc(world: &mut World, layer: Entity, pos: BlockPos, disc: &ItemStack) {
    keep(world, layer, pos, Some(disc), fields::set_many);
}

/// The disc of the jukebox at `pos` once its song is over, which a hopper under it may take.
pub(crate) fn finished_disc(world: &World, layer: Entity, pos: BlockPos) -> Option<ItemStack> {
    is_finished(world, layer, pos).then(|| held(world, layer, pos)).flatten()
}

pub(crate) fn take_disc(world: &mut World, layer: Entity, pos: BlockPos) {
    keep(world, layer, pos, None, fields::set_many);
}

fn now_playing(title: &str) -> Text {
    let content = TextContent::JukeboxPopup(TextContentJukeboxPopup { message: "record.nowPlaying".to_owned(), parameters: vec![format!("%{title}")] });
    Text { needs_translation: true, category: TextCategory::Parameters, r#type: TextType::JukeboxPopup, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

type SetFields = fn(&mut World, Entity, BlockPos, &str, &[(&str, Option<Value>)]);

/// Puts `disc` in the jukebox at `pos`, its song started, or takes what it holds out (`None`),
/// its entity kept by `set` (shown or not). One emptied has finished (bds-z2open.txt).
fn keep(world: &mut World, layer: Entity, pos: BlockPos, disc: Option<&ItemStack>, set: SetFields) {
    let record = disc.zip(level(world, layer)).map(|(disc, level)| Value::Compound(disc.fields(level.registry())));
    let finished = Value::Byte(if disc.is_some() { 0 } else { 1 });
    set(world, layer, pos, ENTITY, &[(FINISHED, Some(finished)), (RECORD, record), (TICKS, Some(Value::Long(0)))]);
    redstone::emit(world, layer, pos, if disc.is_some() { PLAYING } else { 0 });
    songs::playing(world, layer, pos, disc.is_some());
    super::container_changed(world, layer, pos);
}

/// The jukeboxes of a column that came from storage, playing: their songs go on.
pub(super) fn arrived(world: &mut World, layer: Entity, playing: &[BlockPos]) {
    for pos in playing {
        songs::resume(world, layer, *pos);
    }
}

/// A click on the block `name` at `pos`: false when it is no jukebox.
pub(super) fn use_jukebox(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if name != JUKEBOX {
        return false;
    }
    // BDS showed the jukebox whole only a tick after a disc went in, and at once and a tick after one came out (bds-z2open.txt).
    if let Some(disc) = held(world, layer, pos) {
        keep(world, layer, pos, None, fields::set_many_quietly);
        super::send::show_whole(world, layer, pos);
        super::shown_again::next_tick(world, layer, pos);
        let [x, y, z] = pos.map(|c| c as f32);
        drops::toss(world, layer, disc, [x + 0.5, y + OUT, z + 0.5]);
        return true;
    }
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some((title, disc)) = held.and_then(|stack| Some((title(&stack.name)?, stack))) else { return true };
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
    }
    keep(world, layer, pos, Some(&ItemStack { count: 1, ..disc }), fields::set_many_quietly);
    super::shown_again::next_tick(world, layer, pos);
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
        assert_eq!([disc("minecraft:music_disc_5"), disc("minecraft:music_disc_lava_chicken")].map(|disc| disc.map(|(_, signal, _)| signal)), [Some(15), Some(9)]);
    }
}
