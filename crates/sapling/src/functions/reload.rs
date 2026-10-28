//! `/reload`: the behavior packs' functions read from disk again, on every shard, as BDS's reload
//! command answers (bdsre 0x143af3fb0, docs/research/server-leftovers.md). `/reload all` reloads
//! the world on BDS, for its host only: a dedicated server has none.

use std::sync::{Arc, Mutex, PoisonError};

use sapling_api::{Command, CommandEnum, Output, Sender, ShardCtx};

use super::Functions;
use crate::packs::{BehaviorPacks, PackError};

/// The functions every shard runs, and the packs they were read from.
#[derive(Clone)]
pub(crate) struct Library {
    current: Arc<Mutex<Arc<Functions>>>,
    origins: Arc<BehaviorPacks>,
}

impl Library {
    pub fn new(packs: BehaviorPacks) -> Self {
        let (functions, origins) = packs.build_keeping_origins();
        Self { current: Arc::new(Mutex::new(Arc::new(functions))), origins: Arc::new(origins) }
    }

    /// The functions now; each shard notices a reload by the pointer changing.
    pub fn functions(&self) -> Arc<Functions> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn reload(&self) -> Result<(), PackError> {
        let functions = Arc::new(self.origins.reread()?.build());
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = functions;
        Ok(())
    }
}

#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum ReloadAll {
    All,
}

#[derive(Command)]
#[command(crate = "crate", name = "reload", description = "Reloads all function and script files from all behavior packs.", permission = "operator")]
pub(crate) struct Reload {
    all: Option<ReloadAll>,
}

impl Command for Reload {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        if self.all.is_some() {
            return out.translate("commands.reload.onlyHost", Vec::new(), false);
        }
        let Some(library) = ctx.state::<super::run::FunctionState>().library() else { return out.translate("commands.reload.error", Vec::new(), false) };
        match library.reload() {
            Ok(()) => out.translate("commands.reload.success", Vec::new(), true),
            Err(error) => {
                tracing::warn!(%error, "/reload: the behavior packs could not be read again");
                out.translate("commands.reload.error", Vec::new(), false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reload_reads_the_pack_folder_again() {
        let dir = std::env::temp_dir().join(format!("sapling-reload-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("functions")).unwrap();
        let manifest = r#"{"header":{"uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f004","version":[1,0,0]},"modules":[{"type":"data"}]}"#;
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
        std::fs::write(dir.join("functions/hello.mcfunction"), "say one\n").unwrap();
        let library = Library::new(BehaviorPacks::new().pack(&dir).unwrap());
        let before = library.functions();
        std::fs::write(dir.join("functions/hello.mcfunction"), "say two\n").unwrap();
        std::fs::write(dir.join("functions/added.mcfunction"), "say three\n").unwrap();
        let reloaded = library.reload();
        std::fs::remove_dir_all(&dir).unwrap();
        reloaded.unwrap();
        let after = library.functions();
        assert_eq!(before.by_name["hello"].lines, ["say one"]);
        assert_eq!((after.by_name["hello"].lines.as_slice(), after.by_name.contains_key("added")), (&["say two".to_owned()][..], true));
    }
}
