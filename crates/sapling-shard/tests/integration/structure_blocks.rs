//! Structure blocks (blocks/structure_blocks.rs; docs/research/structure-blocks.md).

use crate::common;

use acacia_proto::packets::{
    BlockEntityData, StructureBlockUpdate, StructureTemplateDataExportRequest, StructureTemplateDataExportRequestRequestType as Request, StructureTemplateDataExportResponse, UpdateBlock,
};
use acacia_proto::types::{
    BlockCoordinates, StructureBlockSettings, StructureBlockSettingsAnimationMode as Animation, StructureBlockSettingsMirror, StructureBlockSettingsRotation, Vec3f,
};
use common::shooting::{ALEX, ready};
use common::{Harness, all};
use sapling_api::{Block, BlockPos, GameMode, Permission};

const SAVER: BlockPos = [2, 65, 0];
const LOADER: BlockPos = [2, 65, 6];
const SAVE: i32 = 1;
const LOAD: i32 = 2;

fn coords([x, y, z]: [i32; 3]) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

fn settings(animation: Animation, seconds: f32) -> StructureBlockSettings {
    StructureBlockSettings {
        palette_name: "default".into(),
        ignore_entities: false,
        ignore_blocks: false,
        non_ticking_players_and_ticking_areas: false,
        size: coords([3, 3, 3]),
        structure_offset: coords([1, 0, 0]),
        last_editing_player_unique_id: 0,
        rotation: StructureBlockSettingsRotation::None,
        mirror: StructureBlockSettingsMirror::None,
        animation_mode: animation,
        animation_duration: seconds,
        integrity: 100.0,
        seed: 0,
        pivot: Vec3f { x: 1.0, y: 1.0, z: 1.0 },
    }
}

/// An editor's update of the block at `pos` to `mode`, its box the 3 cells east of it.
fn update(pos: BlockPos, mode: i32, name: &str, animation: Animation, trigger: bool) -> StructureBlockUpdate {
    StructureBlockUpdate {
        position: coords(pos),
        structure_name: name.into(),
        filtered_structure_name: String::new(),
        data_field: String::new(),
        include_players: false,
        show_bounding_box: true,
        structure_block_type: mode,
        settings: settings(animation, 2.0),
        redstone_save_mode: 0,
        should_trigger: trigger,
        water_logged: false,
    }
}

/// Alex, a creative operator, with a structure block at `SAVER` and at `LOADER`, stone at the
/// bottom of the box east of `SAVER` and glass at its top.
fn set_up() -> Harness {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.remote.run(|ctx| {
        ctx.set_permission(ALEX, Permission::Operator);
        for pos in [SAVER, LOADER] {
            assert!(ctx.set_block(pos, &Block::new("minecraft:structure_block")));
        }
        assert!(ctx.set_block([3, 65, 0], &Block::new("minecraft:stone")));
        assert!(ctx.set_block([4, 67, 1], &Block::new("minecraft:glass")));
    });
    h.tick_all();
    h.tick_all();
    h
}

fn block_at(h: &mut Harness, pos: BlockPos) -> String {
    let (send, receive) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| send.send(ctx.block(pos).map(|block| format!("{}[{}]", block.name, block.properties)).unwrap_or_default()).unwrap());
    h.tick_all();
    receive.recv().unwrap()
}

fn structures(h: &mut Harness) -> Vec<String> {
    let (send, receive) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| send.send(ctx.structures()).unwrap());
    h.tick_all();
    receive.recv().unwrap()
}

#[test]
fn a_block_put_as_load_is_shown_then_turned_save_a_tick_on() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_block(LOADER, &Block::new("minecraft:structure_block").with("structure_block_type=load"))));
    let put = h.tick_all().to(1).to_vec();
    let again = h.tick_all().to(1).to_vec();
    for sent in [&put, &again] {
        assert_eq!((all::<UpdateBlock>(sent).len(), all::<BlockEntityData>(sent).len()), (1, 1));
    }
    assert!(block_at(&mut h, LOADER).contains("structure_block_type=save"));
}

#[test]
fn the_editor_saves_by_the_name_as_typed_and_loads_it() {
    let mut h = set_up();
    h.receive(1, &update(SAVER, SAVE, "sb1", Animation::None, true));
    let shown = h.tick_all().to(1).to_vec();
    assert_eq!((all::<UpdateBlock>(&shown).len(), all::<BlockEntityData>(&shown).len()), (1, 1), "the update shown back");
    assert_eq!(structures(&mut h), ["sb1"], "kept unnamespaced, as BDS kept it");

    h.receive(1, &update(LOADER, LOAD, "sb1", Animation::None, true));
    h.tick_all();
    assert_eq!([block_at(&mut h, [3, 65, 6]), block_at(&mut h, [4, 67, 7])], ["minecraft:stone[]", "minecraft:glass[]"]);

    for (name, found) in [("sb1", true), ("nosuch", false)] {
        let request = StructureTemplateDataExportRequest { name: name.into(), position: coords(SAVER), settings: settings(Animation::None, 0.0), request_type: Request::QuerySavedStructure };
        h.receive(1, &request);
        let answer = all::<StructureTemplateDataExportResponse>(h.tick_all().to(1)).pop().expect("answered");
        assert_eq!((answer.success, answer.nbt.is_some()), (found, found), "{name}");
    }
}

#[test]
fn power_saves_a_save_block() {
    let mut h = set_up();
    h.receive(1, &update(SAVER, SAVE, "sb4", Animation::None, false));
    h.tick_all();
    assert!(structures(&mut h).is_empty());
    h.remote.run(|ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("minecraft:redstone_block"))));
    (0..4).for_each(|_| drop(h.tick_all()));
    assert_eq!(structures(&mut h), ["sb4"]);
}

#[test]
fn a_layer_by_layer_load_puts_a_layer_each_third_of_its_time() {
    let mut h = set_up();
    h.receive(1, &update(SAVER, SAVE, "anim", Animation::None, true));
    h.tick_all();
    h.receive(1, &update(LOADER, LOAD, "anim", Animation::Layers, true));
    h.tick_all();
    let (bottom, top) = ([3, 65, 6], [4, 67, 7]);
    (0..8).for_each(|_| drop(h.tick_all()));
    assert_eq!(block_at(&mut h, bottom), "minecraft:air[]", "nothing before 2 s / 3");
    (0..10).for_each(|_| drop(h.tick_all()));
    assert_eq!([block_at(&mut h, bottom), block_at(&mut h, top)], ["minecraft:stone[]", "minecraft:air[]"]);
    (0..25).for_each(|_| drop(h.tick_all()));
    assert_eq!(block_at(&mut h, top), "minecraft:glass[]");
}
