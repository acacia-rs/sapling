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

/// A disc's title as BDS told it of `13`; the rest are the game's own.
const TITLES: &[(&str, &str)] = &[
    ("13", "C418 - 13"), ("cat", "C418 - cat"), ("blocks", "C418 - blocks"), ("chirp", "C418 - chirp"), ("far", "C418 - far"), ("mall", "C418 - mall"),
    ("mellohi", "C418 - mellohi"), ("stal", "C418 - stal"), ("strad", "C418 - strad"), ("ward", "C418 - ward"), ("11", "C418 - 11"), ("wait", "C418 - wait"),
    ("otherside", "Lena Raine - otherside"), ("5", "Samuel Åberg - 5"), ("pigstep", "Lena Raine - Pigstep"), ("relic", "Aaron Cherof - Relic"),
    ("creator", "Lena Raine - Creator"), ("creator_music_box", "Lena Raine - Creator (Music Box)"), ("precipice", "Aaron Cherof - Precipice"),
    ("tears", "Amos Roddy - Tears"), ("lava_chicken", "Hyper Potions - Lava Chicken"),
];

/// The disc in each jukebox that holds one, by world.
#[derive(Resource, Default)]
pub(crate) struct Discs(HashMap<(Entity, BlockPos), ItemStack>);

fn title(disc: &str) -> Option<&'static str> {
    let short = disc.strip_prefix(DISC)?;
    TITLES.iter().find(|(name, _)| *name == short).map(|(_, title)| *title)
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
        return true;
    }
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some((title, disc)) = held.and_then(|stack| Some((title(&stack.name)?, stack))) else { return true };
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
    }
    world.resource_mut::<Discs>().0.insert((layer, pos), ItemStack { count: 1, ..disc });
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
    }
}
