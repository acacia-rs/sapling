//! Vaults (BDS `VaultBlockActor`, docs/research/spawners.md "Vault"): `vault_state` goes inactive,
//! active while a player not yet rewarded is near, unlocking when such a player uses the key, and
//! ejecting, one item each [`STEP`] ticks; each player is rewarded once (`rewarded_players`).

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode, Item};
use sapling_world::ItemStack;

use super::eject::{event, item, roll, sound};
use super::fields::{Fields, as_longs, compounds, longs};
use super::{Near, centre, dice, distance, now, property, set_state};
use crate::components::{Layer, Mode};

const REWARD: &str = "loot_tables/chests/trial_chambers/reward.json";
const OMINOUS_REWARD: &str = "loot_tables/chests/trial_chambers/reward_ominous.json";
const KEY: &str = "minecraft:trial_key";
const OMINOUS_KEY: &str = "minecraft:ominous_trial_key";
const ACTIVATION: f32 = 4.0;
const DEACTIVATION: f32 = 4.5;
/// Ticks between state updates, display changes and ejected items.
const STEP: i64 = 20;
/// Ticks from a key's use to the first item.
const UNLOCKING: i64 = 14;
/// The most players a vault remembers; the earliest goes first.
const REMEMBERED: usize = 128;
/// A refused key is heard no more often than this.
const FAIL_GAP: i64 = 15;
/// Items come out this far over the block's corner, at this speed up and up to this times
/// (`rand - rand`) any way.
const OUT: [f32; 3] = [0.5, 1.2 - 0.125, 0.5];
/// Where an ejected item is heard from, over the corner (research/captures/bds-x2ran.txt).
const EJECT_SOUND: [f32; 3] = [0.5, 1.2, 0.5];
const UP: f32 = 0.2;
const SPREAD: f32 = 0.034455;

/// When each vault last refused a key aloud; BDS keeps it unsaved too.
#[derive(Resource, Default)]
struct Refusals(HashMap<(Entity, BlockPos), i64>);

/// Its `config`, with BDS's defaults; an ominous vault's ([U]: its template sets them) where none is saved.
struct Config {
    loot_table: String,
    display_table: Option<String>,
    activation: f32,
    deactivation: f32,
    key: String,
}

impl Config {
    fn of(fields: &Fields, ominous: bool) -> Self {
        let config = fields.inner("config");
        let (table, key) = if ominous { (OMINOUS_REWARD, OMINOUS_KEY) } else { (REWARD, KEY) };
        let activation = config.float("activation_range", ACTIVATION);
        Self {
            loot_table: config.string("loot_table").unwrap_or(table).to_owned(),
            display_table: config.string("override_loot_table_to_display").filter(|table| !table.is_empty()).map(str::to_owned),
            activation,
            deactivation: config.float("deactivation_range", DEACTIVATION).max(activation),
            key: config.get("key_item").and_then(ItemStack::parse).map_or_else(|| key.to_owned(), |stack| stack.name),
        }
    }
}

/// A vault's `data` and what it shows.
struct Vault {
    fields: Fields,
    data: Fields,
    config: Config,
    state: &'static str,
}

impl Vault {
    fn rewarded(&self) -> Vec<i64> {
        as_longs(&self.data.list("rewarded_players"))
    }

    /// The players near enough by `range` that it has not rewarded.
    fn connected(&self, pos: BlockPos, near: &[Near], range: f32) -> Vec<i64> {
        let rewarded = self.rewarded();
        let mut ids: Vec<i64> = near.iter().filter(|player| !rewarded.contains(&player.unique_id) && distance(centre(pos), player.eyes()) <= range).map(|player| player.unique_id).collect();
        ids.sort_unstable();
        ids
    }

    fn display(&mut self, shown: Option<Value>) {
        let mut shared = self.fields.inner("shared_data");
        match shown {
            Some(item) => {
                self.data.set("display_item", item.clone());
                shared.set("display_item", item);
            }
            None => {
                self.data.0.retain(|(key, _)| key != "display_item");
                shared.0.retain(|(key, _)| key != "display_item");
            }
        }
        self.fields.set("shared_data", shared.into_value());
    }

    fn connect(&mut self, connected: &[i64]) {
        let mut shared = self.fields.inner("shared_data");
        shared.set("connected_players", longs(connected));
        self.fields.set("shared_data", shared.into_value());
    }

    fn into_value(mut self) -> Value {
        self.fields.set("data", self.data.into_value());
        self.fields.into_value()
    }
}

fn load(world: &World, layer: Entity, pos: BlockPos, fields: Fields) -> Option<Vault> {
    let state = property(world, layer, pos, "vault_state")?;
    let ominous = property(world, layer, pos, "ominous") == Some("1");
    let config = Config::of(&fields, ominous);
    Some(Vault { data: fields.inner("data"), fields, config, state })
}

/// One item of a roll of `table` to show: none for an empty roll.
fn pick_display(world: &mut World, layer: Entity, table: &str) -> Option<Value> {
    let rolled = roll(world, table);
    let at = if rolled.len() <= 1 { 0 } else { dice(world).roll(rolled.len() as u32) as usize };
    let (name, count) = rolled.into_iter().nth(at)?;
    stack_value(world, layer, name, count)
}

fn stack_value(world: &World, layer: Entity, name: String, count: u32) -> Option<Value> {
    let registry = world.get::<Layer>(layer)?.world.registry();
    crate::inventory::stored(&Item::new(name, count.min(64) as u8), registry).map(|stack| Value::Compound(stack.fields(registry)))
}

pub(super) fn tick(world: &mut World, layer: Entity, pos: BlockPos, fields: Fields, near: &[Near]) {
    let Some(mut vault) = load(world, layer, pos, fields) else { return };
    let now = now(world);
    let mut shown = false;
    if vault.state == "active" && now % STEP == 0 {
        let table = vault.config.display_table.clone().unwrap_or_else(|| vault.config.loot_table.clone());
        let item = pick_display(world, layer, &table);
        vault.display(item);
        shown = true;
    }
    // A stored world's ticks are BDS's, not this shard's.
    let resumes = vault.data.int("state_updating_resumes_at", 0).min(now + STEP);
    let mut kept = shown;
    let (mut entered, mut heard) = (false, None);
    if now >= resumes {
        let next = step(world, layer, pos, &mut vault, near);
        vault.data.set("state_updating_resumes_at", Value::Long(now + next.1));
        if let Some(to) = next.0 {
            heard = enter(world, layer, pos, &mut vault, to);
            (shown, entered) = (true, true);
        }
        shown |= vault.state == "ejecting";
        kept = true;
    }
    // BDS showed an idle vault's entity once, at its set (research/captures/bds-s7ents.txt); a busy one's
    // block again with it each second (bds-x2ran.txt).
    if shown && !entered {
        if let Some(mut level) = world.get_mut::<crate::components::Layer>(layer) {
            _ = level.world.set_block_entity(pos, &vault.into_value());
        }
        crate::blocks::show_whole(world, layer, pos);
    } else if shown {
        crate::blocks::keep_entity(world, layer, pos, &vault.into_value());
    } else if kept && let Some(mut level) = world.get_mut::<crate::components::Layer>(layer) {
        _ = level.world.set_block_entity(pos, &vault.into_value());
    }
    if let Some(heard) = heard {
        sound(world, layer, pos, heard, corner(pos), -1);
    }
}

/// One state update: the state to go to, if another, and the ticks to the next.
fn step(world: &mut World, layer: Entity, pos: BlockPos, vault: &mut Vault, near: &[Near]) -> (Option<&'static str>, i64) {
    let settle = |vault: &mut Vault, range: f32| {
        let connected = vault.connected(pos, near, range);
        vault.connect(&connected);
        if connected.is_empty() { "inactive" } else { "active" }
    };
    let (activation, deactivation) = (vault.config.activation, vault.config.deactivation);
    match vault.state {
        "inactive" => (Some(settle(vault, activation)).filter(|to| *to != "inactive"), STEP),
        "active" => (Some(settle(vault, deactivation)).filter(|to| *to != "active"), STEP),
        "unlocking" => (Some("ejecting"), STEP),
        "ejecting" => {
            let mut items = vault.data.list("items_to_eject");
            let Some(next) = items.pop() else {
                vault.data.set("total_ejections_needed", Value::Long(0));
                return (Some(settle(vault, deactivation)), STEP);
            };
            let total = vault.data.int("total_ejections_needed", 1).max(1);
            let pitch = if total == 1 { 1000 } else { ((1.0 - (items.len() as f32) / (total - 1) as f32) * 1000.0) as i32 };
            vault.display(items.last().cloned());
            vault.data.set("items_to_eject", compounds(items));
            if let Some(stack) = ItemStack::parse(&next) {
                let at: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + OUT[axis]);
                let mut spread = || { let mut dice = dice(world); (dice.fraction() - dice.fraction()) * SPREAD };
                let motion = [spread(), UP + spread(), spread()];
                item(world, layer, (stack.name, stack.count.into()), at, motion);
            }
            event(world, layer, pos, LevelEventEvent::AnimationVaultEjectItem, corner(pos), 0);
            sound(world, layer, pos, "vault.eject_item", std::array::from_fn(|axis| pos[axis] as f32 + EJECT_SOUND[axis]), pitch);
            (None, STEP)
        }
        _ => (None, STEP),
    }
}

/// Goes to `to`, told as BDS's `onTransitionBetweenStates` tells it: its animation now, the sound it
/// returns after its block and entity (research/captures/bds-x2ran.txt).
#[must_use]
fn enter(world: &mut World, layer: Entity, pos: BlockPos, vault: &mut Vault, to: &'static str) -> Option<&'static str> {
    let sound = match (vault.state, to) {
        ("ejecting", _) => Some("vault.close_shutter"),
        (_, "unlocking") => Some("vault.insert_item"),
        (_, "ejecting") => Some("vault.open_shutter"),
        _ => None,
    };
    match to {
        "inactive" => {
            vault.display(None);
            event(world, layer, pos, LevelEventEvent::AnimationVaultDeactivate, corner(pos), 0);
        }
        "active" => {
            if vault.data.get("display_item").is_none() {
                let table = vault.config.display_table.clone().unwrap_or_else(|| vault.config.loot_table.clone());
                let item = pick_display(world, layer, &table);
                vault.display(item);
            }
            event(world, layer, pos, LevelEventEvent::AnimationVaultActivate, corner(pos), 0);
        }
        _ => {}
    }
    vault.state = to;
    set_state(world, layer, pos, "vault_state", to);
    sound
}

/// A block entity as `/clone` copies it: a vault's copy is connected to nobody until its next update,
/// as BDS's showed (research/captures/bds-x2ran.txt); any other as it was.
pub(crate) fn cloned(entity: &Value) -> Value {
    let Value::Compound(fields) = entity else { return entity.clone() };
    let mut fields = Fields(fields.clone());
    if fields.string("id") != Some(super::VAULT) {
        return entity.clone();
    }
    let mut shared = fields.inner("shared_data");
    shared.set("connected_players", longs(&[]));
    fields.set("shared_data", shared.into_value());
    fields.into_value()
}

/// Where BDS tells a vault's sounds and animations: its cell's corner.
fn corner(pos: BlockPos) -> [f32; 3] {
    pos.map(|c| c as f32)
}

/// A click with `held` on an active vault: its key, from a player not yet rewarded, unlocks it
/// (`VaultBlockActor::tryInsertKey`). `Some(spent)` on an active vault, `None` otherwise.
pub(super) fn use_key(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, held: &str) -> Option<bool> {
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
    let heard = enter(world, layer, pos, &mut vault, "unlocking");
    crate::blocks::keep_entity(world, layer, pos, &vault.into_value());
    if let Some(heard) = heard {
        sound(world, layer, pos, heard, corner(pos), -1);
    }
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    Some(!creative)
}
