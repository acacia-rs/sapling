//! What a `/loot` source makes: a table's roll, a mob's death loot or a block's drop, as stacks.

use sapling_api::command::Selected;
use sapling_api::{Item, Output, Sender, ShardCtx, Slot};
use sapling_data::Killer;
use sapling_data::enchant::Enchant;

use super::Source;
use crate::vanilla::selecting::Roll;
use crate::vanilla::{item_named, no_target};

/// The tool a source is run with: the sender's hands by name, otherwise an item by name.
pub(super) fn tool(ctx: &ShardCtx<'_>, sender: &Sender, word: Option<&str>) -> Option<Item> {
    let word = word?;
    let player = sender.player().map(|player| player.id);
    match word.to_ascii_lowercase().as_str() {
        "mainhand" => ctx.held_item(player?),
        "offhand" => ctx.item(player?, Slot::Offhand),
        _ => Some(Item::new(item_named(word), 1)),
    }
}

fn looting(tool: Option<&Item>) -> u8 {
    let level = tool.and_then(|tool| tool.enchantments.iter().find(|(id, _)| *id == Enchant::Looting as i16));
    level.map_or(0, |(_, level)| (*level).clamp(0, 255) as u8)
}

/// (item, count) as stacks no bigger than the item stacks.
fn stacks(made: Vec<(String, u32)>) -> Vec<Item> {
    let mut items = Vec::new();
    for (name, mut count) in made {
        let most = u32::from(sapling_data::max_stack(&name).max(1));
        while count > 0 {
            let taken = count.min(most);
            items.push(Item::new(name.clone(), taken as u8));
            count -= taken;
        }
    }
    items
}

/// The stacks `source` makes; `None` once `out` holds the refusal.
pub(super) fn items(ctx: &mut ShardCtx<'_>, sender: &Sender, source: &Source, tool: Option<&Item>, out: &mut Output) -> Option<Vec<Item>> {
    let killer = Killer::Player { looting: looting(tool) };
    let mut roll = Roll::new();
    match source {
        Source::Table(path) => {
            let table = format!("loot_tables/{}.json", path.to_ascii_lowercase());
            if !sapling_data::has_loot_table(&table) {
                out.translate("commands.loot.failure.invalidLootTable", vec![table], false);
                return None;
            }
            Some(stacks(sapling_data::table_loot_for(&table, killer, &mut || roll.next())))
        }
        Source::Kill(selector) => {
            let found = ctx.select(sender, selector);
            match found.as_slice() {
                [] => {
                    no_target(out);
                    None
                }
                [Selected::Player(_)] => Some(Vec::new()),
                [Selected::Entity(entity)] => Some(stacks(sapling_data::mob_loot(&entity.kind, false, killer, &mut || roll.next()))),
                _ => {
                    out.translate("commands.generic.tooManyTargets", Vec::new(), false);
                    None
                }
            }
        }
        Source::Mine(position) => {
            let pos = position.resolve(ctx, sender);
            Some(ctx.block_loot(pos, tool))
        }
    }
}
