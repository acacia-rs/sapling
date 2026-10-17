//! Jukeboxes: a click puts the disc in the hand in, with its title told, and the next gives it
//! back (docs/research/block-uses.md, "Jukebox").
// TODO: the disc is kept in memory, not in the block's entity, so a restart forgets it and a
// broken jukebox keeps it; no music is told, as BDS told none; hoppers neither fill nor empty one.

use std::collections::HashMap;

use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentJukeboxPopup, TextType};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_world::ItemStack;

use crate::components::{Mode, Outgoing};
use crate::{drops, inventory};

const JUKEBOX: &str = "minecraft:jukebox";
const DISC: &str = "minecraft:music_disc_";
/// Above the jukebox's floor, where BDS let the disc go.
const OUT: f32 = 1.71;

/// A disc's title, as BDS told it of `13`, and what a comparator reads of a jukebox playing it:
/// measured for thirteen (research/captures/bds-discs.txt), the Java game's for the rest.
const DISCS: &[(&str, &str, u8)] = &[
    ("13", "C418 - 13", 1), ("cat", "C418 - cat", 2), ("blocks", "C418 - blocks", 3), ("chirp", "C418 - chirp", 4), ("far", "C418 - far", 5), ("mall", "C418 - mall", 6),
    ("mellohi", "C418 - mellohi", 7), ("stal", "C418 - stal", 8), ("strad", "C418 - strad", 9), ("ward", "C418 - ward", 10), ("11", "C418 - 11", 11), ("wait", "C418 - wait", 12),
    ("otherside", "Lena Raine - otherside", 14), ("5", "Samuel Åberg - 5", 15), ("pigstep", "Lena Raine - Pigstep", 13), ("relic", "Aaron Cherof - Relic", 14),
    ("creator", "Lena Raine - Creator", 12), ("creator_music_box", "Lena Raine - Creator (Music Box)", 11), ("precipice", "Aaron Cherof - Precipice", 13),
    ("tears", "Amos Roddy - Tears", 10), ("lava_chicken", "Hyper Potions - Lava Chicken", 9),
];

/// The disc in each jukebox that holds one, by world.
#[derive(Resource, Default)]
pub(crate) struct Discs(HashMap<(Entity, BlockPos), ItemStack>);

fn disc(item: &str) -> Option<(&'static str, u8)> {
    let short = item.strip_prefix(DISC)?;
    DISCS.iter().find(|(name, ..)| *name == short).map(|(_, title, signal)| (*title, *signal))
}

fn title(item: &str) -> Option<&'static str> {
    disc(item).map(|(title, _)| title)
}

/// What a comparator reads of the block at `pos`: `None` for what is no jukebox.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    let level = super::level(world, layer).filter(|level| level.in_height(pos[1]))?;
    level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == JUKEBOX)?;
    let held = world.get_resource::<Discs>().and_then(|discs| discs.0.get(&(layer, pos)));
    Some(held.and_then(|held| disc(&held.name)).map_or(0, |(_, signal)| signal))
}

fn now_playing(title: &str) -> Text {
    let content = TextContent::JukeboxPopup(TextContentJukeboxPopup { message: "record.nowPlaying".to_owned(), parameters: vec![format!("%{title}")] });
    Text { needs_translation: true, category: TextCategory::Parameters, r#type: TextType::JukeboxPopup, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

/// A click on the block `name` at `pos`: false when it is no jukebox.
pub(super) fn use_jukebox(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if name != JUKEBOX {
        return false;
    }
    if let Some(disc) = world.get_resource_or_insert_with(Discs::default).0.remove(&(layer, pos)) {
        let [x, y, z] = pos.map(|c| c as f32);
        drops::toss(world, layer, disc, [x + 0.5, y + OUT, z + 0.5]);
        super::container_changed(world, layer, pos);
        return true;
    }
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some((title, disc)) = held.and_then(|stack| Some((title(&stack.name)?, stack))) else { return true };
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
    }
    world.resource_mut::<Discs>().0.insert((layer, pos), ItemStack { count: 1, ..disc });
    super::container_changed(world, layer, pos);
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
