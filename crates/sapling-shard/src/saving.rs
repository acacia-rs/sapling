//! Backup holds (`/save hold|query|resume`, sapling-api's `SaveState`) over every world of the
//! shard, stepped as BDS's FileArchiver steps its state (bdsre 0x1400b88a0, docs/research/
//! server-leftovers.md): a hold writes everything out and stops writes; a query once the writes
//! landed flushes storage and lists the files; resume lets writes go on.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use bevy_ecs::prelude::*;
use sapling_api::{SaveState, SavedFile};

use crate::components::{Layer, Settings};
use crate::{actors, blocks, inventory, scoreboard};

/// Files BDS's own LevelDB keeps for itself, which no copy needs.
const NOT_COPIED: [&str; 3] = ["LOCK", "LOG", "LOG.old"];

/// The shard's hold; none is [`SaveState::Idle`].
#[derive(Resource)]
struct Hold(SaveState);

/// Hands every world what the shard keeps outside their columns: players' items, actors, block
/// ticks, the scoreboard, maps, and the level's difficulty and tick distance.
pub(crate) fn write_back_all(world: &mut World) {
    inventory::save_all(world);
    actors::write_back(world);
    blocks::write_back_ticks(world);
    scoreboard::keep_all(world);
    crate::maps::keep_all(world);
    let config = &world.resource::<Settings>().0;
    let (difficulty, tick_distance) = (config.difficulty as i32, config.tick_distance);
    for mut layer in world.query::<&mut Layer>().iter_mut(world) {
        let level = layer.world.level_mut();
        (level.difficulty, level.chunk_tick_range) = (Some(difficulty), Some(tick_distance));
    }
}

/// `/save hold`; false while a hold is saving or ready.
pub(crate) fn hold(world: &mut World) -> bool {
    if matches!(state(world), SaveState::Saving | SaveState::Ready(_)) {
        return false;
    }
    write_back_all(world);
    world.query::<&mut Layer>().iter_mut(world).for_each(|mut layer| layer.world.hold_saves());
    set(world, SaveState::Saving);
    true
}

/// `/save query`: a saving hold whose writes all landed is flushed and listed here.
pub(crate) fn query(world: &mut World) -> SaveState {
    let saving = matches!(state(world), SaveState::Saving);
    if saving && world.query::<&Layer>().iter(world).all(|layer| layer.world.writes_landed()) {
        let next = match snapshot(world) {
            Ok(files) => SaveState::Ready(files),
            Err(error) => {
                tracing::error!(%error, "a backup hold could not flush its worlds: saving goes on");
                world.query::<&mut Layer>().iter_mut(world).for_each(|mut layer| layer.world.release_saves());
                SaveState::Failed
            }
        };
        set(world, next);
    }
    state(world)
}

/// `/save resume`; false unless the hold was ready.
pub(crate) fn resume(world: &mut World) -> bool {
    if !matches!(state(world), SaveState::Ready(_)) {
        return false;
    }
    world.query::<&mut Layer>().iter_mut(world).for_each(|mut layer| layer.world.release_saves());
    set(world, SaveState::Idle);
    true
}

fn state(world: &World) -> SaveState {
    world.get_resource::<Hold>().map_or(SaveState::Idle, |hold| hold.0.clone())
}

fn set(world: &mut World, next: SaveState) {
    world.insert_resource(Hold(next));
}

/// Flushes every world's storage and lists the files of their folders.
fn snapshot(world: &mut World) -> Result<Vec<SavedFile>, sapling_world::StorageError> {
    let mut folders = BTreeSet::new();
    for layer in world.query::<&Layer>().iter(world) {
        layer.world.flush_storage()?;
        folders.extend(layer.world.storage_folder());
    }
    let mut files = Vec::new();
    for folder in folders {
        let name = folder.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        list(&folder, &PathBuf::from(name), &mut files)?;
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn list(dir: &Path, shown: &Path, files: &mut Vec<SavedFile>) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let (kind, name) = (entry.file_type()?, entry.file_name());
        let shown = shown.join(&name);
        if kind.is_dir() {
            list(&entry.path(), &shown, files)?;
        } else if !NOT_COPIED.iter().any(|skipped| name == *skipped) {
            let path = shown.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
            files.push(SavedFile { path, size: entry.metadata()?.len() });
        }
    }
    Ok(())
}
