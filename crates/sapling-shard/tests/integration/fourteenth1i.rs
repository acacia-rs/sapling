//! Group I of the fourteenth brief (research/captures/bds-i1place.txt, bds-i1stay.txt, bds-i1misc.txt,
//! bds-i1trip.txt): support by face, a wall's outline rule, a slab under a top slab, hanging
//! propagules, a burnt torch's broken loop, spawner and sculk experience, and tripwires.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Action as BlockAction;
use common::asking::ask;
use common::{Harness, all, block_actions, block_changes, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

/// Puts `blocks` in the next tick, the first that [`watch`] sees.
fn put(h: &mut Harness, blocks: &[([i32; 3], &str, &str)]) {
    let blocks: Vec<([i32; 3], Block)> = blocks.iter().map(|(pos, name, properties)| (*pos, Block::new(format!("minecraft:{name}")).with(*properties))).collect();
    h.remote.run(move |ctx| {
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{pos:?} {block:?}");
        }
    });
}

/// What Steve is shown in each of the next `ticks` ticks.
fn watch(h: &mut Harness, ticks: usize) -> Vec<Vec<RawPacket>> {
    (0..ticks).map(|_| h.tick_all().to(2).to_vec()).collect()
}

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    ask(h, move |ctx| cells.iter().map(|at| ctx.block(*at).map(|block| block.name.replace("minecraft:", "")).unwrap_or_default()).collect())
}

fn properties(h: &mut Harness, at: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(at).map(|block| block.properties).unwrap_or_default())
}

fn state(name: &str, properties: &str) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(name, properties).unwrap_or_else(|| panic!("{name} [{properties}]")))
}

fn hold(h: &mut Harness, item: &'static str, mode: GameMode) {
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, mode);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, 16))));
    });
    h.tick_all();
}

const SUPPORTERS: [([i32; 3], &str, &str); 5] =
    [([0, 65, 2], "smooth_stone_slab", "minecraft:vertical_half=top"), ([2, 65, 2], "oak_leaves", ""), ([4, 65, 2], "oak_fence", ""), ([6, 65, 2], "cauldron", ""), ([8, 65, 2], "glass", "")];

fn on_each(name: &'static str, properties: &'static str) -> Vec<([i32; 3], &'static str, &'static str)> {
    SUPPORTERS.iter().map(|([x, y, z], ..)| ([*x, y + 1, *z], name, properties)).collect()
}

#[test]
fn a_face_holds_by_the_support_its_block_gives_what_stands_on_it() {
    let mut h = ready();
    put(&mut h, &SUPPORTERS);
    watch(&mut h, 2);
    let tops: Vec<[i32; 3]> = (0..5).map(|i| [2 * i, 66, 2]).collect();
    for (name, properties, kept) in [("torch", "torch_facing_direction=top", "torch"), ("stone_pressure_plate", "", "stone_pressure_plate"), ("flower_pot", "", "flower_pot")] {
        put(&mut h, &on_each(name, properties));
        watch(&mut h, 3);
        let expected: Vec<&str> = match name {
            "stone_pressure_plate" => vec![kept, "air", kept, kept, kept],
            _ => vec![kept, "air", kept, "air", kept],
        };
        assert_eq!(names(&mut h, tops.clone()), expected, "{name}: a top slab, leaves, a fence, a cauldron, glass (bds-i1stay.txt)");
        put(&mut h, &on_each("air", ""));
        watch(&mut h, 2);
    }
    // A lever wants the centre and the edge: a fence's top has no edge.
    put(&mut h, &[([2, 65, -1], "oak_fence", ""), ([-2, 65, -1], "glass", "")]);
    hold(&mut h, "minecraft:lever", GameMode::Creative);
    for supporter in [[2, 65, -1], [-2, 65, -1]] {
        h.receive(1, &click(supporter, 1, Some("minecraft:lever")));
        watch(&mut h, 2);
    }
    assert_eq!(names(&mut h, vec![[2, 66, -1], [-2, 66, -1]]), ["air", "lever"], "bds-i1place.txt");
}

#[test]
fn a_wall_reads_the_outline_of_what_stands_on_it() {
    let mut h = ready();
    put(&mut h, &[([-1, 65, 4], "cobblestone_wall", ""), ([0, 65, 4], "cobblestone_wall", ""), ([1, 65, 4], "cobblestone_wall", "")]);
    watch(&mut h, 2);
    put(&mut h, &[([0, 66, 4], "stone_pressure_plate", "")]);
    watch(&mut h, 3);
    let wall = properties(&mut h, [0, 65, 4]);
    assert!(wall.contains("wall_connection_type_east=tall") && wall.contains("wall_post_bit=0"), "a plate's outline reaches the arms (bds-i1stay.txt): {wall}");
    put(&mut h, &[([0, 66, 4], "torch", "torch_facing_direction=top")]);
    watch(&mut h, 3);
    let wall = properties(&mut h, [0, 65, 4]);
    assert!(wall.contains("wall_connection_type_east=short") && wall.contains("wall_post_bit=1"), "a torch's only the post: {wall}");
}

#[test]
fn a_slab_put_under_a_top_slab_of_its_kind_makes_a_double_slab() {
    let mut h = ready();
    put(&mut h, &[([2, 67, 0], "smooth_stone_slab", "minecraft:vertical_half=top")]);
    watch(&mut h, 1);
    hold(&mut h, "minecraft:smooth_stone_slab", GameMode::Creative);
    h.receive(1, &click([2, 67, 0], 0, None));
    watch(&mut h, 2);
    assert_eq!(names(&mut h, vec![[2, 67, 0], [2, 66, 0]]), ["smooth_stone_double_slab", "air"], "bds-i1misc.txt");
    assert_eq!(properties(&mut h, [2, 67, 0]), "minecraft:vertical_half=bottom");
}

#[test]
fn bone_meal_ages_a_hanging_propagule_a_stage_a_click_up_to_4() {
    let mut h = ready();
    put(&mut h, &[([2, 68, 0], "mangrove_leaves", "persistent_bit=1,update_bit=0"), ([2, 67, 0], "mangrove_propagule", "hanging=1,propagule_stage=0")]);
    watch(&mut h, 1);
    hold(&mut h, "minecraft:bone_meal", GameMode::Survival);
    let mut stages = Vec::new();
    for _ in 0..5 {
        h.receive(1, &click([2, 67, 0], 0, None));
        let shown = watch(&mut h, 1);
        let grew = all::<LevelEvent>(&shown[0]).iter().any(|event| event.event == LevelEventEvent::ParticleCropGrowth);
        stages.push((properties(&mut h, [2, 67, 0]), grew));
    }
    let expected: Vec<(String, bool)> = [1, 2, 3, 4, 4].iter().enumerate().map(|(i, stage)| (format!("hanging=1,propagule_stage={stage}"), i < 4)).collect();
    assert_eq!(stages, expected, "bds-i1misc.txt: a stage a click, refused at 4");
}

#[test]
fn a_burnt_torch_lights_again_when_its_loop_is_broken() {
    let mut h = ready();
    put(&mut h, &[([6, 65, 4], "stone", ""), ([7, 66, 4], "stone", ""), ([5, 66, 4], "redstone_lamp", ""), ([6, 66, 4], "redstone_wire", "redstone_signal=0")]);
    watch(&mut h, 1);
    put(&mut h, &[([7, 65, 4], "redstone_torch", "torch_facing_direction=west")]);
    watch(&mut h, 80);
    assert_eq!(names(&mut h, vec![[7, 65, 4]]), ["unlit_redstone_torch"], "burnt out");
    put(&mut h, &[([6, 66, 4], "air", "")]);
    let out = watch(&mut h, 8);
    let lit = state("minecraft:redstone_torch", "torch_facing_direction=west");
    let relit = out.iter().position(|sent| block_changes(sent).contains(&([7, 65, 4], lit)));
    assert!(relit.is_some_and(|tick| tick <= 4), "BDS relit it 3 ticks after the dust went (bds-i1misc.txt): {relit:?}");
}

#[test]
fn a_spawner_and_sculk_broken_leave_their_experience() {
    let mut h = ready();
    put(&mut h, &[([2, 65, 0], "mob_spawner", ""), ([4, 65, 0], "sculk_sensor", ""), ([6, 65, 0], "sculk_vein", "")]);
    watch(&mut h, 1);
    let mut left = Vec::new();
    for at in [[2, 65, 0], [4, 65, 0], [6, 65, 0]] {
        h.remote.run(move |ctx| assert!(ctx.break_block(at)));
        left.push(watch(&mut h, 2).iter().flat_map(|sent| common::orbs::worths(sent)).sum::<i32>());
    }
    assert!((15..=43).contains(&left[0]) && left[1..] == [5, 0], "bds-i1misc.txt: {left:?}");
}

#[test]
fn a_falling_anvil_hurts_a_cow_under_it() {
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, -3.5]).is_some()));
    watch(&mut h, 1);
    put(&mut h, &[([4, 69, -4], "anvil", "minecraft:cardinal_direction=south")]);
    let out = watch(&mut h, 30);
    let hurt = out.iter().any(|sent| all::<EntityEvent>(sent).iter().any(|event| event.event_id == EntityEventEventId::HurtAnimation));
    assert!(hurt, "bds-i1anvil.txt: a cow under a drop of 4 was hurt");
}

fn hook(direction: u8, attached: bool, powered: bool) -> u32 {
    state("minecraft:tripwire_hook", &format!("attached_bit={},direction={direction},powered_bit={}", u8::from(attached), u8::from(powered)))
}

/// Hooks at x -3 and 3 on stones, string between, at `z`.
fn line(h: &mut Harness, z: i32) {
    put(h, &[([-4, 65, z], "stone", ""), ([4, 65, z], "stone", ""), ([-3, 65, z], "tripwire_hook", "attached_bit=0,direction=3,powered_bit=0"), ([3, 65, z], "tripwire_hook", "attached_bit=0,direction=1,powered_bit=0")]);
    put(h, &(-2..=2).map(|x| ([x, 65, z], "trip_wire", "")).collect::<Vec<_>>());
    watch(h, 2);
    assert_eq!(properties(h, [3, 65, z]), "attached_bit=1,direction=1,powered_bit=0");
}

#[test]
fn a_wire_broken_by_hand_trips_its_hooks_which_let_go_10_ticks_on() {
    let mut h = ready();
    line(&mut h, 3);
    hold(&mut h, "minecraft:dirt", GameMode::Survival);
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, [0, 65, 3])]));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [0, 65, 3])]));
    let out = watch(&mut h, 14);
    let tripped = out.iter().position(|sent| block_changes(sent).contains(&([3, 65, 3], hook(1, true, true)))).expect("tripped");
    let sounds: Vec<String> = all::<LevelSoundEvent>(&out[tripped]).iter().map(|sound| sound.sound_id.clone()).filter(|id| id.starts_with("power")).collect();
    assert_eq!(sounds, ["power.on", "power.on"], "far hook, then near (bds-i1trip.txt)");
    let let_go = out.iter().position(|sent| block_changes(sent).contains(&([3, 65, 3], hook(1, false, false))));
    assert_eq!(let_go, Some(tripped + 10));

    // A wire taken away by command tells no hook (a fresh line: the dropped string lies in the old one).
    line(&mut h, 6);
    put(&mut h, &[([0, 65, 6], "air", "")]);
    watch(&mut h, 12);
    assert_eq!(properties(&mut h, [3, 65, 6]), "attached_bit=1,direction=1,powered_bit=0", "bds-i1trip.txt");
}
