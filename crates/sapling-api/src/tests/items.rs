use crate::testing::{Fake, player};
use crate::{DropEvent, Event, Handler, Item, Player, ShardCtx, Slot};

/// Hands out a compass on join and keeps players from throwing it away.
struct Kit;

impl Handler for Kit {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.clear_inventory(player.id);
        ctx.set_item(player.id, Slot::Main(2), Some(&Item::new("minecraft:compass", 1)));
        ctx.give(player.id, &Item::new("minecraft:bread", 16));
    }

    fn on_drop(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut DropEvent) {
        if ctx.held_item(player.id).as_ref() == Some(event.item()) && event.item().name == "minecraft:compass" {
            event.cancel();
        }
    }
}

#[test]
fn a_handler_fills_an_inventory_and_refuses_a_drop() {
    let (mut fake, steve) = (Fake::with(&["Steve"]), player(1, "Steve"));
    Kit.on_join(&mut ShardCtx::new(&mut fake), &steve);
    assert_eq!(fake.log, ["clear 1", "item 1 Main(2) Some((\"minecraft:compass\", 1))", "give 1 minecraft:bread x16"]);

    let mut ctx = ShardCtx::new(&mut fake);
    assert_eq!((ctx.held_slot(steve.id), ctx.item(steve.id, Slot::Offhand)), (Some(2), None));
    let (mut compass, mut bread) = (DropEvent::new(player(1, "Steve"), Item::new("minecraft:compass", 1)), DropEvent::new(player(1, "Steve"), Item::new("minecraft:bread", 3)));
    Kit.on_drop(&mut ctx, &steve, &mut compass);
    Kit.on_drop(&mut ctx, &steve, &mut bread);
    assert!(compass.is_cancelled() && compass.is_stopped() && !bread.is_cancelled());
    assert!(ctx.set_item(steve.id, Slot::Main(2), None) && ctx.held_item(steve.id).is_none());
}
