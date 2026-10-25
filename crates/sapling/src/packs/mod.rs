//! Packs, as BDS takes them from its `resource_packs` and `behavior_packs` folders: resource packs
//! that joining clients download ([`ResourcePacks`]), and behavior packs whose functions the server
//! runs ([`BehaviorPacks`]). Nothing is loaded at run time: packs are read once, when the server is
//! built. Of a behavior pack only its functions are read (docs/NEXT.md lists the rest).

mod archive;
mod behavior;
mod manifest;
mod resource;
mod source;

use std::fs;
use std::path::{Path, PathBuf};

pub use behavior::BehaviorPacks;
pub use resource::ResourcePacks;

use self::source::Source;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}: {reason}")]
    Invalid { path: PathBuf, reason: String },
    #[error("{list} names pack {id}, which none of the pack folders has")]
    Missing { list: PathBuf, id: String },
}

/// The packs a world folder's list (`world_resource_packs.json`, `world_behavior_packs.json`)
/// names, in its order, found in the world's own `folder` or in `packs`. No list: none.
fn world_packs(world: &Path, list: &str, folder: &str, packs: &Path) -> Result<Vec<Source>, PackError> {
    let list = world.join(list);
    let text = match fs::read(&list) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(PackError::Io { path: list, source }),
    };
    let entries: serde_json::Value = serde_json::from_slice(&text).map_err(|e| PackError::Invalid { path: list.clone(), reason: e.to_string() })?;
    let mut found = Vec::new();
    for dir in [world.join(folder), packs.to_owned()].into_iter().filter(|dir| dir.is_dir()) {
        found.extend(source::scan(&dir)?);
    }
    let mut chosen = Vec::new();
    for entry in entries.as_array().into_iter().flatten() {
        let uuid = entry.get("pack_id").and_then(|id| id.as_str()).unwrap_or_default().to_ascii_lowercase();
        let version = manifest::version(entry.get("version"));
        let at = found.iter().position(|pack| pack.manifest.uuid == uuid && version.as_ref().is_none_or(|version| *version == pack.manifest.version));
        let at = at.ok_or_else(|| PackError::Missing { list: list.clone(), id: format!("{uuid}_{}", version.unwrap_or_default()) })?;
        chosen.push(found.swap_remove(at));
    }
    Ok(chosen)
}
