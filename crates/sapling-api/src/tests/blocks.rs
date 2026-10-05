use crate::testing::{Fake, player};
use crate::{Block, BreakEvent, Cancellable, Face, Handler, InteractEvent, PlaceEvent, Player, ShardCtx};

/// Lets "Steve" build and nobody else; turns placed sand into glass and marks what was broken.
struct Plot;

impl Handler for Plot {
    fn on_break(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut BreakEvent) {
        if player.name != "Steve" {
            return event.cancel();
        }
        event.allow();
        let [x, y, z] = event.pos();
        ctx.set_block([x, y + 1, z], &Block::new("torch").with("torch_facing_direction=top"));
    }

    fn on_place(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut PlaceEvent) {
        let (against, _) = event.against();
        if player.name != "Steve" || ctx.block(against).is_none_or(|block| block.is_air()) {
            return event.cancel();
        }
        event.allow();
        if event.block.name == "minecraft:sand" {
            event.block = Block::new("glass");
        }
    }
}

#[test]
fn block_events_end_as_the_last_handler_left_them() {
    let mut event = BreakEvent::new([1, 2, 3], Block::new("stone"), Vec::new(), false);
    assert!(event.is_cancelled() && !event.is_stopped(), "refused by default, yet every handler sees it");
    event.allow();
    assert!(!event.is_cancelled() && !event.is_stopped());
    event.cancel();
    assert!(event.is_cancelled() && event.is_stopped(), "a handler's cancel skips the rest");
    event.allow();
    assert!(!event.is_cancelled() && !event.is_stopped());
    assert_eq!((event.pos(), event.block().name.as_str()), ([1, 2, 3], "minecraft:stone"));

    let mut click = InteractEvent::new([0, 64, 0], Face::North, Block::new("minecraft:lever").with("open_bit=0"), true);
    assert!(!click.is_cancelled() && click.face() == Face::North && click.block().properties == "open_bit=0");
    click.cancel();
    assert!(click.is_cancelled() && click.pos() == [0, 64, 0]);
}

#[test]
fn faces_point_at_their_neighbours() {
    let neighbours: Vec<_> = Face::ALL.iter().map(|face| face.neighbour([10, 64, -5])).collect();
    assert_eq!(neighbours, [[10, 63, -5], [10, 65, -5], [10, 64, -6], [10, 64, -4], [9, 64, -5], [11, 64, -5]]);
    assert_eq!((Face::from_index(1), Face::from_index(5), Face::from_index(6), Face::from_index(255)), (Some(Face::Up), Some(Face::East), None, None));
    assert_eq!(Face::Up.neighbour([0, i32::MAX, 0]), [0, i32::MAX, 0], "no overflow at the edge of the numbers");
    assert!(Block::air().is_air() && !Block::new("mod:air").is_air() && Block::new("mod:lamp").name == "mod:lamp");
}

#[test]
fn handlers_decide_about_blocks_and_change_the_world_through_the_context() {
    let mut backend = Fake::with(&["Steve", "Alex"]);
    let (steve, alex) = (player(1, "Steve"), player(2, "Alex"));
    let mut handler: Box<dyn Handler> = Box::new(Plot);
    assert!(ShardCtx::new(&mut backend).set_block([0, 64, 0], &Block::new("stone")));
    assert!(!ShardCtx::new(&mut backend).set_block([0, -1, 0], &Block::new("stone")), "no world there");
    assert!(!ShardCtx::new(&mut backend).set_block([0, 65, 0], &Block::new("mod:lamp")), "no such block");

    let mut broken = BreakEvent::new([0, 64, 0], Block::new("stone"), Vec::new(), false);
    handler.on_break(&mut ShardCtx::new(&mut backend), &alex, &mut broken);
    assert!(broken.is_cancelled());
    handler.on_break(&mut ShardCtx::new(&mut backend), &steve, &mut broken);
    assert!(!broken.is_cancelled());

    let mut placed = PlaceEvent::new([0, 65, 0], [0, 64, 0], Face::Up, Block::new("sand"), false);
    handler.on_place(&mut ShardCtx::new(&mut backend), &steve, &mut placed);
    assert!(!placed.is_cancelled() && placed.block == Block::new("glass") && placed.pos() == [0, 65, 0]);
    let mut floating = PlaceEvent::new([5, 65, 5], [5, 64, 5], Face::Up, Block::new("sand"), true);
    handler.on_place(&mut ShardCtx::new(&mut backend), &steve, &mut floating);
    assert!(floating.is_cancelled(), "nothing to build on");

    // The default methods leave an event as it came.
    let mut click = InteractEvent::new([0, 64, 0], Face::Up, Block::new("stone"), true);
    handler.on_interact(&mut ShardCtx::new(&mut backend), &alex, &mut click);
    assert!(!click.is_cancelled());
    assert_eq!(backend.log, ["block [0, 64, 0] minecraft:stone []", "block [0, 65, 0] minecraft:torch [torch_facing_direction=top]"]);
    assert_eq!(ShardCtx::new(&mut backend).block([0, 65, 0]).map(|b| b.name), Some("minecraft:torch".into()));
    assert!(ShardCtx::new(&mut backend).block([0, -1, 0]).is_none());
}
