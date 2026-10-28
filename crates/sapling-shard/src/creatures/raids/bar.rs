//! A raid's boss bar, shown to each player within [`super::REACH`] of its village: `%raid.name`,
//! filling over the wait before a wave and emptying with the wave's health, then
//! `%raid.name - %raid.victory` or `- %raid.defeat` (BDS's strings at 0x14116bbf0), as the Java
//! game's `Raid.updateBossbar`. It takes the raid slot of the player's extra bars, apart from a plugin's.

use bevy_ecs::prelude::*;
use sapling_api::{BossBar, BossBarColour};

use super::{Raid, Stage};
use crate::components::{InLayer, Position, in_world};
use crate::extra_bars::{self, BarKey, ExtraBars};

const NAME: &str = "%raid.name";
const VICTORY: &str = "%raid.name - %raid.victory";
const DEFEAT: &str = "%raid.name - %raid.defeat";

fn bar(world: &World, raid: &Raid, now: u64) -> BossBar {
    let (text, progress) = match raid.stage {
        Stage::Gathering(until) => (NAME, (1.0 - until.saturating_sub(now) as f32 / super::GATHERS as f32).clamp(0.0, 1.0)),
        Stage::Fighting => (NAME, (super::health(world, &raid.raiders) / raid.most).clamp(0.0, 1.0)),
        Stage::Won(_) => (VICTORY, 0.0),
        Stage::Lost(_) => (DEFEAT, 0.0),
    };
    BossBar { text: text.into(), progress, colour: BossBarColour::Red }
}

/// Shows each player the bar of the raid it is in, if any; extra_bars sends only what changed.
pub(super) fn show(world: &mut World, raids: &[Raid], now: u64) {
    for player in in_world(world) {
        let (Some(layer), Some(feet)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|at| at.feet)) else { continue };
        let want = raids.iter().find(|raid| raid.reaches(layer, feet)).map(|raid| bar(world, raid, now));
        if want.is_none() && world.get::<ExtraBars>(player).is_none() {
            continue;
        }
        extra_bars::set(world, player, BarKey::Raid, want.as_ref());
    }
}
