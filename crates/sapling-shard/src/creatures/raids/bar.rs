//! A raid's boss bar, shown to each player within [`super::REACH`] of its village: `raid.name`,
//! filling over the wait before a wave and emptying with the wave's health, `- %raid.progress 0`
//! empty for the first seconds after a wave is cleared, then `%raid.name - %raid.victory`, or
//! `- %raid.defeat` with the health left (BDS's strings at 0x14116bbf0; bds-raid.txt, bds-raidlost.txt). It takes the raid slot of the player's extra bars, apart from a plugin's.

use bevy_ecs::prelude::*;
use sapling_api::{BossBar, BossBarColour};

use super::{Raid, Stage};
use crate::components::{InLayer, Position, in_world};
use crate::extra_bars::{self, BarKey, ExtraBars};

// BDS sends the running raid's name without the `%` the others have (research/captures/bds-raid.txt).
const NAME: &str = "raid.name";
const CLEARED: &str = "%raid.name - %raid.progress 0";
const VICTORY: &str = "%raid.name - %raid.victory";
const DEFEAT: &str = "%raid.name - %raid.defeat";

fn bar(world: &World, raid: &Raid, now: u64) -> BossBar {
    let left = || (super::health(world, &raid.raiders) / raid.most).clamp(0.0, 1.0);
    let (text, progress) = match raid.stage {
        Stage::Gathering(until) if until.saturating_sub(now) > super::GATHERS => (CLEARED, 0.0),
        Stage::Gathering(until) => (NAME, (1.0 - until.saturating_sub(now) as f32 / super::GATHERS as f32).clamp(0.0, 1.0)),
        Stage::Fighting => (NAME, left()),
        Stage::Won(_) => (VICTORY, 0.0),
        // The bar keeps the wave's health left after a defeat (bds-raidlost.txt).
        Stage::Lost(_) => (DEFEAT, left()),
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
