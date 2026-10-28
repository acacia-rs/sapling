//! `/save hold|query|resume`, BDS's backup steps (bedrock_server_how_to.html), answered as its
//! save command answers (bdsre 0x1400b88a0, docs/research/server-leftovers.md). The hold covers
//! the worlds of the shard the command runs on ([`ShardCtx::hold_saves`]). BDS keeps `/save` for
//! its console and owners; here it is for operators.

use sapling_api::{Command, CommandEnum, Output, SaveState, Sender, ShardCtx};

#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum SaveMode {
    Hold,
    Query,
    Resume,
}

#[derive(Command)]
#[command(crate = "crate", name = "save", description = "Control or check how the game saves data to disk.", permission = "operator")]
pub(crate) struct Save {
    mode: SaveMode,
}

impl Command for Save {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        match self.mode {
            SaveMode::Hold if ctx.hold_saves() => out.translate("commands.save.start", Vec::new(), true),
            SaveMode::Hold => out.translate("commands.generic.running", Vec::new(), false),
            SaveMode::Query => query(ctx.save_state(), out),
            SaveMode::Resume if ctx.resume_saves() => out.translate("commands.save-on.success", Vec::new(), true),
            SaveMode::Resume => out.translate("commands.save-on.notDone", Vec::new(), false),
        }
    }
}

/// Ready: the success line, then `path:size` of every file, `, `-joined.
fn query(state: SaveState, out: &mut Output) {
    match state {
        SaveState::Ready(files) => {
            out.translate("commands.save-all.success", Vec::new(), true);
            if !files.is_empty() {
                let listed: Vec<String> = files.iter().map(|file| format!("{}:{}", file.path, file.size)).collect();
                out.translate("%%s", vec![listed.join(", ")], true);
            }
        }
        SaveState::Failed => out.translate("commands.save-all.error", Vec::new(), false),
        SaveState::Idle | SaveState::Saving => out.translate("commands.save-on.notDone", Vec::new(), false),
    }
}

#[cfg(test)]
mod tests {
    use sapling_api::SavedFile;

    use super::*;

    #[test]
    fn a_ready_hold_lists_its_files_with_their_sizes() {
        let mut out = Output::default();
        let files = vec![SavedFile { path: "w/db/CURRENT".into(), size: 16 }, SavedFile { path: "w/level.dat".into(), size: 2349 }];
        query(SaveState::Ready(files), &mut out);
        let messages = out.messages();
        assert_eq!(messages[0].text, "commands.save-all.success");
        assert_eq!(sapling_data::command_text(&messages[1].text, &messages[1].parameters), "w/db/CURRENT:16, w/level.dat:2349");
        let mut out = Output::default();
        query(SaveState::Saving, &mut out);
        assert!(!out.succeeded() && out.messages()[0].text == "commands.save-on.notDone");
    }
}
