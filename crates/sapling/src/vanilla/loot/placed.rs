//! Where `/loot` puts what it made, and its answer: the number of stacks made, summed over the
//! targets, or for `replace` the items and the stacks put (research/captures/bds-t1dloot.txt).
//! `replace` puts every stack made, or the count's first, from the slot on through the slots
//! after it: a player's 36 (hotbar 7 ran into the inventory), a mob's hands then head (a cow's
//! mainhand ran into its offhand and head), a container's.

use sapling_api::{BlockPos, Item, MobSlot, Output, PlayerId, Sender, ShardCtx, Slot};

use super::Target;
use crate::vanilla::{answer_name, no_target, players, targets};

/// A player's slots, as one container: hotbar then inventory.
const PLAYER_SLOTS: usize = 36;
/// The last of [`MOB_SLOTS`] a mob's stacks run on to: the head.
const SPILLS_TO: usize = 2;
const MOB_SLOTS: [MobSlot; 6] = [MobSlot::MainHand, MobSlot::OffHand, MobSlot::Armor(0), MobSlot::Armor(1), MobSlot::Armor(2), MobSlot::Armor(3)];

pub(super) fn put(ctx: &mut ShardCtx<'_>, sender: &Sender, target: &Target, items: &[Item], out: &mut Output) {
    let made = items.len();
    match target {
        Target::Spawn(position) => {
            // BDS drops at the cell's corner, an eighth up (bds-e2rails.txt, bds-lootcmd2.txt).
            let [x, y, z] = position.resolve(ctx, sender).map(f32::floor);
            let at = [x, y + 0.125, z];
            items.iter().for_each(|item| _ = ctx.drop_item(item, at));
            success(out, made);
        }
        Target::Give(selector) => {
            let found = players(ctx, sender, Some(selector));
            if found.is_empty() {
                return no_target(out);
            }
            for player in &found {
                items.iter().for_each(|item| _ = ctx.give(player.id, item));
            }
            success(out, made * found.len());
        }
        Target::Insert(position) => {
            let pos = position.resolve(ctx, sender);
            let Some(size) = ctx.container_size(pos) else { return no_container(out, pos) };
            items.iter().for_each(|item| insert(ctx, pos, size, item));
            success(out, made);
        }
        Target::Entity { target, slot_type, slot, count } => replace_entity(ctx, sender, target, slot_type, *slot, *count, items, out),
        Target::Block { position, slot, count } => {
            let pos = position.resolve(ctx, sender);
            let Some(size) = ctx.container_size(pos) else { return no_container(out, pos) };
            let Some(start) = usize::try_from(*slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, *slot, size, "none") };
            let Some(put) = putting(items, *count, start, size, size, out) else { return };
            for (offset, item) in put.iter().enumerate() {
                ctx.set_container_item(pos, start + offset, Some(item));
            }
            out.translate("commands.loot.replaceSuccess.block", vec![super::CONTAINER.into(), slot.to_string(), total(put).to_string(), put.len().to_string()], true);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn replace_entity(ctx: &mut ShardCtx<'_>, sender: &Sender, target: &sapling_api::command::Selector, slot_type: &str, slot: i32, count: Option<i32>, items: &[Item], out: &mut Output) {
    let (first, size) = slot_range(slot_type);
    let Some(start) = usize::try_from(slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, slot, size, slot_type) };
    let (found, mobs) = targets(ctx, sender, Some(target));
    if found.is_empty() && mobs.is_empty() {
        return no_target(out);
    }
    let shown = |out: &mut Output, name: String, put: &[Item]| {
        let shown = [slot_type.to_owned(), slot.to_string(), name, total(put).to_string(), put.len().to_string()];
        out.translate("commands.loot.replaceSuccess", shown.into(), true);
    };
    // A hand or armour slot of a player takes one stack.
    let (from, slots) = if matches!(slot_type, "slot.hotbar" | "slot.inventory") { (first + start, PLAYER_SLOTS) } else { (0, 1) };
    for player in found {
        let Some(put) = putting(items, count, from, slots, size, out) else { return };
        for (offset, item) in put.iter().enumerate() {
            ctx.set_item(player.id, player_slot(ctx, player.id, slot_type, first + start + offset), Some(item));
        }
        shown(out, player.name, put);
    }
    for mob in mobs {
        let Some(at) = MOB_SLOTS.iter().position(|kind| Some(*kind) == mob_slot(slot_type)) else {
            return out.translate("commands.loot.failure.invalidSlotTypeForEntity", vec![answer_name(ctx, &mob), slot_type.into()], false);
        };
        // Through the hands and the head, which a cow's head did not run on from; one in the rest.
        let end = if at <= SPILLS_TO { SPILLS_TO + 1 } else { at + 1 };
        let Some(put) = putting(items, count, at, end, size, out) else { return };
        for (kind, item) in MOB_SLOTS[at..].iter().zip(put) {
            _ = ctx.set_mob_item(mob.id, *kind, Some(item));
        }
        shown(out, answer_name(ctx, &mob), put);
    }
}

/// The stacks `replace` puts from container slot `from` of `size`: all, or the count's first,
/// as many as there is room for; `countExceedsContainerSize` with the slot type's size where the
/// count reaches past the container.
fn putting<'a>(items: &'a [Item], count: Option<i32>, from: usize, size: usize, shown_size: usize, out: &mut Output) -> Option<&'a [Item]> {
    let wanted = match count {
        Some(count) => usize::try_from(count).unwrap_or(0),
        None => items.len(),
    };
    if count.is_some() && from + wanted > size {
        out.translate("commands.loot.failure.countExceedsContainerSize", vec![shown_size.to_string()], false);
        return None;
    }
    Some(&items[..wanted.min(items.len()).min(size - from)])
}

fn total(items: &[Item]) -> u32 {
    items.iter().map(|item| u32::from(item.count)).sum()
}

fn player_slot(ctx: &ShardCtx<'_>, player: PlayerId, slot_type: &str, index: usize) -> Slot {
    match slot_type {
        "slot.weapon.mainhand" => Slot::Main(ctx.held_slot(player).unwrap_or(0)),
        "slot.weapon.offhand" => Slot::Offhand,
        armour if armour.starts_with("slot.armor.") => Slot::Armor(slot_range(armour).0 as u8),
        _ => Slot::Main(index as u8),
    }
}

fn mob_slot(slot_type: &str) -> Option<MobSlot> {
    Some(match slot_type {
        "slot.weapon.mainhand" => MobSlot::MainHand,
        "slot.weapon.offhand" => MobSlot::OffHand,
        armour if armour.starts_with("slot.armor.") => MobSlot::Armor(slot_range(armour).0 as u8),
        _ => return None,
    })
}

fn success(out: &mut Output, stacks: usize) {
    out.translate("commands.loot.success", vec![stacks.to_string()], true);
}

fn no_container(out: &mut Output, [x, y, z]: BlockPos) {
    out.translate("commands.loot.failure.noContainer", vec![format!("Pos({x},{y},{z})")], false);
}

fn bad_slot(out: &mut Output, slot: i32, size: usize, slot_type: &str) {
    out.translate("commands.loot.failure.invalidSlotIdForSlotType", vec![slot.to_string(), "0".into(), (size - 1).to_string(), slot_type.into()], false);
}

/// The player slot the first of a slot type is, and how many it has.
fn slot_range(slot_type: &str) -> (usize, usize) {
    match slot_type {
        "slot.hotbar" => (0, 9),
        "slot.inventory" => (9, 27),
        "slot.armor.head" => (0, 1),
        "slot.armor.chest" => (1, 1),
        "slot.armor.legs" => (2, 1),
        "slot.armor.feet" => (3, 1),
        _ => (0, 1),
    }
}

/// Puts `item` into the container at `pos` as a hopper would: onto its own kind first, then into
/// the first empty slots; what does not fit is lost.
fn insert(ctx: &mut ShardCtx<'_>, pos: BlockPos, size: usize, item: &Item) {
    let most = sapling_data::max_stack(&item.name).max(1);
    let same = |held: &Item| Item { count: 0, ..held.clone() } == Item { count: 0, ..item.clone() };
    let mut left = item.count;
    for slot in 0..size {
        match ctx.container_item(pos, slot) {
            Some(mut held) if left > 0 && same(&held) && held.count < most => {
                let moved = left.min(most - held.count);
                held.count += moved;
                left -= moved;
                ctx.set_container_item(pos, slot, Some(&held));
            }
            _ => {}
        }
    }
    for slot in 0..size {
        if left > 0 && ctx.container_item(pos, slot).is_none() {
            let moved = left.min(most);
            ctx.set_container_item(pos, slot, Some(&Item { count: moved, ..item.clone() }));
            left -= moved;
        }
    }
}
