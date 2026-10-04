//! Breaking, placing and clicking blocks: what a client asks for is checked, offered to the
//! handlers and then either done for everyone who sees the place or undone on the client that
//! predicted it (docs/DESIGN.md "Step 6 as built").

mod send;

use std::collections::HashSet;

use acacia_proto::packets::{InventoryTransaction, PlayerAuthInput, PlayerAuthInputBlockActionItem};
use acacia_proto::types::{Action, ItemV4, TransactionTransactionData, TransactionUseItem, TransactionUseItemActionType};
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, BreakEvent, Face, GameMode, InteractEvent, PlaceEvent, Player};
use sapling_data::EYE_HEIGHT;
use sapling_world::BlockRegistry;

use crate::components::{InLayer, Layer, Mode, PacketQueues, PerTick, Position, Stage, in_world_identity};
use crate::movement::Motion;
use crate::plugins::Plugins;

/// Block actions looked at per player per tick; a client holding a break sends two or three.
const MAX_ACTIONS_PER_TICK: usize = 16;
/// Block clicks looked at per player per tick; more are dropped without an answer.
const MAX_CLICKS_PER_TICK: usize = 4;
/// Blocks from the eye to the middle of a block: BDS took a click at 6.4 and 12.2 (creative) and
/// refused 7.3 and 13.2.
const REACH: f32 = 7.0;
const CREATIVE_REACH: f32 = 13.0;
const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;
/// Blocks a click uses rather than builds on, by the end of their name. TODO: a per-block flag.
const USED: &[&str] = &[
    "door", "button", "lever", "fence_gate", "chest", "barrel", "shulker_box", "furnace", "smoker", "crafting_table", "anvil", "enchanting_table", "hopper",
    "dropper", "dispenser", "_sign", "noteblock", "repeater", "comparator", "brewing_stand", "beacon", "loom", "grindstone", "stonecutter_block", ":bed",
    "cartography_table", "smithing_table",
];

enum Input {
    Actions(Vec<PlayerAuthInputBlockActionItem>),
    Click(TransactionUseItem),
}

/// What this tick's inputs asked of blocks, in arrival order, for [`dispatch`].
#[derive(Resource, Default)]
pub(crate) struct BlockInputs(Vec<(Entity, Input)>);

impl BlockInputs {
    /// Takes the block actions and the item use out of a player's input.
    pub fn take_from(&mut self, from: Entity, input: &mut PlayerAuthInput) {
        if let Some(mut actions) = input.block_action.take() {
            actions.truncate(MAX_ACTIONS_PER_TICK);
            self.0.push((from, Input::Actions(actions)));
        }
        if let Some(transaction) = input.transaction.take() {
            self.0.push((from, Input::Click(transaction.data)));
        }
    }
}

/// Where a request happens, once it passed the checks every request must.
struct Site {
    layer: Entity,
    mode: GameMode,
    sneaking: bool,
}

fn level(world: &World, layer: Entity) -> Option<&sapling_world::World> {
    world.get::<Layer>(layer).map(|layer| &layer.world)
}

/// The site of a request at `pos`, unless it is outside the world's height, in a column that is
/// not loaded or further from the player's eyes than its game mode reaches.
fn site(world: &World, player: Entity, pos: BlockPos) -> Option<Site> {
    let (layer, mode) = (world.get::<InLayer>(player)?.0, world.get::<Mode>(player)?.0);
    let (position, level) = (world.get::<Position>(player)?, level(world, layer)?);
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let distance: f32 = (0..3).map(|axis| (pos[axis] as f32 + 0.5 - eye[axis]).powi(2)).sum();
    let reach = if mode == GameMode::Creative { CREATIVE_REACH } else { REACH };
    let known = level.in_height(pos[1]) && level.is_loaded(send::column(pos));
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    (known && distance <= reach * reach).then_some(Site { layer, mode, sneaking })
}

fn is_used(name: &str) -> bool {
    USED.iter().any(|end| name.ends_with(end))
}

/// The block a held item places. The item is the client's word: there are no inventories yet.
fn held_block(registry: &BlockRegistry, item: &ItemV4) -> Option<u32> {
    let id = registry.runtime_id_from_hash(item.block_runtime_id).filter(|_| item.network_id != 0 && item.count > 0)?;
    registry.get(id).is_some_and(|state| !state.is_air()).then_some(id)
}

/// Whether a player of `layer` stands in the cell at `pos`.
fn is_occupied(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let [x, y, z] = pos.map(|c| c as f32);
    let overlaps = |feet: [f32; 3]| {
        let across = |at: f32, min: f32| at + PLAYER_HALF_WIDTH > min && at - PLAYER_HALF_WIDTH < min + 1.0;
        across(feet[0], x) && across(feet[2], z) && feet[1] + PLAYER_HEIGHT > y && feet[1] < y + 1.0
    };
    let mut players = world.query::<(&InLayer, &Stage, &Position)>();
    players.iter(world).any(|(in_layer, stage, position)| in_layer.0 == layer && *stage == Stage::InWorld && overlaps(position.feet))
}

/// Runs this tick's block requests through the checks and the handlers.
pub(crate) fn dispatch(world: &mut World, plugins: &mut Plugins) {
    let mut inputs = std::mem::take(&mut world.resource_mut::<BlockInputs>().0);
    let transactions = world.resource_mut::<PacketQueues>().take::<InventoryTransaction>();
    inputs.extend(transactions.into_iter().filter_map(|(from, packet)| match packet.transaction.transaction_data {
        TransactionTransactionData::ItemUse(click) => Some((from, Input::Click(click))),
        _ => None,
    }));
    let (mut actions, mut clicks) = (PerTick::new(MAX_ACTIONS_PER_TICK), PerTick::new(MAX_CLICKS_PER_TICK));
    // A client names a finished break twice: as a block action and as an item use.
    let mut broken = HashSet::new();
    for (entity, input) in inputs {
        let Some(player) = in_world_identity(world, entity) else { continue };
        let mut request = Request { world: &mut *world, plugins: &mut *plugins, entity, player: &player };
        match input {
            Input::Actions(items) => {
                for item in items.iter().filter(|_| actions.allow(entity)) {
                    let pos = [item.position.x, item.position.y, item.position.z];
                    match item.action {
                        Action::StartBreak => request.punch(pos, u32::try_from(item.face).ok().and_then(Face::from_index)),
                        Action::PredictBreak if broken.insert((entity, pos)) => request.break_block(pos),
                        _ => {}
                    }
                }
            }
            Input::Click(click) if clicks.allow(entity) => {
                let pos = [click.block_position.x, click.block_position.y, click.block_position.z];
                match click.action_type {
                    TransactionUseItemActionType::ClickBlock => request.click(pos, Face::from_index(u32::from(click.face)), &click.held_item),
                    TransactionUseItemActionType::BreakBlock if broken.insert((entity, pos)) => request.break_block(pos),
                    _ => {}
                }
            }
            Input::Click(_) => {}
        }
    }
}

/// One player's request while it is being answered.
struct Request<'a> {
    world: &'a mut World,
    plugins: &'a mut Plugins,
    entity: Entity,
    player: &'a Player,
}

impl Request<'_> {
    fn resync(&mut self, positions: &[BlockPos]) {
        send::resync(self.world, self.entity, positions);
    }

    /// The registry and the layer-0 block at `pos`.
    fn block(&self, site: &Site, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
        level(self.world, site.layer).map(|level| (level.registry(), level.block(x, y, z)))
    }

    /// A hit that starts a break: viewers see fragments fly off the face, as from BDS.
    fn punch(&mut self, pos: BlockPos, face: Option<Face>) {
        let (Some(site), Some(face)) = (site(self.world, self.entity, pos), face) else { return };
        let Some((registry, id)) = self.block(&site, pos) else { return };
        if site.mode != GameMode::Spectator && registry.get(id).is_some_and(|state| !state.is_air() && !state.is_liquid()) {
            let packet = send::punch_particles(registry, pos, face, id);
            send::to_viewers(self.world, site.layer, pos, &[packet]);
        }
    }

    /// The client finished breaking a block. TODO: no check that it took the time the block needs.
    fn break_block(&mut self, pos: BlockPos) {
        let above = Face::Up.neighbour(pos);
        let Some(site) = site(self.world, self.entity, pos) else { return self.resync(&[pos, above]) };
        let Some((registry, id)) = self.block(&site, pos) else { return };
        if registry.get(id).is_none_or(|state| state.is_air() || state.is_liquid()) {
            return self.resync(&[pos, above]);
        }
        let allowed = matches!(site.mode, GameMode::Survival | GameMode::Creative);
        let (event, player) = (BreakEvent::new(pos, send::describe(registry, id), allowed), self.player);
        let event = self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_break(ctx, player, event));
        if event.is_cancelled() || !self.remove(&site, pos) {
            self.resync(&[pos, above]);
        }
    }

    /// Takes the block at `pos` out of the world: what is left is the water it stood in, or air.
    fn remove(&mut self, site: &Site, [x, y, z]: BlockPos) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let (id, left) = (level.block(x, y, z), level.liquid(x, y, z));
        let particles = send::destroy_particles(level.registry(), [x, y, z], id);
        if !send::set(self.world, site.layer, [x, y, z], left) {
            return false;
        }
        send::clear_liquid(self.world, site.layer, [x, y, z]);
        send::to_viewers(self.world, site.layer, [x, y, z], &[particles]);
        true
    }

    /// A click on `face` of the block at `pos`: a placement if the client holds a block and the
    /// clicked block is not one a click uses, an interaction otherwise.
    fn click(&mut self, pos: BlockPos, face: Option<Face>, held: &ItemV4) {
        let Some(face) = face else { return self.resync(&[pos]) };
        let target = face.neighbour(pos);
        let Some(site) = site(self.world, self.entity, pos) else { return self.resync(&[pos, target]) };
        let Some((registry, clicked)) = self.block(&site, pos) else { return };
        let Some(state) = registry.get(clicked).filter(|state| !state.is_air()) else { return self.resync(&[pos, target]) };
        match held_block(registry, held).filter(|_| site.sneaking || !is_used(state.name)) {
            Some(id) => self.place(&site, pos, face, id),
            None => self.interact(&site, pos, face, send::describe(registry, clicked)),
        }
    }

    fn place(&mut self, site: &Site, against: BlockPos, face: Face, id: u32) {
        let pos = face.neighbour(against);
        let Some(level) = level(self.world, site.layer) else { return };
        let registry = level.registry();
        let is_free = level.in_height(pos[1])
            && level.is_loaded(send::column(pos))
            && registry.get(level.block(pos[0], pos[1], pos[2])).is_some_and(|state| state.is_air() || state.is_liquid());
        let (block, is_solid) = (send::describe(registry, id), registry.get(id).is_some_and(|state| state.is_solid()));
        if !is_free || is_solid && is_occupied(self.world, site.layer, pos) {
            return self.resync(&[against, pos]);
        }
        let (event, player) = (PlaceEvent::new(pos, against, face, block, site.mode == GameMode::Creative), self.player);
        let event = self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_place(ctx, player, event));
        if event.is_cancelled() || !self.put(site, pos, &event.block) {
            self.resync(&[against, pos]);
        }
    }

    /// Puts what a place event ended with into the world; false for a block the game lacks.
    fn put(&mut self, site: &Site, pos: BlockPos, block: &Block) -> bool {
        let Some(registry) = level(self.world, site.layer).map(|level| level.registry()) else { return false };
        let Some(id) = send::resolve(registry, block).filter(|_| !block.is_air()) else { return false };
        let sound = send::place_sound(registry, pos, id);
        let placed = send::set(self.world, site.layer, pos, id);
        if placed {
            send::to_viewers(self.world, site.layer, pos, &[sound]);
        }
        placed
    }

    /// Nothing is implemented for any block yet, so allowed or not the click changes nothing and
    /// the client is shown the block and its neighbours as they are (a door is two blocks).
    fn interact(&mut self, site: &Site, pos: BlockPos, face: Face, block: Block) {
        let (event, player) = (InteractEvent::new(pos, face, block, site.mode != GameMode::Spectator), self.player);
        self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_interact(ctx, player, event));
        self.resync(&[pos, face.neighbour(pos), Face::Up.neighbour(pos), Face::Down.neighbour(pos)]);
    }
}

/// The block at `pos` of a layer's world; `None` where it is not loaded or does not reach.
pub(crate) fn block_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Block> {
    let level = level(world, layer)?;
    let known = level.in_height(pos[1]) && level.is_loaded(send::column(pos));
    known.then(|| send::describe(level.registry(), level.block(pos[0], pos[1], pos[2])))
}

/// Sets a block for a handler: no event, and everyone who sees the place is shown it.
pub(crate) fn put_block(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> bool {
    let Some(id) = level(world, layer).and_then(|level| send::resolve(level.registry(), block)) else { return false };
    send::set(world, layer, pos, id)
}
