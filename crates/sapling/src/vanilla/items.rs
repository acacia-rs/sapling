//! `/give` and `/clear`, with BDS's data values, components and filters
//! (research/captures/bds-itemcmds.txt).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Item, ItemLock, Output, Permission, Player, Sender, ShardCtx, Slot};

use super::{display_name, item_named, no_target, players, tell_target};

/// The most `/give` hands out at once.
const MOST: i32 = 32_767;
/// The colours old names number by data value: `/give @s wool 3 14` gave red wool.
const COLOURS: [&str; 16] = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black"];
// TODO: only `wool` was tried on BDS; the rest are the same flattening.
const COLOURED: [(&str, &str); 7] = [
    ("wool", "wool"),
    ("carpet", "carpet"),
    ("concrete", "concrete"),
    ("concrete_powder", "concrete_powder"),
    ("stained_glass", "stained_glass"),
    ("stained_glass_pane", "stained_glass_pane"),
    ("stained_hardened_clay", "terracotta"),
];

/// The item an item name and data value name, and the data left for it: an old coloured name
/// takes its colour from the data.
fn item_and_data(name: &str, data: Option<i32>) -> (String, Option<i16>) {
    let plain = name.strip_prefix("minecraft:").unwrap_or(name);
    let coloured = COLOURED.iter().find(|(old, _)| *old == plain).zip(data.and_then(|data| COLOURS.get(usize::try_from(data).ok()?)));
    match coloured {
        Some(((_, new), colour)) => (format!("minecraft:{colour}_{new}"), None),
        None => (item_named(name), data.map(|data| data as i16)),
    }
}

/// An item name that names an item, or a syntax error at it.
fn item_word(args: &mut Args<'_>) -> Result<String, ParseError> {
    let mark = args.mark();
    let name = args.word()?.to_owned();
    if !sapling_data::is_item(&item_and_data(&name, None).0) && !COLOURED.iter().any(|(old, _)| name.trim_start_matches("minecraft:") == *old) {
        return Err(args.error_at(mark));
    }
    Ok(name)
}

/// What `/give`'s components set on the stack, or the line BDS refuses one with.
fn components(json: &str) -> Result<(bool, Option<ItemLock>), (String, Vec<String>)> {
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(json).map_err(|error| ("commands.give.tagError".to_owned(), vec![error.to_string()]))?;
    let (mut keep_on_death, mut lock) = (false, None);
    for (name, value) in &parsed {
        match name.as_str() {
            "minecraft:keep_on_death" => keep_on_death = true,
            "minecraft:item_lock" => match value.get("mode").and_then(serde_json::Value::as_str) {
                Some("lock_in_slot") => lock = Some(ItemLock::InSlot),
                Some("lock_in_inventory") => lock = Some(ItemLock::InInventory),
                _ => return Err(("commands.give.tagError".to_owned(), vec![name.clone()])),
            },
            // TODO: `minecraft:can_destroy` and `minecraft:can_place_on`: stacks do not carry them yet.
            _ => return Err((format!("Component '{name}' is not a supported component."), Vec::new())),
        }
    }
    Ok((keep_on_death, lock))
}

/// `/give <player> <itemName> [amount] [data] [components]`.
pub(crate) struct Give {
    player: Selector,
    item: String,
    amount: Option<i32>,
    data: Option<i32>,
    components: Option<String>,
}

impl CommandSyntax for Give {
    fn spec() -> CommandSpec {
        let components = ParameterSpec { kind: ParamKind::Json, ..ParameterSpec::of::<Option<i32>>("components") };
        let parameters = vec![ParameterSpec::of::<Selector>("player"), ParameterSpec::of::<String>("itemName"), ParameterSpec::of::<Option<i32>>("amount"), ParameterSpec::of::<Option<i32>>("data"), components];
        CommandSpec { name: "give".into(), aliases: Vec::new(), description: "Gives an item to a player".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (player, item) = (Selector::parse(args)?, item_word(args)?);
        let (amount, data) = (Option::<i32>::parse(args)?, Option::<i32>::parse(args)?);
        let components = if args.is_empty() { None } else { Some(args.json()?.to_owned()) };
        args.finish()?;
        Ok(Self { player, item, amount, data, components })
    }
}

impl Command for Give {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let amount = self.amount.unwrap_or(1);
        if amount < 1 {
            return out.translate("commands.generic.num.tooSmall", vec![amount.to_string(), "1".into()], false);
        }
        if amount > MOST {
            return out.translate("commands.generic.num.tooBig", vec![amount.to_string(), MOST.to_string()], false);
        }
        if let Some(data) = self.data.filter(|data| *data < 0) {
            return out.translate("commands.generic.num.tooSmall", vec![data.to_string(), "0".into()], false);
        }
        let (keep_on_death, lock) = match self.components.as_deref().map(components).transpose() {
            Ok(set) => set.unwrap_or_default(),
            Err((line, parameters)) => return out.translate(line, parameters, false),
        };
        let (name, data) = item_and_data(&self.item, self.data);
        let item = Item { damage: data.unwrap_or(0), keep_on_death, lock, ..Item::new(name, 1) };
        let (shown, count) = (display_name(&item.name), amount.to_string());
        let given = players(ctx, sender, Some(&self.player));
        for player in &given {
            give_all(ctx, player, &item, amount);
            tell_target(ctx, player.id, "commands.give.successRecipient", &[shown.clone(), count.clone()]);
        }
        match given.is_empty() {
            true => no_target(out),
            false => {
                out.translate("commands.give.success", vec![shown, count, names(&given)], true);
                out.done_to(given.len() as u32);
            }
        }
    }
}

/// Gives `amount` of `item` a stack at a time, until the inventory is full.
// TODO: BDS gave 32767 stone into a 2304-stone inventory; whether it dropped the rest was not seen.
fn give_all(ctx: &mut ShardCtx<'_>, player: &Player, item: &Item, amount: i32) {
    let stack = i32::from(sapling_data::max_stack(&item.name).max(1));
    let mut left = amount;
    while left > 0 {
        let count = left.min(stack);
        if ctx.give(player.id, &Item { count: count as u8, ..item.clone() }) > 0 {
            break;
        }
        left -= count;
    }
}

fn names(players: &[Player]) -> String {
    players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ")
}

/// Every slot `/clear` looks in, in the order it takes from them.
fn slots() -> impl Iterator<Item = Slot> {
    (0..36).map(Slot::Main).chain((0..4).map(Slot::Armor)).chain([Slot::Offhand])
}

/// `/clear [player] [itemName] [data] [maxCount]`: data -1 is any; a `maxCount` of 0 counts
/// what matches without taking it.
#[derive(Command)]
#[command(crate = "crate", name = "clear", description = "Clears items from a player's inventory", permission = "operator")]
pub(crate) struct Clear {
    player: Option<Selector>,
    item_name: Option<ItemName>,
    data: Option<i32>,
    max_count: Option<i32>,
}

/// An item name `/clear` takes: one the game has.
pub(crate) struct ItemName(String);

impl Parameter for ItemName {
    fn kind() -> ParamKind {
        ParamKind::String
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        item_word(args).map(Self)
    }
}

impl Command for Clear {
    /// One line per player, as BDS answers.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let cleared = players(ctx, sender, self.player.as_ref());
        if cleared.is_empty() {
            return no_target(out);
        }
        let wanted = self.item_name.as_ref().map(|name| item_and_data(&name.0, self.data.filter(|data| *data >= 0)));
        for player in &cleared {
            let removed = match &wanted {
                None => ctx.clear_inventory(player.id),
                Some((name, data)) => take(ctx, player, name, *data, self.max_count),
            };
            match (removed, self.max_count) {
                (count, Some(0)) => out.translate("commands.clear.testing", vec![player.name.clone(), count.to_string()], count > 0),
                (0, _) => out.translate("commands.clear.failure.no.items", vec![player.name.clone()], false),
                (removed, _) => out.translate("commands.clear.success", vec![player.name.clone(), removed.to_string()], true),
            }
        }
    }
}

/// Takes up to `most` (all for none or a negative) of the item from a player's slots; how many
/// it took, or with `most` 0 how many there are.
fn take(ctx: &mut ShardCtx<'_>, player: &Player, name: &str, data: Option<i16>, most: Option<i32>) -> u32 {
    let mut left = most.filter(|most| *most >= 0).map_or(u32::MAX, |most| most as u32);
    let mut found = 0;
    for slot in slots() {
        let Some(mut item) = ctx.item(player.id, slot).filter(|item| item.name == name && data.is_none_or(|data| data == item.damage)) else { continue };
        if most == Some(0) {
            found += u32::from(item.count);
            continue;
        }
        let taken = u32::from(item.count).min(left);
        item.count -= taken as u8;
        ctx.set_item(player.id, slot, Some(&item).filter(|item| item.count > 0));
        (found, left) = (found + taken, left - taken);
        if left == 0 {
            break;
        }
    }
    found
}
