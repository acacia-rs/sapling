//! Blocks that hold a stack leave it whole when they break: by hand, by losing their support and in
//! a blast (blocks/entities.rs, blocks/loot.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::AddItemEntity;
use acacia_proto::types::ItemV4Extra;
use common::asking::ask;
use common::stored::compound;
use common::{Harness, all};
use sapling_api::{Block, BlockEntityData};

fn stack(name: &str, tag: Value) -> Value {
    compound([("Name", Value::String(name.into())), ("Count", Value::Byte(1)), ("Damage", Value::Short(0)), ("tag", tag)])
}

fn enchanted(name: &str) -> Value {
    let sharpness = compound([("id", Value::Short(9)), ("lvl", Value::Short(5))]);
    stack(name, compound([("ench", Value::List(List { tag: 10, items: vec![sharpness] }))]))
}

/// Puts `block` at `pos` holding the entity `fields`.
fn put(h: &mut Harness, block: Block, pos: [i32; 3], fields: Vec<(&str, Value)>) {
    let mut fields: Vec<_> = fields.into_iter().map(|(key, value)| (key.into(), value)).collect();
    fields.extend(["x", "y", "z"].into_iter().zip(pos).map(|(axis, at)| (axis.into(), Value::Int(at))));
    let entity = Value::Compound(fields);
    let done = ask(h, move |ctx| (ctx.set_block(pos, &block), ctx.set_block_entity(pos, &BlockEntityData::new(entity))));
    assert_eq!(done, (true, true), "{pos:?}");
}

/// The tag of each stack of `item` left lying, as text.
fn left_tags(sent: &[RawPacket], item: &str) -> Vec<String> {
    let id = sapling_data::item(&sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None }, None).expect("an item").network_id;
    all::<AddItemEntity>(sent)
        .into_iter()
        .filter(|left| left.item.network_id == id)
        .map(|left| match &left.item.extra {
            ItemV4Extra::Default(Some(extra)) => format!("{:?}", extra.nbt),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn a_lectern_broken_leaves_its_book_with_its_pages() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let page = compound([("text", Value::String("held page".into())), ("photoname", Value::String("".into()))]);
    let book = stack("minecraft:written_book", compound([("pages", Value::List(List { tag: 10, items: vec![page] })), ("title", Value::String("Kept".into()))]));
    let fields = vec![("id", Value::String("Lectern".into())), ("book", book), ("hasBook", Value::Byte(1)), ("page", Value::Int(0)), ("totalPages", Value::Int(1))];
    put(&mut h, Block::new("minecraft:lectern"), [2, 65, 0], fields);
    h.remote.run(|ctx| assert!(ctx.break_block([2, 65, 0])));
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let tags = left_tags(&sent, "minecraft:written_book");
    assert!(tags.len() == 1 && tags[0].contains("held page") && tags[0].contains("Kept"), "{tags:?}");
}

#[test]
fn a_frame_that_loses_its_block_leaves_its_item_enchanted() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let fields = vec![("id", Value::String("ItemFrame".into())), ("Item", enchanted("minecraft:diamond_sword")), ("ItemRotation", Value::Float(0.0)), ("ItemDropChance", Value::Float(1.0))];
    put(&mut h, Block::new("minecraft:frame").with("facing_direction=1,item_frame_map_bit=0,item_frame_photo_bit=0"), [2, 65, 0], fields);
    h.remote.run(|ctx| assert!(ctx.break_block([2, 64, 0])));
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(ask(&mut h, |ctx| ctx.block([2, 65, 0]).is_some_and(|gone| gone.is_air())), "the frame fell");
    let tags = left_tags(&sent, "minecraft:diamond_sword");
    assert!(tags.len() == 1 && tags[0].contains("ench"), "{tags:?}");
    assert_eq!(left_tags(&sent, "minecraft:frame").len(), 1);
}

#[test]
fn a_blasted_chiseled_bookshelf_leaves_its_enchanted_books_whole() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let empty = compound([("Name", Value::String("".into())), ("Count", Value::Byte(0)), ("Damage", Value::Short(0))]);
    let mut books = vec![enchanted("minecraft:enchanted_book"), stack("minecraft:book", compound([])), enchanted("minecraft:enchanted_book")];
    books.extend(std::iter::repeat_n(empty, 3));
    let fields = vec![("id", Value::String("ChiseledBookshelf".into())), ("Items", Value::List(List { tag: 10, items: books })), ("LastInteractedSlot", Value::Int(0))];
    put(&mut h, Block::new("minecraft:chiseled_bookshelf").with("books_stored=7,direction=0"), [2, 65, 0], fields);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:tnt", [0.5, 65.0, 0.5]).is_some()));
    let sent: Vec<RawPacket> = (0..90).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let tags = left_tags(&sent, "minecraft:enchanted_book");
    assert!(tags.len() == 2 && tags.iter().all(|tag| tag.contains("ench")), "{tags:?}");
}
