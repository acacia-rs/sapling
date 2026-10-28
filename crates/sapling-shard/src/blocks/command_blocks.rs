//! Command blocks, as BDS kept and ran them (docs/research/command-blocks.md): the entity a put
//! block gets, shown again a tick on; an operator's editor in creative mode and the
//! CommandBlockUpdate it sends; power, which marks an impulse or repeating block's condition and
//! queues its run; the comparator, which reads its `SuccessCount`. Runs and chains: run.rs.

mod run;

use acacia_proto::nbt::{List, Str, Value};
use acacia_proto::packets::{CommandBlockUpdate, CommandBlockUpdateContent, CommandBlockUpdateContentTrueMode as UpdateMode};
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, GameMode, Permission};

use super::entity_fields::Fields;
use super::{level, send, shown_again};
use crate::commands::Access;
use crate::components::{InLayer, Mode, Outgoing, PacketQueues};
use crate::inventory::Inventory;

pub(super) use run::tick;

const ENTITY: &str = "CommandBlock";
/// BDS's `Version` of a command block's entity.
const VERSION: i32 = 52;
/// `TAG_String`, the element tag of `LastOutputParams` once it holds any.
const STRING: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Impulse,
    Repeating,
    Chain,
}

impl Kind {
    pub(super) fn of(block: &str) -> Option<Kind> {
        Some(match block.strip_prefix("minecraft:")? {
            "command_block" => Kind::Impulse,
            "repeating_command_block" => Kind::Repeating,
            "chain_command_block" => Kind::Chain,
            _ => return None,
        })
    }

    fn block(self) -> &'static str {
        match self {
            Kind::Impulse => "minecraft:command_block",
            Kind::Repeating => "minecraft:repeating_command_block",
            Kind::Chain => "minecraft:chain_command_block",
        }
    }
}

/// A command block's state: its kind, the way it faces and its `conditional_bit`.
pub(super) struct Shape {
    pub kind: Kind,
    pub facing: Face,
    pub conditional: bool,
}

pub(super) fn shape(world: &World, layer: Entity, pos: BlockPos) -> Option<Shape> {
    let block = send::block_at(world, layer, pos)?;
    let kind = Kind::of(&block.name)?;
    let get = |key: &str| block.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map(|(_, value)| value.to_owned());
    let facing = get("facing_direction").and_then(|face| face.parse().ok()).and_then(Face::from_index).unwrap_or(Face::Down);
    Some(Shape { kind, facing, conditional: get("conditional_bit").as_deref() == Some("1") })
}

/// A command block's entity, field by field.
pub(super) fn read(world: &World, layer: Entity, pos: BlockPos) -> Option<Fields> {
    Fields::read(world, layer, pos, ENTITY)
}

impl Fields {
    /// The entity BDS gave a command block put where none was: a chain block always active, a
    /// repeating one running on its first tick (research/captures/bds-cmdlinks.txt).
    fn fresh(kind: Kind, pos: BlockPos) -> Fields {
        let Value::Compound(mut fields) = sapling_world::new_block_entity(ENTITY, pos) else { unreachable!("a new entity is a compound") };
        let byte = |on: bool| Value::Byte(on.into());
        let text = || Value::String(Str::default());
        fields.extend(
            [
                ("BlockEntityVersion", Value::Int(0)),
                ("Command", text()),
                ("CustomName", text()),
                ("ExecuteOnFirstTick", byte(kind == Kind::Repeating)),
                ("LPCommandMode", Value::Int(0)),
                ("LPCondionalMode", byte(false)),
                ("LPRedstoneMode", byte(false)),
                ("LastExecution", Value::Long(0)),
                ("LastOutput", text()),
                ("LastOutputParams", Value::List(List { tag: 0, items: Vec::new() })),
                ("SuccessCount", Value::Int(0)),
                ("TickDelay", Value::Int(0)),
                ("TrackOutput", byte(true)),
                ("Version", Value::Int(VERSION)),
                ("auto", byte(kind == Kind::Chain)),
                ("conditionMet", byte(false)),
                ("conditionalMode", byte(false)),
                ("powered", byte(false)),
            ]
            .map(|(key, value)| (key.into(), value)),
        );
        Fields(fields)
    }

    /// Sets `LastOutput` and its parameters, as BDS keeps them: an empty list has no element tag.
    pub(super) fn set_output(&mut self, text: &str, parameters: &[String]) {
        self.put("LastOutput", Value::String(text.into()));
        let items: Vec<Value> = parameters.iter().map(|parameter| Value::String(parameter.as_str().into())).collect();
        self.put("LastOutputParams", Value::List(List { tag: if items.is_empty() { 0 } else { STRING }, items }));
    }
}

/// Gives a command block just put its entity: the one the cell held where that was a command
/// block's (a repeating block put over an impulse one kept it, shown once), else a fresh one,
/// shown now and again a tick on (research/captures/bds-cmdchains.txt).
pub(super) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, before: Option<&Value>) {
    let Some(shape) = shape(world, layer, pos) else { return };
    match before.and_then(|before| Fields::of(before, ENTITY)) {
        Some(kept) => {
            let starts = !kept.flag("powered") && kept.flag("auto");
            kept.show(world, layer, pos);
            if starts {
                run::start(world, layer, pos, &shape);
            }
        }
        None => {
            Fields::fresh(shape.kind, pos).show(world, layer, pos);
            shown_again::next_tick(world, layer, pos);
        }
    }
}

/// Whether a player may put, open and edit command blocks: an operator in creative mode, as on
/// BDS, which refused a survival place and gave a survival click nothing (bds-cmdblocks.txt).
pub(super) fn may_edit(world: &World, player: Entity) -> bool {
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    creative && world.get::<Access>(player).is_some_and(|access| access.permission == Permission::Operator)
}

/// A click on the block `name`. False for anything but a command block.
pub(super) fn open_editor(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    Kind::of(name).is_some() && show_editor(world, player, layer, pos, WindowType::CommandBlock)
}

/// An editor's click: the block's entity, then its screen; anyone else's: nothing, as BDS sent
/// nothing to a survival click. False where the block has no entity.
pub(super) fn show_editor(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, screen: WindowType) -> bool {
    if !may_edit(world, player) {
        return true;
    }
    let Some(shown) = level(world, layer).and_then(|level| level.block_entity(pos)).map(|entity| send::block_entity_data(pos, entity)) else { return false };
    let Some(window) = world.get_mut::<Inventory>(player).map(|mut inventory| inventory.next_window()) else { return false };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(shown);
        out.push(&sapling_data::block_screen(WindowID::from_raw(window.into()), screen, pos));
    }
    true
}

/// What a comparator reads of a command block: its `SuccessCount`.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    shape(world, layer, pos)?;
    Some(read(world, layer, pos)?.int("SuccessCount").clamp(0, 15) as u8)
}

/// A command block's signal changed: it keeps `powered`, and a rise starts an impulse or repeating
/// block that needs power (BDS `CommandBlock::_onRedstoneUpdate`).
pub(super) fn powered(world: &mut World, layer: Entity, pos: BlockPos, on: bool) {
    let (Some(shape), Some(mut fields)) = (shape(world, layer, pos), read(world, layer, pos)) else { return };
    if fields.flag("powered") == on {
        return;
    }
    fields.set_flag("powered", on);
    let starts = on && !fields.flag("auto") && shape.kind != Kind::Chain;
    fields.keep(world, layer, pos);
    if starts {
        run::start(world, layer, pos, &shape);
    }
}

/// Applies the editors' CommandBlockUpdates: the block becomes the kind and condition asked, its
/// entity takes the rest, both shown back. A command that does not parse keeps the syntax error
/// as its output, as BDS compiled it on the update (bds-cmdblocks.txt).
pub(crate) fn dispatch_updates(world: &mut World) {
    for (player, update) in world.resource_mut::<PacketQueues>().take::<CommandBlockUpdate>() {
        let CommandBlockUpdateContent::True(at) = &update.content else { continue };
        let pos = [at.position.x, at.position.y, at.position.z];
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0).filter(|_| may_edit(world, player)) else { continue };
        let (Some(old), Some(mut fields)) = (shape(world, layer, pos), read(world, layer, pos)) else { continue };
        let kind = match at.mode {
            UpdateMode::Impulse => Kind::Impulse,
            UpdateMode::Repeat => Kind::Repeating,
            UpdateMode::Chain => Kind::Chain,
            UpdateMode::Unknown(_) => continue,
        };
        let was_auto = fields.flag("auto");
        fields.put("Command", Value::String(update.command.as_str().into()));
        fields.put("CustomName", Value::String(update.name.as_str().into()));
        if update.filtered_name.is_empty() {
            fields.0.retain(|(name, _)| &**name != "FilteredCustomName");
        } else {
            fields.put("FilteredCustomName", Value::String(update.filtered_name.as_str().into()));
        }
        fields.set_flag("TrackOutput", update.should_track_output);
        fields.put("TickDelay", Value::Int(update.tick_delay.max(0)));
        fields.set_flag("ExecuteOnFirstTick", update.execute_on_first_tick);
        fields.set_flag("auto", !at.needs_redstone);
        fields.set_flag("conditionalMode", at.conditional);
        match run::syntax_error(world, &update.command) {
            Some((text, parameters)) => fields.set_output(&text, &parameters),
            None => fields.set_output(&update.last_output, &[]),
        }
        let facing = Face::ALL.iter().position(|face| *face == old.facing).unwrap_or(0);
        let block = Block::new(kind.block()).with(format!("conditional_bit={},facing_direction={facing}", u8::from(at.conditional)));
        let id = level(world, layer).and_then(|level| send::resolve(level.registry(), &block));
        if !id.is_some_and(|id| send::set(world, layer, pos, id)) {
            send::show(world, layer, pos);
        }
        let powered = fields.flag("powered");
        fields.show(world, layer, pos);
        // BDS `CommandBlockActor::setAutomatic`: made always active, or made repeating.
        let starts = (!was_auto || old.kind != Kind::Repeating && kind == Kind::Repeating) && !at.needs_redstone;
        if starts && kind != Kind::Chain && !powered {
            let shape = Shape { kind, facing: old.facing, conditional: at.conditional };
            run::start(world, layer, pos, &shape);
        }
    }
}
