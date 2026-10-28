//! A raid's boss bar, shown to each player within [`super::REACH`] of its village: `%raid.name`,
//! filling over the wait before a wave and emptying with the wave's health, then
//! `%raid.name - %raid.victory` or `- %raid.defeat` (BDS's strings at 0x14116bbf0), as the Java
//! game's `Raid.updateBossbar`.
// TODO: it hangs on the player's own entity, as a plugin's bar does (hud.rs): the two share it.

use acacia_proto::packets::{BossEvent, BossEventColor, BossEventOverlay, BossEventType};
use bevy_ecs::prelude::*;

use super::{Raid, Stage};
use crate::components::{InLayer, NetId, Outgoing, Position, in_world};

const NAME: &str = "%raid.name";
const VICTORY: &str = "%raid.name - %raid.victory";
const DEFEAT: &str = "%raid.name - %raid.defeat";

/// What a player was last shown: the title and the progress.
#[derive(Component, Clone, PartialEq)]
struct Shown(String, f32);

fn bar(world: &World, raid: &Raid, now: u64) -> Shown {
    match raid.stage {
        Stage::Gathering(until) => Shown(NAME.into(), (1.0 - until.saturating_sub(now) as f32 / super::GATHERS as f32).clamp(0.0, 1.0)),
        Stage::Fighting => Shown(NAME.into(), (super::health(world, &raid.raiders) / raid.most).clamp(0.0, 1.0)),
        Stage::Won(_) => Shown(VICTORY.into(), 0.0),
        Stage::Lost(_) => Shown(DEFEAT.into(), 0.0),
    }
}

fn event(id: u64, r#type: BossEventType, shown: &Shown) -> BossEvent {
    BossEvent { target_entity_id: id as i64, r#type, title: shown.0.clone(), filtered_title: String::new(), progress: shown.1, color: BossEventColor::Red, overlay: BossEventOverlay::Progress }
}

/// Shows each player the bar of the raid it is in, if any, as a change from what it was shown.
pub(super) fn show(world: &mut World, raids: &[Raid], now: u64) {
    for player in in_world(world) {
        let (Some(layer), Some(feet), Some(id)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|at| at.feet), world.get::<NetId>(player).map(|id| id.0)) else { continue };
        let want = raids.iter().find(|raid| raid.reaches(layer, feet)).map(|raid| bar(world, raid, now));
        let had = world.get::<Shown>(player).cloned();
        let events: Vec<BossEvent> = match (&had, &want) {
            (None, None) => continue,
            (None, Some(new)) => vec![event(id, BossEventType::ShowBar, new)],
            (Some(_), None) => vec![event(id, BossEventType::HideBar, &Shown(String::new(), 0.0))],
            (Some(old), Some(new)) => {
                let changes = [(old.0 != new.0, BossEventType::SetBarTitle), (old.1 != new.1, BossEventType::SetBarProgress)];
                changes.into_iter().filter(|(changed, _)| *changed).map(|(_, r#type)| event(id, r#type, new)).collect()
            }
        };
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            events.iter().for_each(|event| out.push(event));
        }
        match want {
            Some(new) => world.entity_mut(player).insert(new),
            None => world.entity_mut(player).remove::<Shown>(),
        };
    }
}
