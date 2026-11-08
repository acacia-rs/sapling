//! A player's breaks: the hit that starts one, giving it up, and the finished break.

use sapling_api::{BlockPos, BreakEvent, Face, GameMode};

use super::{Request, Site, site};
use crate::blocks::{self, after_break, doors, level, loot, mining, send, updates};
use crate::components::InLayer;
use crate::plugins;

impl Request<'_> {
    /// A hit that starts a break: viewers see fragments fly off the face, as from BDS. A frame's
    /// item or a lectern's book knocked out still starts the break (research/captures/bds-t3uses.txt).
    pub(super) fn punch(&mut self, pos: BlockPos, face: Option<Face>) {
        let (Some(site), Some(face)) = (site(self.world, self.entity, pos), face) else { return };
        if site.mode != GameMode::Spectator && blocks::fire_punch::put_out(self.world, self.entity, site.layer, pos, face) {
            return;
        }
        let knocked = site.mode != GameMode::Spectator && (blocks::holders::hit(self.world, site.layer, pos, site.mode != GameMode::Creative) || blocks::lectern::hit(self.world, site.layer, pos));
        if knocked && site.mode != GameMode::Survival {
            return;
        }
        let Some((registry, id)) = self.block(&site, pos) else { return };
        let Some(hardness) = blocks::custom::breakable(registry, id).filter(|_| site.mode != GameMode::Spectator) else { return };
        let (packet, block) = (send::punch_particles(registry, pos, face, id), send::describe(registry, id));
        let mines = mining::mines(self.world, self.entity, site.mode, &block.name);
        if !crate::hand_events::may_start_break(self.world, self.entity, pos, block) {
            return;
        }
        match mines {
            true => mining::start(self.world, self.entity, site.layer, pos, &hardness, packet),
            false => send::to_viewers(self.world, site.layer, pos, &[packet]),
        }
    }

    pub(super) fn abort_break(&mut self) {
        if let Some(layer) = self.world.get::<InLayer>(self.entity).map(|layer| layer.0) {
            mining::abort(self.world, self.entity, layer);
        }
    }

    /// The client finished breaking a block. In survival the event starts refused unless the
    /// break was started and took its time (mining.rs); a handler may still allow it.
    pub(super) fn break_block(&mut self, pos: BlockPos) {
        let above = Face::Up.neighbour(pos);
        let took_its_time = mining::finish(self.world, self.entity, pos);
        let Some(site) = site(self.world, self.entity, pos) else { return self.resync(&[pos, above]) };
        let Some((registry, id)) = self.block(&site, pos) else { return };
        let Some(mining) = blocks::custom::breakable(registry, id) else { return self.resync(&[pos, above]) };
        let block = send::describe(registry, id);
        let name = block.name.clone();
        let by = mining::breaker(self.world, self.entity, &mining);
        let (mut drops, mut tags) = (Vec::new(), loot::Tags::default());
        let mines = mining::mines(self.world, self.entity, site.mode, &name);
        if mines {
            drops = loot::thrown(self.world, &block.name, &block.properties, &by);
            tags = loot::add_entity_loot(self.world, site.layer, pos, &mut drops);
        } else if site.mode == GameMode::Creative && name == blocks::entities::POT {
            // Its loot and stack, not the pot (research/captures/bds-loot-probe2.txt).
            tags = loot::add_entity_loot(self.world, site.layer, pos, &mut drops);
        }
        let allowed = site.mode == GameMode::Creative || mines && took_its_time;
        let event = plugins::offer(self.world,BreakEvent::new(self.player.clone(), pos, block, drops, allowed));
        let bees = crate::creatures::bees_in(self.world, site.layer, pos);
        let goes = !event.is_cancelled();
        if goes {
            blocks::tripwire::breaking(self.world, site.layer, pos, by.shears);
        }
        match goes && self.remove(&site, pos) {
            true => {
                if let Some(kept) = crate::creatures::hive_broken(self.world, site.layer, pos, &name, bees, mines && by.silk_touch) {
                    tags = loot::Tags::one(&name, Some(kept));
                }
                loot::spill_tagged(self.world, site.layer, pos, &event.drops, &mut tags);
                if mines {
                    after_break::leave(self.world, site.layer, pos, &name, &by);
                }
                if mines && mining.hardness > 0.0 {
                    crate::inventory::wear_tool_in_hand(self.world, self.entity);
                }
                if by.harvests {
                    crate::vitals::tire(self.world, self.entity, crate::vitals::BREAK_TIRES);
                }
            }
            false => self.resync(&[pos, above]),
        }
    }

    /// Takes the block at `pos` out of the world: what is left is the water it stood in, or air.
    fn remove(&mut self, site: &Site, [x, y, z]: BlockPos) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let (id, air) = (level.block(x, y, z), level.registry().air_id());
        let particles = send::destroy_particles(level.registry(), [x, y, z], id);
        doors::remove_other_half(self.world, site.layer, [x, y, z]);
        crate::containers::spill(self.world, site.layer, [x, y, z], site.mode == GameMode::Creative);
        blocks::bookshelves::spill(self.world, site.layer, [x, y, z], site.mode == GameMode::Creative);
        // BDS sends the fragments before the block; the cell is loaded and not air, so it changes.
        send::to_viewers(self.world, site.layer, [x, y, z], &[particles]);
        if !send::set(self.world, site.layer, [x, y, z], air) {
            return false;
        }
        updates::settle(self.world, site.layer, [x, y, z]);
        blocks::vibrate_at(self.world, site.layer, [x, y, z], blocks::GameEvent::BlockDestroy, true);
        true
    }
}
