//! A key used on a vault (`VaultBlockActor::tryInsertKey`, docs/research/spawners.md "Vault").

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};

use super::super::eject::{roll, sound};
use super::super::fields::{Fields, compounds, longs};
use super::super::{centre, now};
use super::{REMEMBERED, UNLOCKING, corner, enter, load, shown, stack_value};
use crate::components::Mode;

/// A refused key is heard no more often than this.
const FAIL_GAP: i64 = 15;

/// When each vault last refused a key aloud; BDS keeps it unsaved too.
#[derive(Resource, Default)]
struct Refusals(HashMap<(Entity, BlockPos), i64>);

/// A click with `held` on an active vault: its key, from a player not yet rewarded, unlocks it, and
/// the player is shown the vault's cells and the clicked face's (`beside`) first; a refusal is only
/// heard. `Some(spent)` on an active vault, `None` otherwise.
pub(in crate::spawners) fn use_key(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, beside: BlockPos, held: &str) -> Option<bool> {
    let mut vault = load(world, layer, pos, Fields::at(world, layer, pos)?)?;
    if vault.state != "active" {
        return None;
    }
    let unique_id = world.get::<crate::tags::Marks>(player).map_or(0, |marks| marks.unique_id);
    let refusal = if held != vault.config.key { Some("vault.insert_item_fail") } else if vault.rewarded().contains(&unique_id) { Some("vault.reject_rewarded_player") } else { None };
    let now = now(world);
    if let Some(refusal) = refusal {
        let mut refusals = world.get_resource_or_insert_with(Refusals::default);
        if refusals.0.get(&(layer, pos)).is_none_or(|last| now - last >= FAIL_GAP) {
            refusals.0.insert((layer, pos), now);
            sound(world, layer, pos, refusal, centre(pos), -1);
        }
        return Some(false);
    }
    let table = vault.config.loot_table.clone();
    let items: Vec<Value> = roll(world, &table).into_iter().filter_map(|(name, count)| stack_value(world, layer, name, count)).collect();
    if items.is_empty() {
        return Some(false);
    }
    vault.data.set("total_ejections_needed", Value::Long(items.len() as i64));
    vault.display(items.last().cloned());
    vault.data.set("items_to_eject", compounds(items));
    vault.data.set("state_updating_resumes_at", Value::Long(now + UNLOCKING));
    let mut rewarded = vault.rewarded();
    rewarded.push(unique_id);
    let over = rewarded.len().saturating_sub(REMEMBERED);
    rewarded.drain(..over);
    vault.data.set("rewarded_players", longs(&rewarded));
    crate::blocks::about_click_first(world, player, pos, beside, |world| {
        let heard = enter(world, layer, pos, &mut vault, "unlocking");
        crate::blocks::keep_entity(world, layer, pos, &vault.into_value());
        if let Some(heard) = heard {
            sound(world, layer, pos, heard, corner(pos), -1);
        }
    });
    shown::next_tick(world, layer, pos, false, None);
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    Some(!creative)
}
