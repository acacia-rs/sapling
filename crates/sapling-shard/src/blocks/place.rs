//! Placing: which block a click builds, where it may go, and what it costs the placer.

use acacia_proto::types::ItemV4;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, GameMode, PlaceEvent};
use sapling_world::BlockRegistry;

use super::facing::{self, Click};
use super::{Request, Site, doors, entities, falling, items, level, rails, send, signs, updates};
use crate::components::{InLayer, Position, Stage};
use crate::inventory::{self, Inventory, Place};

pub(super) const PLAYER_HALF_WIDTH: f32 = 0.3;
pub(super) const PLAYER_HEIGHT: f32 = 1.8;
/// Blocks a block placed into their cell takes the place of, by the end of their name: those
/// the Java game marks replaceable, in Bedrock's names.
const REPLACED: &[&str] = &[
    "short_grass", "tall_grass", ":fern", "large_fern", "deadbush", ":vine", ":fire", "soul_fire", "seagrass", "structure_void", "bubble_column", "crimson_roots",
    "warped_roots", "nether_sprouts", "glow_lichen", "resin_clump", "leaf_litter", "hanging_roots", "dry_grass", ":bush",
];
/// A snow layer gives way while it is a single one.
const THIN_SNOW: (&str, &str) = ("minecraft:snow_layer", "height=0");

/// The block a stack places on `face`: the one it names by `hash`, or the one its item builds
/// (items.rs).
fn block_of_stack(registry: &BlockRegistry, hash: Option<u32>, item: Option<&str>, face: Face) -> Option<u32> {
    let named = hash.and_then(|hash| registry.runtime_id_from_hash(hash)).filter(|id| registry.get(*id).is_some_and(|state| !state.is_air()));
    named.or_else(|| items::block_of(registry, item?, face))
}

/// The block the item a client says it holds places on `face`.
pub(super) fn claimed_block(registry: &BlockRegistry, item: &ItemV4, face: Face) -> Option<u32> {
    let is_held = item.network_id != 0 && item.count > 0;
    block_of_stack(registry, Some(item.block_runtime_id).filter(|hash| *hash != 0), sapling_data::item_name(item.network_id.into()), face).filter(|_| is_held)
}

/// The block the stack in a player's hand places on `face`, by the server's own inventory.
pub(super) fn held_block(world: &World, player: Entity, registry: &BlockRegistry, face: Face) -> Option<u32> {
    let stack = &world.get::<Inventory>(player)?.in_hand().as_ref()?.stack;
    block_of_stack(registry, stack.block, Some(&stack.name), face)
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
        let id = doors::hinged(level, facing::oriented(registry, id, click), pos, click.quarter());
        let id = items::attached(registry, id, level.block(against[0], against[1], against[2]), site.sneaking);
        let id = super::ground::settled(level, pos, id);
        let gives = |cell: BlockPos| level.in_height(cell[1]) && level.is_loaded(send::column(cell)) && gives_way(registry, level.block(cell[0], cell[1], cell[2]));
        let stands = |cell: BlockPos| registry.get(level.block(cell[0], cell[1] - 1, cell[2])).is_some_and(|state| !state.is_air());
        // A second cell beside the first is a bed's: both halves lie on something.
        let has_headroom = doors::second_cell(registry, id, pos).is_none_or(|(cell, _)| gives(cell) && (cell[1] != pos[1] || stands(cell) && stands(pos)));
        let (block, is_solid) = (send::describe(registry, id), registry.get(id).is_some_and(|state| state.is_solid()));
        // What hangs on a side or under a ceiling BDS put on no floor (bds-placing.txt,
        // bds-placing2.txt): a ladder, a tripwire hook and a vine on a side alone; hanging moss
        // and roots, a spore blossom and a hanging sign not on a floor.
        let named = |ends: &[&str]| registry.get(id).is_some_and(|state| ends.iter().any(|end| state.name.ends_with(end)));
        let on_side = named(&[":ladder", "tripwire_hook", ":vine"]) && matches!(click.face, Face::Up | Face::Down);
        let off_floor = named(&["pale_hanging_moss", "hanging_roots", "spore_blossom", "_hanging_sign"]) && click.face == Face::Up;
        let is_held_up = !updates::lacks_support(self.world, site.layer, pos, id) && !on_side && !off_floor;
        if !is_free || !has_headroom || !is_held_up || is_solid && is_occupied(self.world, site.layer, pos) {
            self.resync(&[against, pos]);
            return false;
        }
        let allowed = site.mode == GameMode::Creative || site.mode == GameMode::Survival && from_hand;
        let event = self.plugins.offer(self.world, PlaceEvent::new(self.player.clone(), pos, against, click.face, block, allowed));
        let placed = !event.is_cancelled() && self.put(site, pos, click, &event.block);
        if !placed {
            self.resync(&[against, pos]);
        }
        placed
    }

    /// Puts what a place event ended with into the world; false for a block the game lacks.
    fn put(&mut self, site: &Site, pos: BlockPos, click: &Click, block: &Block) -> bool {
        let Some(registry) = level(self.world, site.layer).map(|level| level.registry()) else { return false };
        let Some(whole) = send::resolve(registry, block).filter(|_| !block.is_air()) else { return false };
        let id = doors::lower_half(registry, whole);
        let sound = send::place_sound(registry, pos, id);
        let placed = send::set(self.world, site.layer, pos, falling::as_put(self.world, site.layer, pos, id));
        if placed {
            send::to_viewers(self.world, site.layer, pos, &[sound]);
            let second = doors::place_second_cell(self.world, site.layer, pos, whole);
            // A bed's colour and a banner's are the item's, which the block cannot say.
            let damage = self.world.get::<Inventory>(self.entity).and_then(|inventory| inventory.in_hand().as_ref()).map_or(0, |held| held.stack.damage);
            let placing = entities::Placing { damage, yaw: click.yaw, properties: &block.properties };
            for cell in [Some(pos), second].into_iter().flatten() {
                if let Some(entity) = entities::placed(&block.name, cell, &placing) {
                    send::keep_entity(self.world, site.layer, cell, &entity);
                }
            }
            signs::open_editor(self.world, self.entity, site.layer, pos, &block.name);
            rails::join_placed(self.world, site.layer, pos);
            let tag = self.world.get::<Inventory>(self.entity).and_then(|inventory| inventory.in_hand().as_ref()).and_then(|held| held.stack.tag.clone());
            crate::containers::furnish(self.world, site.layer, pos, &block.name, tag.as_ref(), click.face);
            super::pistons::furnish(self.world, site.layer, pos);
            updates::settle(self.world, site.layer, pos);
        }
        placed
    }
}
