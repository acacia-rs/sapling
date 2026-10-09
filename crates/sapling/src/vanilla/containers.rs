//! `/replaceitem block <position> slot.container <slot> <item> [amount] [data]`: puts an item into
//! a container block's slot, answering as BDS does (research/captures/bds-replaceitem.txt).
// TODO: `/replaceitem entity`; the data value is read and not used; BDS's bound on the amount may be
// the item's own stack size (only stone's 64 was recorded), and an amount below 1 was not tried.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError};
use sapling_api::{Command, Item, Output, Permission, Sender, ShardCtx};

use super::{display_name, origin};

const SLOT_TYPE: &str = "slot.container";
const MOST: i32 = 64;

pub(crate) struct ReplaceItem {
    position: BlockCoords,
    slot: i32,
    item: String,
    /// How BDS reports the item when the game does not have it.
    item_wrong: ParseError,
    amount: Option<i32>,
}

/// The next word, which must be `word` (in any case).
fn literal(args: &mut Args<'_>, word: &str) -> Result<(), ParseError> {
    let mark = args.mark();
    args.word()?.eq_ignore_ascii_case(word).then_some(()).ok_or_else(|| args.error_at(mark))
}

impl CommandSyntax for ReplaceItem {
    fn spec() -> CommandSpec {
        let parameters = vec![
            ParameterSpec::literal("replaceitem", "block"),
            ParameterSpec::of::<BlockCoords>("position"),
            ParameterSpec::literal("replaceitem", SLOT_TYPE),
            ParameterSpec::of::<i32>("slotId"),
            ParameterSpec { name: "itemName".into(), kind: ParamKind::String, optional: false },
            ParameterSpec::of::<Option<i32>>("amount"),
            ParameterSpec::of::<Option<i32>>("data"),
        ];
        CommandSpec {
            name: "replaceitem".into(),
            aliases: Vec::new(),
            description: "Replaces items in inventories".into(),
            permission: Permission::Operator,
            overloads: vec![Overload { parameters }],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        literal(args, "block")?;
        let position = BlockCoords::parse(args)?;
        literal(args, SLOT_TYPE)?;
        let slot = i32::parse(args)?;
        let mark = args.mark();
        let item = args.word()?.to_owned();
        let item_wrong = args.error_at(mark);
        let amount = Option::<i32>::parse(args)?;
        let _data = Option::<i32>::parse(args)?;
        args.finish()?;
        Ok(Self { position, slot, item, item_wrong, amount })
    }
}

impl Command for ReplaceItem {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let count = self.amount.unwrap_or(1);
        if count > MOST {
            return out.translate("commands.generic.num.tooBig", vec![count.to_string(), MOST.to_string()], false);
        }
        let pos = self.position.from(origin(ctx, sender));
        let Some(size) = ctx.container_size(pos) else {
            let [x, y, z] = pos;
            return out.translate("commands.replaceitem.noContainer", vec![format!("Pos({x},{y},{z})")], false);
        };
        let Some(slot) = usize::try_from(self.slot).ok().filter(|slot| *slot < size) else {
            return out.translate("commands.replaceitem.badSlotNumber", vec![self.slot.to_string(), "0".into(), (size - 1).to_string()], false);
        };
        let name = if self.item.contains(':') { self.item.clone() } else { format!("minecraft:{}", self.item) };
        let item = (name != "minecraft:air").then(|| Item::new(&name, count.clamp(1, MOST) as u8));
        if !ctx.set_container_item(pos, slot, item.as_ref()) {
            return out.refuse(self.item_wrong);
        }
        out.translate("commands.replaceitem.success", vec![SLOT_TYPE.into(), slot.to_string(), count.to_string(), display_name(&name)], true);
    }
}
