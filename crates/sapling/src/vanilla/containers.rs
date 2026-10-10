//! `/replaceitem block <position> slot.container <slot> <item> [amount] [data]` puts an item into
//! a container block's slot, `/replaceitem entity <target> <slot type> <slot> <item> [amount]
//! [data]` into a player's, answering as BDS does (research/captures/bds-replaceitem.txt,
//! bds-hit.txt).
// TODO: of entities only players, and of their slots the hand, the hotbar and the inventory; BDS's
// refusals of the entity form were not recorded; the data value is the item's, unchecked; BDS's bound
// on the amount may be the item's own stack size (only stone's 64 was recorded), and an amount
// below 1 was not tried.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Item, Output, Permission, Sender, ShardCtx, Slot};

use super::{display_name, no_target, origin, players};

const SLOT_TYPE: &str = "slot.container";
/// A player's slot types, and the inventory slot the first of each is; the hand is the held one.
const HAND: &str = "slot.weapon.mainhand";
const PLAYER_SLOTS: &[(&str, u8, u8)] = &[("slot.hotbar", 0, 9), ("slot.inventory", 9, 27)];
const ARMOUR_SLOTS: [&str; 4] = ["slot.armor.head", "slot.armor.chest", "slot.armor.legs", "slot.armor.feet"];
const MOST: i32 = 64;

enum Target {
    Block(BlockCoords),
    Entity(Selector, String),
}

pub(crate) struct ReplaceItem {
    target: Target,
    slot: i32,
    item: String,
    /// How BDS reports the item when the game does not have it.
    item_wrong: ParseError,
    amount: Option<i32>,
    data: Option<i32>,
}

/// The next word, which must be `word` (in any case).
fn literal(args: &mut Args<'_>, word: &str) -> Result<(), ParseError> {
    let mark = args.mark();
    args.word()?.eq_ignore_ascii_case(word).then_some(()).ok_or_else(|| args.error_at(mark))
}

impl CommandSyntax for ReplaceItem {
    fn spec() -> CommandSpec {
        let rest = || {
            vec![
                ParameterSpec::of::<i32>("slotId"),
                ParameterSpec { name: "itemName".into(), kind: ParamKind::String, optional: false },
                ParameterSpec::of::<Option<i32>>("amount"),
                ParameterSpec::of::<Option<i32>>("data"),
            ]
        };
        let block = [ParameterSpec::literal("replaceitem", "block"), ParameterSpec::of::<BlockCoords>("position"), ParameterSpec::literal("replaceitem", SLOT_TYPE)];
        let entity = [ParameterSpec::literal("replaceitem", "entity"), ParameterSpec::of::<Selector>("target"), ParameterSpec { name: "slotType".into(), kind: ParamKind::String, optional: false }];
        CommandSpec {
            name: "replaceitem".into(),
            aliases: Vec::new(),
            description: "Replaces items in inventories".into(),
            permission: Permission::Operator,
            overloads: vec![Overload { parameters: block.into_iter().chain(rest()).collect() }, Overload { parameters: entity.into_iter().chain(rest()).collect() }],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let target = match args.word()?.to_ascii_lowercase().as_str() {
            "block" => {
                let position = BlockCoords::parse(args)?;
                literal(args, SLOT_TYPE)?;
                Target::Block(position)
            }
            "entity" => {
                let selector = Selector::parse(args)?;
                let mark = args.mark();
                let kind = args.word()?.to_ascii_lowercase();
                if kind != HAND && !ARMOUR_SLOTS.contains(&kind.as_str()) && !PLAYER_SLOTS.iter().any(|(known, ..)| *known == kind) {
                    return Err(args.error_at(mark));
                }
                Target::Entity(selector, kind)
            }
            _ => return Err(args.error_at(mark)),
        };
        let slot = i32::parse(args)?;
        let mark = args.mark();
        let item = args.word()?.to_owned();
        let item_wrong = args.error_at(mark);
        let amount = Option::<i32>::parse(args)?;
        let data = Option::<i32>::parse(args)?;
        args.finish()?;
        Ok(Self { target, slot, item, item_wrong, amount, data })
    }
}

impl Command for ReplaceItem {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let count = self.amount.unwrap_or(1);
        if count > MOST {
            return out.translate("commands.generic.num.tooBig", vec![count.to_string(), MOST.to_string()], false);
        }
        let name = if self.item.contains(':') { self.item.clone() } else { format!("minecraft:{}", self.item) };
        let item = (name != "minecraft:air").then(|| Item { damage: self.data.unwrap_or(0) as i16, ..Item::new(&name, count.clamp(1, MOST) as u8) });
        let bad_slot = |out: &mut Output, size: usize| out.translate("commands.replaceitem.badSlotNumber", vec![self.slot.to_string(), "0".into(), (size - 1).to_string()], false);
        match &self.target {
            Target::Block(position) => {
                let pos = position.from(origin(ctx, sender));
                let Some(size) = ctx.container_size(pos) else {
                    let [x, y, z] = pos;
                    return out.translate("commands.replaceitem.noContainer", vec![format!("Pos({x},{y},{z})")], false);
                };
                let Some(slot) = usize::try_from(self.slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, size) };
                if !ctx.set_container_item(pos, slot, item.as_ref()) {
                    return out.refuse(self.item_wrong);
                }
                out.translate("commands.replaceitem.success", vec![SLOT_TYPE.into(), slot.to_string(), count.to_string(), display_name(&name)], true);
            }
            Target::Entity(selector, kind) => {
                let found = players(ctx, sender, selector);
                if found.is_empty() {
                    return no_target(out);
                }
                let (first, size) = PLAYER_SLOTS.iter().find(|(known, ..)| known == kind).map_or((0, 1), |(_, first, size)| (*first, *size));
                let Some(slot) = u8::try_from(self.slot).ok().filter(|slot| *slot < size) else { return bad_slot(out, size.into()) };
                for player in found {
                    let held = if kind == HAND { ctx.held_slot(player.id).unwrap_or(0) } else { first + slot };
                    let place = ARMOUR_SLOTS.iter().position(|known| known == kind).map_or(Slot::Main(held), |piece| Slot::Armor(piece as u8));
                    if !ctx.set_item(player.id, place, item.as_ref()) {
                        return out.refuse(self.item_wrong);
                    }
                    out.translate("commands.replaceitem.success.entity", vec![kind.clone(), slot.to_string(), player.name.clone(), count.to_string(), display_name(&name)], true);
                }
            }
        }
    }
}
