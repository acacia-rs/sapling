//! Sidebar, boss bar, titles and toasts. Packet shapes are BDS 1.26.52's answers to `/scoreboard`,
//! `/title` and a wither (`sapling` example `bds_ui_probe`); what a player was last shown is kept so
//! that a change sends only the difference.

use acacia_proto::packets::{
    BossEvent, BossEventColor, BossEventOverlay, BossEventType, RemoveObjective, SetDisplayObjective, SetScore, SetScoreEntriesItem, SetScoreEntriesItemContent as Content,
    SetScoreEntriesItemContentFakePlayer as FakePlayer, SetScoreEntriesItemContentRemove as Remove, SetScoreEntriesItemEntryType as EntryType, SetTitle, SetTitleType, ToastRequest,
};
use bevy_ecs::prelude::*;
use sapling_api::{Animation, BossBar, BossBarColour, EntityId, PlayerId, ScreenAction, Sidebar, Title, TitleAction, TitleBackend};

use crate::components::{NetId, Outgoing};
use crate::handlers::Ctx;

/// The one objective a sidebar is shown as.
const OBJECTIVE: &str = "sapling";
/// `sort_order`: lowest score on top, so line n has score n.
const ASCENDING: i32 = 0;
/// A SetTitle field left alone.
const UNSET: i32 = -1;

#[derive(Component, Default)]
pub(crate) struct Hud {
    /// A sidebar of the player's own, shown over the board's (scoreboard/sync.rs).
    pub(crate) sidebar: Option<Sidebar>,
    boss_bar: Option<BossBar>,
    /// The objectives of its world's board the client was shown, to take away when it leaves.
    pub(crate) board_objectives: Vec<String>,
    /// The inputs `/inputpermission` stopped (screen.rs).
    pub(crate) input_locks: u32,
}

fn show_objective(title: &str) -> SetDisplayObjective {
    SetDisplayObjective { display_slot: "sidebar".into(), objective_name: OBJECTIVE.into(), display_name: title.to_owned(), criteria_name: "dummy".into(), sort_order: ASCENDING }
}

fn set_line(index: usize, text: &str) -> SetScoreEntriesItem {
    let content = Content::FakePlayer(FakePlayer { objective_name: OBJECTIVE.into(), score: index as i32, custom_name: text.to_owned() });
    SetScoreEntriesItem { entry_type: EntryType::FakePlayer, entry_type_name: "changefakeplayer".into(), scoreboard_id: line_id(index), content }
}

fn remove_line(index: usize) -> SetScoreEntriesItem {
    let content = Content::Remove(Remove { objective_name: Some(OBJECTIVE.into()) });
    SetScoreEntriesItem { entry_type: EntryType::Remove, entry_type_name: "remove".into(), scoreboard_id: line_id(index), content }
}

/// Below zero, where the world's board, counting from 1, has none: the client keeps one entry per id.
fn line_id(index: usize) -> i64 {
    -(index as i64 + 1)
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

/// The bar hangs on the player's own entity, which is always there to carry it. Every event type
/// has the same fields on the wire (docs/DESIGN.md "Boss bar"), so each carries the whole bar.
pub(crate) fn boss_event(id: u64, r#type: BossEventType, bar: &BossBar) -> BossEvent {
    BossEvent { target_entity_id: id as i64, r#type, title: bar.text.clone(), filtered_title: String::new(), progress: bar.progress, color: colour(bar.colour), overlay: BossEventOverlay::Progress }
}

/// The events that turn the bar `old` into `new`: a vanilla client ignores ShowBar for a bar it shows.
pub(crate) fn boss_events(old: Option<&BossBar>, new: Option<&BossBar>) -> Vec<(BossEventType, BossBar)> {
    match (old, new) {
        (None, None) => Vec::new(),
        (None, Some(new)) => vec![(BossEventType::ShowBar, new.clone())],
        (Some(_), None) => vec![(BossEventType::HideBar, BossBar { text: String::new(), progress: 0.0, colour: BossBarColour::Pink })],
        (Some(old), Some(new)) => {
            let changes = [(old.text != new.text, BossEventType::SetBarTitle), (old.progress != new.progress, BossEventType::SetBarProgress), (old.colour != new.colour, BossEventType::Texture)];
            changes.into_iter().filter(|(changed, _)| *changed).map(|(_, r#type)| (r#type, new.clone())).collect()
        }
    }
}

fn title_packet(r#type: SetTitleType, text: &str, times: [i32; 3]) -> SetTitle {
    let [fade_in_time, stay_time, fade_out_time] = times;
    SetTitle { r#type, text: text.to_owned(), fade_in_time, stay_time, fade_out_time, xuid: String::new(), platform_online_id: String::new(), filtered_message: String::new() }
}

pub(crate) fn set_sidebar(world: &mut World, entity: Entity, sidebar: Option<&Sidebar>) {
    let shown = sidebar.map(|s| Sidebar { title: s.title.clone(), lines: s.lines.iter().take(Sidebar::MAX_LINES).cloned().collect() });
    let Ok((mut hud, mut out)) = world.query::<(&mut Hud, &mut Outgoing)>().get_mut(world, entity) else { return };
    send_sidebar(&mut out, hud.sidebar.as_ref(), shown.as_ref());
    let gone = hud.sidebar.is_some() && shown.is_none();
    hud.sidebar = shown;
    if gone {
        crate::scoreboard::show_sidebar(world, entity);
    }
}

pub(crate) fn set_boss_bar(world: &mut World, entity: Entity, bar: Option<&BossBar>) {
    let Ok((id, mut hud, mut out)) = world.query::<(&NetId, &mut Hud, &mut Outgoing)>().get_mut(world, entity) else { return };
    for (r#type, shown) in boss_events(hud.boss_bar.as_ref(), bar) {
        out.push(&boss_event(id.0, r#type, &shown));
    }
    hud.boss_bar = bar.cloned();
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

/// The one SetTitle `/title` and `/titleraw` send for `action` (research/captures/bds-messaging.txt).
fn action_packet(action: &TitleAction) -> SetTitle {
    let (r#type, text, times) = match action {
        TitleAction::Title(text) => (SetTitleType::SetTitle, text.clone(), [UNSET; 3]),
        TitleAction::Subtitle(text) => (SetTitleType::SetSubtitle, text.clone(), [UNSET; 3]),
        TitleAction::ActionBar(text) => (SetTitleType::ActionBarMessage, text.clone(), [UNSET; 3]),
        TitleAction::RawTitle(text) => (SetTitleType::SetTitleJson, text.to_json(), [UNSET; 3]),
        TitleAction::RawSubtitle(text) => (SetTitleType::SetSubtitleJson, text.to_json(), [UNSET; 3]),
        TitleAction::RawActionBar(text) => (SetTitleType::ActionBarMessageJson, text.to_json(), [UNSET; 3]),
        TitleAction::Times { fade_in, stay, fade_out } => (SetTitleType::SetDurations, String::new(), [*fade_in, *stay, *fade_out]),
        TitleAction::Clear => (SetTitleType::Clear, String::new(), [UNSET; 3]),
        TitleAction::Reset => (SetTitleType::Reset, String::new(), [UNSET; 3]),
    };
    title_packet(r#type, &text, times)
}

impl TitleBackend for Ctx<'_> {
    fn title_action(&mut self, player: PlayerId, action: &TitleAction) {
        if let Some(mut out) = self.out(player) {
            out.push(&action_packet(action));
        }
    }

    fn screen_action(&mut self, player: PlayerId, action: &ScreenAction) {
        self.act_on_screen(player, action);
    }

    fn input_locks(&self, player: PlayerId) -> u32 {
        self.locked_inputs(player)
    }

    fn set_input_locks(&mut self, player: PlayerId, locks: u32) {
        self.lock_inputs(player, locks);
    }

    fn animate(&mut self, players: &[PlayerId], entities: &[EntityId], animation: &Animation) {
        self.play_animation(players, entities, animation);
    }
}

pub(crate) fn send_toast(out: &mut Outgoing, title: &str, message: &str) {
    out.push(&ToastRequest { title: title.to_owned(), message: message.to_owned() });
}
