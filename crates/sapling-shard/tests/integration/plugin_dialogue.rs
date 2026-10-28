//! NPC dialogue: opened on an entity, each button press runs its commands with `@initiator` and
//! reaches the callback, and the close comes once (docs/research/ui-events.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{NpcDialogue, NpcDialogueActionType, NpcRequest, NpcRequestActionType, NpcRequestRequestType, SetEntityData};
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use common::asking::ask;
use common::{Harness, all};
use sapling_api::command::{Selected, Selector};
use sapling_api::{Command, Dialogue, Output, PlayerId, Sender, ShardCtx, Tagged};

/// `/mark <selector>`: tags the players it names `said_yes`.
#[derive(Command)]
#[command(crate = "sapling_api", name = "mark")]
struct Mark {
    who: Selector,
}

impl Command for Mark {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        for selected in ctx.select(sender, &self.who) {
            if let Selected::Player(player) = selected {
                ctx.add_tag(Tagged::Player(player.id), "said_yes");
            }
        }
    }
}

fn request(npc: u64, request_type: NpcRequestRequestType, button: i64) -> NpcRequest {
    NpcRequest { runtime_entity_id: npc, request_type, command: String::new(), action_type: NpcRequestActionType::from_raw(button), scene_name: "default".into() }
}

#[test]
fn a_dialogue_runs_its_button_commands_for_the_initiator_and_tells_the_callback() {
    let mut h = Harness::new();
    h.shard.add_command::<Mark>();
    h.enter(1, "Alex");
    let npc = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:villager_v2", [2.5, 65.0, 2.5]).expect("spawned"));
    h.tick(1);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let log = heard.clone();
    h.remote.run(move |ctx| {
        let dialogue = Dialogue::new("Bob").body("Hello").button_running("Yes", ["mark @initiator"]).button("No");
        assert!(ctx.send_dialogue(PlayerId(1), npc, &dialogue, move |_, player, pressed| log.lock().unwrap().push((player.name.clone(), pressed))));
    });
    let sent = h.tick(1);
    let marked = all::<SetEntityData>(&sent).into_iter().find(|data| data.runtime_entity_id == npc.0).expect("the speaker is made an NPC");
    assert_eq!(marked.metadata.iter().map(|item| item.key).collect::<Vec<_>>(), [Key::InteractiveTag, Key::NpcSkinId]);
    let opened = all::<NpcDialogue>(&sent);
    assert_eq!((opened.len(), opened[0].entity_id, opened[0].action_type, opened[0].npc_name.as_str(), opened[0].dialogue.as_str()), (1, npc.0, NpcDialogueActionType::Open, "Bob", "Hello"));
    assert!(opened[0].action_json.contains("\"button_name\":\"Yes\""), "{}", opened[0].action_json);

    h.receive(1, &request(npc.0, NpcRequestRequestType::ExecuteAction, 0));
    h.receive(1, &request(npc.0 + 1, NpcRequestRequestType::ExecuteAction, 1));
    h.tick(1);
    let tags = ask(&mut h, |ctx| ctx.tags(Tagged::Player(PlayerId(1))));
    assert_eq!(tags, ["said_yes"]);
    h.receive(1, &request(npc.0, NpcRequestRequestType::ExecuteClosingCommands, 0));
    h.receive(1, &request(npc.0, NpcRequestRequestType::ExecuteAction, 1));
    h.tick(1);
    let heard = heard.lock().unwrap().clone();
    assert_eq!(heard, [("Alex".to_owned(), Some(0)), ("Alex".to_owned(), None)], "another entity's press and one after the close are not heard");
}

#[test]
fn a_new_dialogue_closes_the_last() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let npc = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:villager_v2", [2.5, 65.0, 2.5]).expect("spawned"));
    let heard = Arc::new(Mutex::new(Vec::new()));
    let (first, second) = (heard.clone(), heard.clone());
    h.remote.run(move |ctx| {
        ctx.send_dialogue(PlayerId(1), npc, &Dialogue::new("a"), move |_, _, pressed| first.lock().unwrap().push(("a", pressed)));
        ctx.send_dialogue(PlayerId(1), npc, &Dialogue::new("b"), move |_, _, pressed| second.lock().unwrap().push(("b", pressed)));
        ctx.close_dialogue(PlayerId(1));
    });
    let sent = h.tick(1);
    let actions: Vec<NpcDialogueActionType> = all::<NpcDialogue>(&sent).into_iter().map(|d| d.action_type).collect();
    assert_eq!(actions, [NpcDialogueActionType::Open, NpcDialogueActionType::Close, NpcDialogueActionType::Open, NpcDialogueActionType::Close]);
    assert_eq!(*heard.lock().unwrap(), [("a", None), ("b", None)]);
}
