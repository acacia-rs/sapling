//! `/replaceitem block <position> slot.container <slot> <item> [amount] [data]` puts an item into
//! a container block's slot, `/replaceitem entity <target> <slot type> <slot> <item> [amount]
//! [data]` into a player's or a mob's, answering as BDS does (research/captures/bds-replaceitem.txt,
//! bds-hit.txt, bds-fixreplace.txt).
// TODO: of a player's slots the hand, the hotbar, the inventory and armour; the data value is the item's, unchecked;
// BDS's bound on the amount may be the item's own stack size (only stone's 64 was recorded), and an amount
// below 1 was not tried.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, EntityInfo, Item, MobSlot, MobSlotRefusal, Output, Permission, Sender, ShardCtx, Slot};

use super::{display_name, item_display_name, no_target, targets};

const SLOT_TYPE: &str = "slot.container";
/// A player's slot types, and the inventory slot the first of each is; the hand is the held one.
const HAND: &str = "slot.weapon.mainhand";
const OFFHAND: &str = "slot.weapon.offhand";
const PLAYER_SLOTS: &[(&str, u8, u8)] = &[("slot.hotbar", 0, 9), ("slot.inventory", 9, 27)];
const ARMOUR_SLOTS: [&str; 4] = ["slot.armor.head", "slot.armor.chest", "slot.armor.legs", "slot.armor.feet"];
/// Slot types only mobs have.
const MOB_SLOTS: [&str; 4] = ["slot.saddle", "slot.armor", "slot.armor.body", "slot.chest"];
const MOST: i32 = 64;

enum Target {
    Block(BlockCoords),
    Entity(Selector, String),
}

pub(crate) struct ReplaceItem {
    target: Target,
    slot: i32,
    item: super::names::ItemName,
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

fn is_slot_type(kind: &str) -> bool {
    [HAND, OFFHAND].contains(&kind) || ARMOUR_SLOTS.contains(&kind) || MOB_SLOTS.contains(&kind) || PLAYER_SLOTS.iter().any(|(known, ..)| *known == kind)
}

/// Where a slot type and number put an item on a mob; `None` for a type no mob has.
fn mob_slot(kind: &str, slot: usize) -> Option<MobSlot> {
    Some(match kind {
        HAND => MobSlot::MainHand,
        OFFHAND => MobSlot::OffHand,
        "slot.saddle" => MobSlot::Saddle,
        // BDS took both for a horse's armour and a llama's carpet (bds-mountdeath.txt).
        "slot.armor" | "slot.armor.body" => MobSlot::Body,
        "slot.chest" => MobSlot::Chest(slot),
        "slot.inventory" => MobSlot::Inventory(slot),
        _ => MobSlot::Armor(ARMOUR_SLOTS.iter().position(|known| *known == kind)? as u8),
    })
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
                if !is_slot_type(&kind) {
                    return Err(args.error_at(mark));
                }
                Target::Entity(selector, kind)
            }
            _ => return Err(args.error_at(mark)),
        };
        let slot = i32::parse(args)?;
        let mark = args.mark();
        let item = super::names::ItemName::parse(args)?;
        let item_wrong = args.error_at(mark);
        let amount = Option::<i32>::parse(args)?;
        let data = Option::<i32>::parse(args)?;
        args.finish()?;
        Ok(Self { target, slot, item, item_wrong, amount, data })
    }
}

impl ReplaceItem {
    fn bad_slot(&self, out: &mut Output, size: usize) {
        out.translate("commands.replaceitem.badSlotNumber", vec![self.slot.to_string(), "0".into(), (size as i64 - 1).to_string()], false);
    }

    fn told(&self, out: &mut Output, kind: &str, slot: impl ToString, who: String, count: i32, item: String) {
        out.translate("commands.replaceitem.success.entity", vec![kind.to_owned(), slot.to_string(), who, count.to_string(), item], true);
    }

    /// Into a mob: BDS refused a slot type the mob lacks with `commands.generic.exception`.
    fn into_mob(&self, ctx: &mut ShardCtx<'_>, out: &mut Output, mob: &EntityInfo, kind: &str, item: Option<&Item>, shown: (i32, String)) {
        let slot = usize::try_from(self.slot).unwrap_or(usize::MAX);
        // The slots a mob wears are one each: past 0 is a bad slot.
        let in_range = matches!(mob_slot(kind, slot), Some(MobSlot::Chest(_) | MobSlot::Inventory(_))) || slot == 0;
        let placed = match mob_slot(kind, slot).filter(|_| in_range) {
            Some(place) => ctx.set_mob_item(mob.id, place, item),
            None if mob_slot(kind, slot).is_some() => Err(MobSlotRefusal::BadSlot { size: 1 }),
            None => Err(MobSlotRefusal::NoSlot),
        };
        match placed {
            Ok(()) => self.told(out, kind, slot, display_name(mob), shown.0, shown.1),
            Err(MobSlotRefusal::BadSlot { size }) => self.bad_slot(out, size),
            Err(_) => out.translate("commands.generic.exception", Vec::new(), false),
        }
    }
}

impl Command for ReplaceItem {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let count = self.amount.unwrap_or(1);
        if count > MOST {
            return out.translate("commands.generic.num.tooBig", vec![count.to_string(), MOST.to_string()], false);
        }
        let name = super::item_named(&self.item.0);
        let data = self.data.unwrap_or(0) as i16;
        let item = (name != "minecraft:air").then(|| Item { damage: data, ..Item::new(&name, count.clamp(1, MOST) as u8) });
        match &self.target {
            Target::Block(position) => {
                let pos = position.resolve(ctx, sender);
                let Some(size) = ctx.container_size(pos) else {
                    let [x, y, z] = pos;
                    return out.translate("commands.replaceitem.noContainer", vec![format!("Pos({x},{y},{z})")], false);
                };
                let Some(slot) = usize::try_from(self.slot).ok().filter(|slot| *slot < size) else { return self.bad_slot(out, size) };
                if !ctx.set_container_item(pos, slot, item.as_ref()) {
                    return out.refuse(self.item_wrong);
                }
                out.translate("commands.replaceitem.success", vec![SLOT_TYPE.into(), slot.to_string(), count.to_string(), item_display_name(&name, data)], true);
            }
            Target::Entity(selector, kind) => {
                let (found, mobs) = targets(ctx, sender, Some(selector));
                if found.is_empty() && mobs.is_empty() {
                    return no_target(out);
                }
                for mob in &mobs {
                    self.into_mob(ctx, out, mob, kind, item.as_ref(), (count, item_display_name(&name, data)));
                }
                if found.is_empty() {
                    return;
                }
                if MOB_SLOTS.contains(&kind.as_str()) {
                    return out.translate("commands.generic.exception", Vec::new(), false);
                }
                let (first, size) = PLAYER_SLOTS.iter().find(|(known, ..)| known == kind).map_or((0, 1), |(_, first, size)| (*first, *size));
                let Some(slot) = u8::try_from(self.slot).ok().filter(|slot| *slot < size) else { return self.bad_slot(out, size.into()) };
                for player in found {
                    let held = if kind == HAND { ctx.held_slot(player.id).unwrap_or(0) } else { first + slot };
                    let place = match ARMOUR_SLOTS.iter().position(|known| known == kind) {
                        Some(piece) => Slot::Armor(piece as u8),
                        None if kind == OFFHAND => Slot::Offhand,
                        None => Slot::Main(held),
                    };
                    if !ctx.set_item(player.id, place, item.as_ref()) {
                        return out.refuse(self.item_wrong);
                    }
                    self.told(out, kind, slot, player.name.clone(), count, item_display_name(&name, data));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_armor_body_is_a_mob_s_body_as_slot_armor_is() {
        assert!(is_slot_type("slot.armor.body"));
        assert_eq!([mob_slot("slot.armor.body", 0), mob_slot("slot.armor", 0)], [Some(MobSlot::Body); 2]);
    }
}
