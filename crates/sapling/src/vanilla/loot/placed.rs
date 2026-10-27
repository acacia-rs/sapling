//! Where `/loot` puts what it made, and its answer: the number of stacks made, summed over the
//! targets, or for `replace` the slots filled.
// TODO: `replace entity` fills players' slots only; BDS also filled a cow's mainhand. What the
// last two numbers of `replaceSuccess` count is a guess: two drills cannot tell them apart.

use sapling_api::{BlockPos, Item, Output, Sender, ShardCtx, Slot};

use super::Target;
use crate::vanilla::{no_target, players};

pub(super) fn put(ctx: &mut ShardCtx<'_>, sender: &Sender, target: &Target, items: &[Item], out: &mut Output) {
    let made = items.len();
    match target {
        Target::Spawn(position) => {
            let at = position.resolve(ctx, sender);
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
        Target::Entity { target, slot_type, slot, count } => {
            let (first, size) = slot_range(slot_type);
            let Some(start) = u8::try_from(*slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, *slot, size.into(), slot_type) };
            let found = players(ctx, sender, Some(target));
            if found.is_empty() {
                return no_target(out);
            }
            let filled = filled(items, *count, usize::from(size - start));
            for player in found {
                for (offset, item) in items[..filled].iter().enumerate() {
                    let index = start + offset as u8;
                    let slot = match slot_type.as_str() {
                        "slot.weapon.mainhand" => Slot::Main(ctx.held_slot(player.id).unwrap_or(0)),
                        "slot.weapon.offhand" => Slot::Offhand,
                        armour if armour.starts_with("slot.armor.") => Slot::Armor(first),
                        _ => Slot::Main(first + index),
                    };
                    ctx.set_item(player.id, slot, Some(item));
                }
                let shown = [slot_type.clone(), slot.to_string(), player.name, filled.to_string(), filled.to_string()];
                out.translate("commands.loot.replaceSuccess", shown.into(), true);
            }
        }
        Target::Block { position, slot, count } => {
            let pos = position.resolve(ctx, sender);
            let Some(size) = ctx.container_size(pos) else { return no_container(out, pos) };
            let Some(start) = usize::try_from(*slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, *slot, size, "none") };
            let filled = filled(items, *count, size - start);
            for (offset, item) in items[..filled].iter().enumerate() {
                ctx.set_container_item(pos, start + offset, Some(item));
            }
            out.translate("commands.loot.replaceSuccess.block", vec![super::CONTAINER.into(), slot.to_string(), filled.to_string(), "1".into()], true);
        }
    }
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

/// The slots `replace` fills: one unless a count says more, never past the stacks or the slots.
fn filled(items: &[Item], count: Option<i32>, room: usize) -> usize {
    usize::try_from(count.unwrap_or(1)).unwrap_or(0).min(items.len()).min(room)
}

/// The inventory slot the first of a slot type is, and how many it has.
fn slot_range(slot_type: &str) -> (u8, u8) {
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
