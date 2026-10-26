//! The lists' files: one JSON array each in the server's folder, as BDS keeps `allowlist.json`.

use std::io;
use std::path::Path;

use serde_json::Value;

use super::entries::Entry;

pub(crate) const ALLOWLIST: &str = "allowlist.json";
pub(crate) const PERMISSIONS: &str = "permissions.json";
pub(crate) const BANNED_PLAYERS: &str = "banned-players.json";
pub(crate) const BANNED_IPS: &str = "banned-ips.json";

/// The entries of `dir/file`; none if there is no such file.
pub(crate) fn read<E: Entry>(dir: &Path, file: &str) -> io::Result<Vec<E>> {
    let path = dir.join(file);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let invalid = |what: String| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {what}", path.display()));
    let value: Value = serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?;
    let entries = value.as_array().ok_or_else(|| invalid("not a JSON array".into()))?;
    entries.iter().map(|entry| E::from_json(entry).ok_or_else(|| invalid(format!("unreadable entry {entry}")))).collect()
}

/// Replaces `dir/file` whole: written beside it first, so a crash leaves the old list or the new.
pub(crate) fn write<E: Entry>(dir: &Path, file: &str, entries: &[E]) -> io::Result<()> {
    let array = Value::Array(entries.iter().map(Entry::to_json).collect());
    let text = serde_json::to_string_pretty(&array).map_err(io::Error::other)?;
    std::fs::create_dir_all(dir)?;
    let partial = dir.join(format!("{file}.partial"));
    std::fs::write(&partial, text + "\n")?;
    std::fs::rename(partial, dir.join(file))
}
