//! Command blocks (blocks/command_blocks.rs; docs/research/command-blocks.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{
    BlockEntityData, CommandBlockUpdate, PlayerAuthInput, CommandBlockUpdateContent, CommandBlockUpdateContentTrue, CommandBlockUpdateContentTrueMode as Mode, ContainerOpen, Text, TextContent, UpdateBlock,
};
use acacia_proto::types::{BlockCoordinates, InputData, WindowType};
use common::shooting::{ALEX, STEVE, ready};
use common::{Harness, all, click};
use sapling_api::{Block, BlockPos, Command, GameMode, Output, Permission, RuleValue, Sender, ShardCtx};

const HEAD: BlockPos = [2, 65, 0];
const LINK: BlockPos = [3, 65, 0];
const WOOL: &str = "minecraft:white_wool";

/// Puts wool under where it runs from.
#[derive(Command)]
#[command(crate = "sapling_api", permission = "operator")]
struct Stamp;

impl Command for Stamp {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let [x, y, z] = ctx.origin(sender).map(|c| c.floor() as i32);
        ctx.set_block([x, y - 1, z], &Block::new(WOOL));
        out.print("stamped");
    }
}

static COUNTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Counts its runs in [`COUNTED`].
#[derive(Command)]
#[command(crate = "sapling_api", permission = "operator")]
struct Count;

impl Command for Count {
    fn run(self, _: &mut ShardCtx<'_>, _: &Sender, _: &mut Output) {
        COUNTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn a_repeating_block_stops_once_nobody_is_near_as_bds_keeps_no_spawn_chunks() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.shard.add_command::<Count>();
    h.remote.run(|ctx| {
        ctx.set_permission(ALEX, Permission::Operator);
        assert!(ctx.set_block(HEAD, &Block::new("minecraft:repeating_command_block").with("conditional_bit=0,facing_direction=5")));
    });
    h.tick_all();
    h.receive(1, &update(HEAD, Mode::Repeat, false, "count"));
    let ran = |h: &mut Harness, ticks: usize| {
        let before = COUNTED.load(std::sync::atomic::Ordering::Relaxed);
        (0..ticks).for_each(|_| drop(h.tick_all()));
        COUNTED.load(std::sync::atomic::Ordering::Relaxed) - before
    };
    assert!(ran(&mut h, 40) >= 35, "it runs every tick");
    const FAR: [f32; 3] = [3000.5, 65.0, 0.5];
    h.remote.run(|ctx| [ALEX, STEVE].into_iter().for_each(|player| ctx.teleport(player, FAR)));
    h.tick_all();
    for client in [1, 2] {
        h.receive(client, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..common::input_at(FAR) });
    }
    ran(&mut h, 220);
    assert_eq!(ran(&mut h, 100), 0, "research/captures/bds-spawnchunks.txt: stopped within a second");
}

fn update(pos: BlockPos, mode: Mode, needs_redstone: bool, command: &str) -> CommandBlockUpdate {
    let content = CommandBlockUpdateContent::True(CommandBlockUpdateContentTrue { position: BlockCoordinates { x: pos[0], y: pos[1], z: pos[2] }, mode, needs_redstone, conditional: false });
    CommandBlockUpdate {
        is_block: true,
        content,
        command: command.into(),
        last_output: String::new(),
        name: String::new(),
        filtered_name: String::new(),
        should_track_output: true,
        tick_delay: 0,
        execute_on_first_tick: false,
    }
}

/// Alex, an operator in creative mode unless `member`, with an impulse block at `HEAD` and a chain
/// block after it, both facing east.
fn set_up(member: bool) -> Harness {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.shard.add_command::<Stamp>();
    h.remote.run(move |ctx| {
        ctx.set_permission(ALEX, if member { Permission::Member } else { Permission::Operator });
        assert!(ctx.set_block(HEAD, &Block::new("minecraft:command_block").with("conditional_bit=0,facing_direction=5")));
        assert!(ctx.set_block(LINK, &Block::new("minecraft:chain_command_block").with("conditional_bit=0,facing_direction=5")));
    });
    h.tick_all();
    h.tick_all();
    h
}

fn entity(packets: &[RawPacket], key: &str) -> Option<Value> {
    all::<BlockEntityData>(packets).pop()?.nbt.decode().ok()?.value.get(key).cloned()
}

fn block_at(h: &mut Harness, pos: BlockPos) -> String {
    let (send, receive) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| send.send(ctx.block(pos).map(|block| block.name).unwrap_or_default()).unwrap());
    h.tick_all();
    receive.recv().unwrap()
}

#[test]
fn a_put_command_block_is_shown_with_its_entity_and_again_a_tick_on() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_block(HEAD, &Block::new("minecraft:repeating_command_block"))));
    let put = h.tick_all().to(1).to_vec();
    let again = h.tick_all().to(1).to_vec();
    for sent in [&put, &again] {
        assert_eq!((all::<UpdateBlock>(sent).len(), all::<BlockEntityData>(sent).len()), (1, 1));
    }
    assert_eq!(entity(&put, "Version"), Some(Value::Int(52)));
    assert_eq!(entity(&put, "ExecuteOnFirstTick"), Some(Value::Byte(1)), "a repeating block runs on its first tick");
    assert_eq!(entity(&put, "auto"), Some(Value::Byte(0)));
    assert!(all::<UpdateBlock>(h.tick_all().to(1)).is_empty(), "shown twice only");
}

#[test]
fn power_runs_the_block_then_its_chain_and_operators_are_told() {
    let mut h = set_up(false);
    h.receive(1, &update(HEAD, Mode::Impulse, true, "stamp"));
    h.receive(1, &update(LINK, Mode::Chain, false, "stamp"));
    assert_eq!(all::<BlockEntityData>(h.tick_all().to(1)).len(), 2, "each update shown back");
    assert_eq!(block_at(&mut h, [2, 64, 0]), "minecraft:stone", "nothing runs unpowered");

    h.remote.run(|ctx| assert!(ctx.set_block([2, 66, 0], &Block::new("minecraft:redstone_block"))));
    let told: Vec<RawPacket> = (0..4).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!([block_at(&mut h, [2, 64, 0]), block_at(&mut h, [3, 64, 0])], [WOOL, WOOL]);
    let echoed = all::<Text>(&told).into_iter().filter(|text| matches!(&text.content, TextContent::Json(json) if json.message.contains("!§r"))).count();
    assert_eq!(echoed, 2, "each run shown to the operator as the block's");

    // A block in hand builds nothing: BDS opened the editor (bds-cmdblocks.txt).
    h.receive(1, &click(HEAD, 2, Some("minecraft:command_block")));
    let opened = h.tick_all().to(1).to_vec();
    assert_eq!(block_at(&mut h, [2, 65, -1]), "minecraft:air");
    assert_eq!(entity(&opened, "SuccessCount"), Some(Value::Int(1)));
    assert_eq!(entity(&opened, "LastOutput"), Some(Value::String("stamped".into())));
    assert_eq!(all::<ContainerOpen>(&opened).pop().map(|open| open.window_type), Some(WindowType::CommandBlock));
}

#[test]
fn no_link_runs_with_the_chain_length_at_zero_and_nothing_with_blocks_off() {
    for rule in ["maxCommandChainLength", "commandBlocksEnabled"] {
        let mut h = set_up(false);
        h.remote.run(move |ctx| assert!(ctx.set_game_rule(rule, if rule.starts_with("max") { RuleValue::Int(0) } else { RuleValue::Bool(false) })));
        h.receive(1, &update(HEAD, Mode::Impulse, true, "stamp"));
        h.receive(1, &update(LINK, Mode::Chain, false, "stamp"));
        h.remote.run(|ctx| assert!(ctx.set_block([2, 66, 0], &Block::new("minecraft:redstone_block"))));
        (0..4).for_each(|_| drop(h.tick_all()));
        let head = if rule.starts_with("max") { WOOL } else { "minecraft:stone" };
        assert_eq!([block_at(&mut h, [2, 64, 0]), block_at(&mut h, [3, 64, 0])], [head, "minecraft:stone"], "{rule}");
    }
}

#[test]
fn a_member_neither_edits_nor_opens_one() {
    let mut h = set_up(true);
    h.receive(1, &update(HEAD, Mode::Impulse, false, "stamp"));
    h.receive(1, &click(HEAD, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    assert!(all::<BlockEntityData>(&sent).is_empty() && all::<ContainerOpen>(&sent).is_empty(), "BDS sent a survival click nothing");
    (0..3).for_each(|_| drop(h.tick_all()));
    assert_eq!(block_at(&mut h, [2, 64, 0]), "minecraft:stone");
}

#[test]
fn a_command_that_does_not_parse_keeps_the_syntax_error() {
    let mut h = set_up(false);
    h.receive(1, &update(HEAD, Mode::Impulse, true, "nosuchcommand"));
    let shown = h.tick_all().to(1).to_vec();
    assert_eq!(entity(&shown, "LastOutput"), Some(Value::String("commands.generic.syntax".into())));
}
