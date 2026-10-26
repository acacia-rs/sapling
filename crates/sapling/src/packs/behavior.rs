//! The behavior packs whose functions the server runs (crate::functions).

use std::path::Path;
use std::sync::Arc;

use super::source::{self, Source};
use super::{PackError, world_packs};
use crate::functions::{Function, Functions};

const FUNCTIONS: &str = "functions/";
const EXTENSION: &str = ".mcfunction";
const TICK_LIST: &str = "functions/tick.json";

/// Behavior packs, of which sapling runs the functions: `/function`, `/schedule` and those each
/// pack's `functions/tick.json` lists, every tick. A function a later pack has under the same
/// name replaces the earlier one's.
///
/// ```no_run
/// # fn run() -> Result<(), sapling::packs::PackError> {
/// let packs = sapling::packs::BehaviorPacks::new().world("worlds/Bedrock level", "behavior_packs")?;
/// let builder = sapling::Server::builder().behavior_packs(packs);
/// # Ok(()) }
/// ```
#[derive(Debug, Default)]
pub struct BehaviorPacks {
    functions: Functions,
}

impl BehaviorPacks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one pack: a folder with a `manifest.json`, or a `.mcpack` or `.zip` of one.
    pub fn pack(self, path: impl AsRef<Path>) -> Result<Self, PackError> {
        Ok(self.with(vec![source::open(path.as_ref())?]))
    }

    /// Adds every pack in `folder`, by file name.
    pub fn folder(self, folder: impl AsRef<Path>) -> Result<Self, PackError> {
        Ok(self.with(source::scan(folder.as_ref())?))
    }

    /// Adds the packs a world folder applies, as BDS does: those its `world_behavior_packs.json`
    /// names, from the world's `behavior_packs` folder or `packs`. The first named comes last, so
    /// that its functions win.
    // TODO: which pack's function BDS keeps when two have one of the same name was not checked.
    pub fn world(self, world: impl AsRef<Path>, packs: impl AsRef<Path>) -> Result<Self, PackError> {
        let mut listed = world_packs(world.as_ref(), "world_behavior_packs.json", "behavior_packs", packs.as_ref())?;
        listed.reverse();
        Ok(self.with(listed))
    }

    fn with(mut self, packs: Vec<Source>) -> Self {
        for pack in packs {
            if !pack.manifest.has_module("data") {
                tracing::warn!(pack = pack.manifest.id(), "not a behavior pack: no data module");
                continue;
            }
            for (path, text) in &pack.files {
                let Some(name) = path.strip_prefix(FUNCTIONS).and_then(|name| name.strip_suffix(EXTENSION)) else { continue };
                match Function::parse(name, &String::from_utf8_lossy(text)) {
                    Some(function) => {
                        self.functions.by_name.insert(name.to_owned(), Arc::new(function));
                    }
                    None => tracing::warn!(pack = pack.manifest.id(), function = name, "an empty function is not loaded"),
                }
            }
            if let Some(list) = pack.file(TICK_LIST) {
                self.functions.tick.extend(tick_list(list).unwrap_or_else(|| {
                    tracing::warn!(pack = pack.manifest.id(), "tick.json has no list of names under \"values\"");
                    Vec::new()
                }));
            }
        }
        self
    }

    /// The names of the functions, sorted.
    pub fn function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.functions.by_name.keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn build(mut self) -> Functions {
        self.functions.tick.retain(|name| {
            let known = self.functions.by_name.contains_key(name);
            if !known {
                tracing::warn!(function = name, "listed in tick.json, but there is no such function");
            }
            known
        });
        self.functions
    }
}

fn tick_list(json: &[u8]) -> Option<Vec<String>> {
    let json = json.strip_prefix(b"\xef\xbb\xbf").unwrap_or(json);
    let root: serde_json::Value = serde_json::from_slice(json).ok()?;
    root.get("values")?.as_array()?.iter().map(|name| name.as_str().map(str::to_owned)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::archive;

    #[test]
    fn functions_come_by_their_path_and_tick_json_names_some() {
        let dir = std::env::temp_dir().join(format!("sapling-behavior-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = br#"{"header":{"uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f003","version":[1,0,0]},"modules":[{"type":"data"}]}"#;
        let files: [(&str, &[u8]); 5] = [
            ("manifest.json", &manifest[..]),
            ("functions/probe/hello.mcfunction", b"# hi\nsay hello\n"),
            ("functions/deep/a/b.mcfunction", b"say deep"),
            ("functions/probe/empty.mcfunction", b""),
            ("functions/tick.json", br#"{"values":["probe/hello","nosuch"]}"#),
        ];
        let zip = archive::write(&files.map(|(name, data)| (name.to_owned(), data.to_vec())));
        std::fs::write(dir.join("probe.mcpack"), zip).unwrap();
        let packs = BehaviorPacks::new().pack(dir.join("probe.mcpack"));
        std::fs::remove_dir_all(&dir).unwrap();
        let packs = packs.unwrap();
        assert_eq!(packs.function_names(), ["deep/a/b", "probe/hello"]);
        let functions = packs.build();
        assert_eq!((functions.tick.as_slice(), functions.by_name["probe/hello"].lines.as_slice()), (&["probe/hello".to_owned()][..], &["say hello".to_owned()][..]));
    }
}
