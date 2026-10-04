//! Sidebar, boss bar, titles and toasts. Packet shapes are BDS 1.26.52's answers to `/scoreboard`,
//! `/title` and a wither (`sapling` example `bds_ui_probe`); what a player was last shown is kept so
//! that a change sends only the difference.

use acacia_proto::packets::{
    BossEvent, BossEventColor, BossEventOverlay, BossEventType, RemoveObjective, SetDisplayObjective, SetScore, SetScoreEntriesItem, SetScoreEntriesItemContent as Content,
    SetScoreEntriesItemContentFakePlayer as FakePlayer, SetScoreEntriesItemContentRemove as Remove, SetScoreEntriesItemEntryType as EntryType, SetTitle, SetTitleType, ToastRequest,
};
use bevy_ecs::prelude::*;
use sapling_api::{BossBar, BossBarColour, Sidebar, Title};

use crate::components::{NetId, Outgoing};

/// The one objective a sidebar is shown as.
const OBJECTIVE: &str = "sapling";
/// `sort_order`: lowest score on top, so line n has score n.
const ASCENDING: i32 = 0;
/// A SetTitle field left alone.
const UNSET: i32 = -1;

#[derive(Component, Default)]
pub(crate) struct Hud {
    sidebar: Option<Sidebar>,
    boss_bar: Option<BossBar>,
}

fn show_objective(title: &str) -> SetDisplayObjective {
    SetDisplayObjective { display_slot: "sidebar".into(), objective_name: OBJECTIVE.into(), display_name: title.to_owned(), criteria_name: "dummy".into(), sort_order: ASCENDING }
}

fn set_line(index: usize, text: &str) -> SetScoreEntriesItem {
    let content = Content::FakePlayer(FakePlayer { objective_name: OBJECTIVE.into(), score: index as i32, custom_name: text.to_owned() });
    SetScoreEntriesItem { entry_type: EntryType::FakePlayer, entry_type_name: "changefakeplayer".into(), scoreboard_id: index as i64 + 1, content }
}

fn remove_line(index: usize) -> SetScoreEntriesItem {
    let content = Content::Remove(Remove { objective_name: Some(OBJECTIVE.into()) });
    SetScoreEntriesItem { entry_type: EntryType::Remove, entry_type_name: "remove".into(), scoreboard_id: index as i64 + 1, content }
}

fn push_scores(out: &mut Outgoing, entries: Vec<SetScoreEntriesItem>) {
    if !entries.is_empty() {
        out.push(&SetScore { entries });
    }
}

/// Sends what turns the sidebar `old` into `new`. A new title rebuilds the board; otherwise only
/// lines that differ are sent, each removed first: the client keeps the text of an entry it knows.
fn send_sidebar(out: &mut Outgoing, old: Option<&Sidebar>, new: Option<&Sidebar>) {
    let old = match (old, new) {
        (Some(old), Some(new)) if old.title == new.title => Some(old),
        (Some(_), _) => {
            out.push(&RemoveObjective { objective_name: OBJECTIVE.into() });
            None
        }
        (None, _) => None,
    };
    let Some(new) = new else { return };
    if old.is_none() {
        out.push(&show_objective(&new.title));
    }
    let before = old.map_or(&[][..], |old| old.lines.as_slice());
    let changed = |index: &usize| before.get(*index) != new.lines.get(*index);
    push_scores(out, (0..before.len()).filter(changed).map(remove_line).collect());
    push_scores(out, (0..new.lines.len()).filter(changed).map(|index| set_line(index, &new.lines[index])).collect());
}

fn colour(colour: BossBarColour) -> BossEventColor {
    match colour {
        BossBarColour::Pink => BossEventColor::Pink,
        BossBarColour::Blue => BossEventColor::Blue,
        BossBarColour::Red => BossEventColor::Red,
        BossBarColour::Green => BossEventColor::Green,
        BossBarColour::Yellow => BossEventColor::Yellow,
        BossBarColour::Purple => BossEventColor::Purple,
        BossBarColour::White => BossEventColor::White,
    }
}

/// The bar hangs on the player's own entity, which is always there to carry it. BDS sends
/// ShowBar again for every change of its wither's bar; so does this.
fn boss_event(id: u64, bar: Option<&BossBar>) -> BossEvent {
    let (r#type, shown) = match bar {
        Some(bar) => (BossEventType::ShowBar, bar.clone()),
        None => (BossEventType::HideBar, BossBar { text: String::new(), progress: 0.0, colour: BossBarColour::Pink }),
    };
    BossEvent { target_entity_id: id as i64, r#type, title: shown.text, filtered_title: String::new(), progress: shown.progress, color: colour(shown.colour), overlay: BossEventOverlay::Progress }
}

fn title_packet(r#type: SetTitleType, text: &str, times: [i32; 3]) -> SetTitle {
    let [fade_in_time, stay_time, fade_out_time] = times;
    SetTitle { r#type, text: text.to_owned(), fade_in_time, stay_time, fade_out_time, xuid: String::new(), platform_online_id: String::new(), filtered_message: String::new() }
}

pub(crate) fn set_sidebar(world: &mut World, entity: Entity, sidebar: Option<&Sidebar>) {
    let shown = sidebar.map(|s| Sidebar { title: s.title.clone(), lines: s.lines.iter().take(Sidebar::MAX_LINES).cloned().collect() });
    let Ok((mut hud, mut out)) = world.query::<(&mut Hud, &mut Outgoing)>().get_mut(world, entity) else { return };
    send_sidebar(&mut out, hud.sidebar.as_ref(), shown.as_ref());
    hud.sidebar = shown;
}

pub(crate) fn set_boss_bar(world: &mut World, entity: Entity, bar: Option<&BossBar>) {
    let Ok((id, mut hud, mut out)) = world.query::<(&NetId, &mut Hud, &mut Outgoing)>().get_mut(world, entity) else { return };
    if hud.boss_bar.as_ref() != bar {
        out.push(&boss_event(id.0, bar));
        hud.boss_bar = bar.cloned();
    }
}

/// Timing, subtitle, then the title that shows them, as `/title` needs them ordered.
pub(crate) fn send_title(out: &mut Outgoing, title: &Title) {
    let ticks = |t: u32| i32::try_from(t).unwrap_or(i32::MAX);
    out.push(&title_packet(SetTitleType::SetDurations, "", [ticks(title.fade_in), ticks(title.stay), ticks(title.fade_out)]));
    if let Some(subtitle) = &title.subtitle {
        out.push(&title_packet(SetTitleType::SetSubtitle, subtitle, [UNSET; 3]));
    }
    out.push(&title_packet(SetTitleType::SetTitle, &title.title, [UNSET; 3]));
}

pub(crate) fn send_actionbar(out: &mut Outgoing, text: &str) {
    out.push(&title_packet(SetTitleType::ActionBarMessage, text, [UNSET; 3]));
}

pub(crate) fn send_toast(out: &mut Outgoing, title: &str, message: &str) {
    out.push(&ToastRequest { title: title.to_owned(), message: message.to_owned() });
}
