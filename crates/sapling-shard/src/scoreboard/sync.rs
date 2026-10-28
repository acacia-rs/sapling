//! Keeps each client showing its world's board, in BDS's packets
//! (research/captures/bds-scoreboard.txt): a player entering a world is shown its slots and their
//! scores after the objectives of the world it left are taken away; one already there is told
//! each [`Change`]. A sidebar of the player's own (hud.rs) hides the board's.

use std::collections::HashMap;

use acacia_proto::packets::{
    RemoveObjective, SetDisplayObjective, SetScore, SetScoreEntriesItem, SetScoreboardIdentity, SetScoreboardIdentityAction, SetScoreboardIdentityEntriesItem, SetScoreEntriesItemContent as Content, SetScoreEntriesItemContentDefault as Default,
    SetScoreEntriesItemContentFakePlayer as FakePlayer, SetScoreEntriesItemContentRemove as Remove, SetScoreEntriesItemEntryType as EntryType,
};
use bevy_ecs::prelude::*;
use sapling_api::{DisplaySlot, Objective, SortOrder};
use sapling_world::ScoreIdentity;

use super::{Board, Change, board_mut, order_byte};
use crate::components::{Identity, InLayer, NetId, Outgoing, in_world};
use crate::hud::Hud;
use crate::tags::{Marks, UniqueId};

/// The world whose board a player's client shows; not carried, so a player from another shard
/// is shown its new world's board anew.
#[derive(Component)]
pub(crate) struct Seen(Entity);

/// Wire ids of the players and entities on the shard, by `UniqueID`.
type WireIds = HashMap<i64, i64>;

fn wire_ids(world: &mut World) -> WireIds {
    let players = world.query::<(&Marks, &NetId)>().iter(world).map(|(marks, id)| (marks.unique_id, id.0 as i64)).collect::<Vec<_>>();
    let entities = world.query::<(&UniqueId, &NetId)>().iter(world).map(|(unique, id)| (unique.0, id.0 as i64)).collect::<Vec<_>>();
    players.into_iter().chain(entities).collect()
}

fn entry(board: &Board, wire: &WireIds, objective: &str, id: i64, score: Option<i32>) -> Option<SetScoreEntriesItem> {
    let objective_name = objective.to_owned();
    let Some(score) = score else {
        return Some(SetScoreEntriesItem { entry_type: EntryType::Remove, entry_type_name: "remove".into(), scoreboard_id: id, content: Content::Remove(Remove { objective_name: Some(objective_name) }) });
    };
    let actor = |unique: &i64| Content::Default(Default { objective_name: objective_name.clone(), score, entity_unique_id: wire.get(unique).copied().unwrap_or(*unique) });
    let (entry_type, name, content) = match board.holder(id)? {
        ScoreIdentity::Player(unique) => (EntryType::Player, "changeplayer", actor(unique)),
        ScoreIdentity::Entity(unique) => (EntryType::Entity, "changeentity", actor(unique)),
        ScoreIdentity::Fake(custom_name) => (EntryType::FakePlayer, "changefakeplayer", Content::FakePlayer(FakePlayer { objective_name, score, custom_name: custom_name.clone() })),
    };
    Some(SetScoreEntriesItem { entry_type, entry_type_name: name.into(), scoreboard_id: id, content })
}

fn cleared(slot: DisplaySlot) -> SetDisplayObjective {
    SetDisplayObjective { display_slot: slot.name().into(), objective_name: String::new(), display_name: String::new(), criteria_name: String::new(), sort_order: 0 }
}

/// Shows `objective` in `slot` with all its scores.
fn show(out: &mut Outgoing, board: &Board, wire: &WireIds, slot: DisplaySlot, objective: &Objective, order: SortOrder) {
    let Objective { name, display_name, criteria } = objective.clone();
    out.push(&SetDisplayObjective { display_slot: slot.name().into(), objective_name: name.clone(), display_name, criteria_name: criteria, sort_order: order_byte(order).into() });
    let entries: Vec<SetScoreEntriesItem> = board.scores_on(&name).into_iter().filter_map(|(id, score)| entry(board, wire, &name, id, Some(score))).collect();
    if !entries.is_empty() {
        out.push(&SetScore { entries });
    }
}

/// What a change is to one player; `own_sidebar` when its sidebar shows its own.
fn tell(out: &mut Outgoing, known: &mut Vec<String>, board: &Board, wire: &WireIds, change: &Change, own_sidebar: bool) {
    match change {
        Change::Removed(name) => {
            out.push(&RemoveObjective { objective_name: name.clone() });
            known.retain(|kept| kept != name);
        }
        Change::Displayed { slot, .. } if *slot == DisplaySlot::Sidebar && own_sidebar => {}
        Change::Displayed { slot, shown, replaced } => {
            if *replaced || shown.is_none() {
                out.push(&cleared(*slot));
            }
            if let Some((objective, order)) = shown {
                show(out, board, wire, *slot, objective, *order);
                known.push(objective.name.clone());
            }
        }
        Change::Score { objective, id, score } => {
            if let Some(entry) = entry(board, wire, objective, *id, *score) {
                out.push(&SetScore { entries: vec![entry] });
            }
        }
    }
}

/// Takes what a client was shown of a board away, then shows it `board`'s slots.
fn enter(out: &mut Outgoing, known: &mut Vec<String>, own_sidebar: bool, board: &Board, wire: &WireIds) {
    for name in known.drain(..) {
        out.push(&RemoveObjective { objective_name: name });
    }
    for (slot, objective, order) in board.displays() {
        if slot != DisplaySlot::Sidebar || !own_sidebar {
            show(out, board, wire, slot, &objective, order);
            known.push(objective.name);
        }
    }
}

/// Sends every player in the world what changed on its board this tick, or the whole board when
/// it is new there; after the handlers, so their changes go out in the tick they made them.
pub(crate) fn send(world: &mut World) {
    let players: Vec<(Entity, Entity)> = in_world(world).into_iter().filter_map(|player| Some((player, crate::dimensions::home(world, world.get::<InLayer>(player)?.0)))).collect();
    for (_, home) in &players {
        board_mut(world, *home);
    }
    let changes: HashMap<Entity, Vec<Change>> = world.query::<(Entity, &mut Board)>().iter_mut(world).map(|(home, mut board)| (home, std::mem::take(&mut board.changes))).collect();
    let entering = players.iter().any(|(player, home)| world.get::<Seen>(*player).is_none_or(|seen| seen.0 != *home));
    if !entering && changes.values().all(Vec::is_empty) {
        return;
    }
    let wire = wire_ids(world);
    for (player, home) in players {
        let seen = world.get::<Seen>(player).map(|seen| seen.0);
        let Some((own, mut known)) = world.get::<Hud>(player).map(|hud| (hud.sidebar.is_some(), hud.board_objectives.clone())) else { continue };
        let Some(board) = world.get::<Board>(home) else { continue };
        let mut packets = Outgoing::default();
        if seen == Some(home) {
            for change in changes.get(&home).into_iter().flatten() {
                tell(&mut packets, &mut known, board, &wire, change, own);
            }
        } else {
            enter(&mut packets, &mut known, own, board, &wire);
        }
        if seen.is_none()
            && let Some(identity) = claim(world, player, home, &wire)
        {
            packets.push(&identity);
        }
        world.entity_mut(player).insert(Seen(home));
        if let Some(mut hud) = world.get_mut::<Hud>(player) {
            hud.board_objectives = known;
        }
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.0.extend(packets.0);
        }
    }
}

/// A player new to the shard takes over the fake holder of its name, as on BDS
/// (research/captures/bds-scoreboard-join.txt): the players seeing the board are told who the id
/// is now, the newcomer after its board showed the fake name. Its scores show as its own from then.
fn claim(world: &mut World, player: Entity, home: Entity, wire: &WireIds) -> Option<SetScoreboardIdentity> {
    let name = world.get::<Identity>(player)?.0.name.clone();
    let unique = world.get::<Marks>(player)?.unique_id;
    let id = world.get_mut::<Board>(home)?.claim(&name, unique)?;
    let entries = vec![SetScoreboardIdentityEntriesItem { scoreboard_id: id, entity_unique_id: Some(wire.get(&unique).copied().unwrap_or(unique)) }];
    let identity = SetScoreboardIdentity { action: SetScoreboardIdentityAction::RegisterIdentity, entries };
    let others: Vec<Entity> = world.query::<(Entity, &Seen)>().iter(world).filter(|(other, seen)| seen.0 == home && *other != player).map(|(other, _)| other).collect();
    for other in others {
        if let Some(mut out) = world.get_mut::<Outgoing>(other) {
            out.push(&identity);
        }
    }
    Some(identity)
}

/// Shows a player the board's sidebar again once its own is gone.
pub(crate) fn show_sidebar(world: &mut World, player: Entity) {
    let Some(seen) = world.get::<Seen>(player).map(|seen| seen.0) else { return };
    let wire = wire_ids(world);
    let Some(board) = world.get::<Board>(seen) else { return };
    let Some((_, objective, order)) = board.displays().into_iter().find(|(slot, ..)| *slot == DisplaySlot::Sidebar) else { return };
    let mut packets = Outgoing::default();
    show(&mut packets, board, &wire, DisplaySlot::Sidebar, &objective, order);
    if let Some(mut hud) = world.get_mut::<Hud>(player) {
        hud.board_objectives.push(objective.name);
    }
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.extend(packets.0);
    }
}
