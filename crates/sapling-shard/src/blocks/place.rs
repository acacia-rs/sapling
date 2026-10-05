//! Placing: which block a click builds, where it may go, and what it costs the placer.

use acacia_proto::types::ItemV4;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, GameMode, PlaceEvent};
use sapling_world::BlockRegistry;

use super::facing::{self, Click};
use super::{Request, Site, doors, level, send, updates};
use crate::components::{InLayer, Position, Stage};
use crate::inventory::{self, Inventory, Place};

const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;
/// Blocks a block placed into their cell takes the place of, by the end of their name: those
/// the Java game marks replaceable, in Bedrock's names.
const REPLACED: &[&str] = &[
    "short_grass", "tall_grass", ":fern", "large_fern", "deadbush", ":vine", ":fire", "soul_fire", "seagrass", "structure_void", "bubble_column", "crimson_roots",
    "warped_roots", "nether_sprouts", "glow_lichen", "resin_clump", "leaf_litter", "hanging_roots", "dry_grass", ":bush",
];
/// A snow layer gives way while it is a single one.
const THIN_SNOW: (&str, &str) = ("minecraft:snow_layer", "height=0");

/// The block the item a client says it holds places.
pub(super) fn claimed_block(registry: &BlockRegistry, item: &ItemV4) -> Option<u32> {
    let id = registry.runtime_id_from_hash(item.block_runtime_id).filter(|_| item.network_id != 0 && item.count > 0)?;
    registry.get(id).is_some_and(|state| !state.is_air()).then_some(id)
}

/// The block the stack in a player's hand places, by the server's own inventory.
pub(super) fn held_block(world: &World, player: Entity, registry: &BlockRegistry) -> Option<u32> {
    let hash = world.get::<Inventory>(player)?.in_hand().as_ref()?.stack.block?;
    registry.runtime_id_from_hash(hash).filter(|id| registry.get(*id).is_some_and(|state| !state.is_air()))
}

/// Whether a block placed into the cell of block `id` takes its place.
pub(super) fn gives_way(registry: &BlockRegistry, id: u32) -> bool {
    let listed = |name: &str| REPLACED.iter().any(|end| name.ends_with(end)) || name.contains(":light_block");
    let is_thin_snow = |name: &str, properties: &str| name == THIN_SNOW.0 && properties.split(',').any(|pair| pair == THIN_SNOW.1);
    registry.get(id).is_some_and(|state| state.is_air() || state.is_liquid() || listed(state.name) || is_thin_snow(state.name, state.properties))
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

impl Request<'_> {
    /// Builds `id`, turned as `click` asks, against the block at `against`, or into its cell
    /// when that block gives way. `from_hand`: the block came out of the server's copy of the
    /// hand, which a survival place then takes one from; any other place needs creative mode or
    /// a handler's word. Outside creative mode the client is shown its windows afterwards, as
    /// from BDS: it counted one off its stack whether the place happened or not.
    pub(super) fn place(&mut self, site: &Site, against: BlockPos, click: &Click, id: u32, from_hand: bool) {
        let placed = self.try_place(site, against, click, id, from_hand);
        if site.mode != GameMode::Creative {
            inventory::change(self.world, self.entity, |inventory| {
                if placed && from_hand {
                    let _ = inventory.take(Place::Main(inventory.selected.into()), 1);
                }
            });
        }
    }

    fn try_place(&mut self, site: &Site, against: BlockPos, click: &Click, id: u32, from_hand: bool) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let registry = level.registry();
        let into_clicked = gives_way(registry, level.block(against[0], against[1], against[2]));
        let pos = if into_clicked { against } else { click.face.neighbour(against) };
        let is_free = level.in_height(pos[1]) && level.is_loaded(send::column(pos)) && gives_way(registry, level.block(pos[0], pos[1], pos[2]));
        let id = facing::oriented(registry, id, click);
        let above = Face::Up.neighbour(pos);
        let has_headroom = !doors::is_tall(registry, id) || level.in_height(above[1]) && gives_way(registry, level.block(above[0], above[1], above[2]));
        let (block, is_solid) = (send::describe(registry, id), registry.get(id).is_some_and(|state| state.is_solid()));
        let is_held_up = !updates::lacks_support(self.world, site.layer, pos, id);
        if !is_free || !has_headroom || !is_held_up || is_solid && is_occupied(self.world, site.layer, pos) {
            self.resync(&[against, pos]);
            return false;
        }
        let allowed = site.mode == GameMode::Creative || site.mode == GameMode::Survival && from_hand;
        let (event, player) = (PlaceEvent::new(pos, against, click.face, block, allowed), self.player);
        let event = self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_place(ctx, player, event));
        let placed = !event.is_cancelled() && self.put(site, pos, &event.block);
        if !placed {
            self.resync(&[against, pos]);
        }
        placed
    }

    /// Puts what a place event ended with into the world; false for a block the game lacks.
    fn put(&mut self, site: &Site, pos: BlockPos, block: &Block) -> bool {
        let Some(registry) = level(self.world, site.layer).map(|level| level.registry()) else { return false };
        let Some(id) = send::resolve(registry, block).filter(|_| !block.is_air()) else { return false };
        let sound = send::place_sound(registry, pos, id);
        let placed = send::set(self.world, site.layer, pos, id);
        if placed {
            send::to_viewers(self.world, site.layer, pos, &[sound]);
            doors::place_upper_half(self.world, site.layer, pos);
            crate::containers::furnish(self.world, site.layer, pos, &block.name);
            updates::settle(self.world, site.layer, pos);
        }
        placed
    }
}
