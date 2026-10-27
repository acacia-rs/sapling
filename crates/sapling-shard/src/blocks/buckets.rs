//! Buckets, by BDS `BucketItem::_useOn` (docs/research/buckets-lava.md): an empty one takes the
//! liquid source that was clicked, a full one pours its liquid into a block that keeps water or
//! the cell on the clicked face. Taking is offered to handlers as a break and pouring as a place,
//! so what protects blocks protects them from buckets too. Powder snow is taken and put as a
//! block, heard as `bucket.fill.powder_snow` and `bucket.empty.powder_snow`; a fish bucket pours
//! water and lets its mob out (creatures/scooping.rs; research/captures/bds-buckets.txt).
// TODO: no milk or cauldrons; a handler's change of the event's block is not followed; the
// nether's evaporation particles are not sent.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use sapling_api::{Block, BlockPos, BreakEvent, Face, GameMode, PlaceEvent};
use sapling_data::LiquidReaction;
use sapling_world::ItemStack;

use super::liquid::{self, Cell, Liquid};
use super::loot::{self, Breaker};
use super::{Request, Site, level, send, waterlog};
use crate::components::Position;
use crate::inventory::{self, Inventory};

const BUCKET: &str = "minecraft:bucket";
const WATER_BUCKET: &str = "minecraft:water_bucket";
const LAVA_BUCKET: &str = "minecraft:lava_bucket";
const SNOW: &str = "minecraft:snow_layer";
const PORTAL: &str = "minecraft:portal";
const POWDER_SNOW: &str = "minecraft:powder_snow";
const SNOW_BUCKET: &str = "minecraft:powder_snow_bucket";

/// What a bucket item holds: `Some(None)` for an empty one, `None` for an item that is no bucket.
fn contents(item: &str) -> Option<Option<Liquid>> {
    match item {
        BUCKET => Some(None),
        WATER_BUCKET => Some(Some(Liquid::Water)),
        LAVA_BUCKET => Some(Some(Liquid::Lava)),
        _ => None,
    }
}

fn source(liquid: Liquid) -> Block {
    Block { name: liquid.name().to_owned(), properties: "liquid_depth=0".to_owned() }
}

impl Request<'_> {
    /// A click with `held` in the hand on `face` of the block at `pos`; false when `held` is no
    /// bucket. As BDS: a refusal resends the cells the client may have predicted and not the
    /// inventory; a use resends the inventory, in creative too.
    pub(super) fn use_bucket(&mut self, site: &Site, pos: BlockPos, face: Face, held: &str) -> bool {
        let mob = crate::creatures::bucketed_kind(held).and_then(|_| self.world.get::<Inventory>(self.entity)?.in_hand().as_ref().map(|held| held.stack.clone()));
        let worked = match (held, contents(held)) {
            (SNOW_BUCKET, _) => self.pour_snow(site, pos, face),
            (_, Some(None)) => self.fill_snow(site, pos).unwrap_or_else(|| self.fill(site, pos)),
            (_, Some(Some(liquid))) => self.pour(site, pos, face, liquid, None),
            (_, None) if mob.is_some() => self.pour(site, pos, face, Liquid::Water, mob.as_ref()),
            (_, None) => return false,
        };
        let target = face.neighbour(pos);
        match worked {
            true => drop(inventory::change(self.world, self.entity, |_| {})),
            false => self.resync(&[pos, Face::Up.neighbour(pos), target, Face::Down.neighbour(target)]),
        }
        true
    }

    fn allowed(site: &Site) -> bool {
        matches!(site.mode, GameMode::Survival | GameMode::Creative)
    }

    /// Takes the liquid source at `pos` into an empty bucket.
    fn fill(&mut self, site: &Site, pos: BlockPos) -> bool {
        let Cell::Liquid(liquid, 0) = liquid::cell(self.world, site.layer, pos) else { return false };
        let event = BreakEvent::new(self.player.clone(), pos, source(liquid), Vec::new(), Self::allowed(site));
        if crate::plugins::offer(self.world, event).is_cancelled() {
            return false;
        }
        // Water walled in by water sources and solid blocks stays: no air pocket in the sea.
        let is_walled = |at: BlockPos| liquid::cell(self.world, site.layer, at) == Cell::Liquid(Liquid::Water, 0) || self.is_solid(site, at);
        if liquid == Liquid::Lava || !Face::ALL.iter().all(|face| is_walled(face.neighbour(pos))) {
            liquid::take(self.world, site.layer, pos);
        }
        self.hear(site, if liquid == Liquid::Water { "bucket.fill.water" } else { "bucket.fill.lava" });
        if site.mode != GameMode::Creative {
            self.swap_held(if liquid == Liquid::Water { WATER_BUCKET } else { LAVA_BUCKET });
        }
        true
    }

    /// Pours a full bucket's `liquid` out: water into the clicked block when that keeps water,
    /// otherwise into the cell on `face`, or the snow layer under that cell.
    fn pour(&mut self, site: &Site, pos: BlockPos, face: Face, liquid: Liquid, mob: Option<&ItemStack>) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let registry = level.registry();
        let at = |cell: BlockPos| (level.block(cell[0], cell[1], cell[2]), level.liquid(cell[0], cell[1], cell[2]));
        let is_water = liquid == Liquid::Water;
        let mut target = if is_water && waterlog::holds_water(registry, at(pos).0) { pos } else { face.neighbour(pos) };
        let below = Face::Down.neighbour(target);
        let snow_under = level.in_height(below[1]) && registry.get(at(below).0).is_some_and(|state| state.name == SNOW);
        if snow_under {
            target = below;
        }
        // The snow layer is cleared first: what is checked is the cell without it.
        let (old, held) = if snow_under { (registry.air_id(), registry.air_id()) } else { at(target) };
        let (Some(old_state), Some(held_state)) = (registry.get(old), registry.get(held)) else { return false };
        let keeps = is_water && waterlog::holds_water(registry, old);
        if !held_state.is_liquid() && (old_state.is_solid() && !keeps || old_state.name == PORTAL || !held_state.is_air()) {
            return false;
        }
        let waterlogs = keeps && !old_state.is_air();
        let (washed, snow) = ((!old_state.is_air() && !waterlogs).then_some((old_state.name, old_state.properties)), at(target).0);
        let pops = waterlog::reaction(registry, old) == LiquidReaction::Popped;
        if washed.is_some() && old_state.is_solid() || super::site(self.world, self.entity, target).is_none() {
            return false;
        }
        let event = PlaceEvent::new(self.player.clone(), target, pos, face, source(liquid), Self::allowed(site));
        if crate::plugins::offer(self.world, event).is_cancelled() {
            return false;
        }
        if snow_under {
            // At the cell's corner, where BDS puts them for a cleared snow layer.
            let hash = super::level(self.world, site.layer).map_or(0, |level| level.registry().network_hash(snow));
            let [x, y, z] = target.map(|c| c as f32);
            let particles = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroy, position: Vec3f { x, y, z }, data: hash as i32 });
            send::to_viewers(self.world, site.layer, target, &[particles]);
        }
        if is_water && liquid::is_hot(self.world, site.layer) {
            let [x, y, z] = target.map(|c| c as f32 + 0.5);
            send::to_viewers(self.world, site.layer, target, &[send::noise("fizz", Vec3f { x, y, z }, -1)]);
        } else {
            // What a Popped block leaves, as if broken; lava burns everything.
            if let Some((name, properties)) = washed.filter(|_| is_water && pops) {
                let left = loot::thrown(self.world, name, properties, &Breaker::UNAIDED);
                loot::spill(self.world, site.layer, target, &left);
            }
            if !liquid::pour(self.world, site.layer, target, liquid, waterlogs) {
                return false;
            }
            let sound = match (is_water, mob) {
                (true, Some(_)) => "bucket.empty.fish",
                (true, None) => "bucket.empty.water",
                (false, _) => "bucket.empty.lava",
            };
            self.hear(site, sound);
        }
        if let Some(bucket) = mob {
            crate::creatures::release_from_bucket(self.world, site.layer, bucket, target);
        }
        if site.mode != GameMode::Creative {
            self.swap_held(BUCKET);
        }
        true
    }

    /// Takes the powder snow at `pos` into an empty bucket; `None` when it is none.
    fn fill_snow(&mut self, site: &Site, pos: BlockPos) -> Option<bool> {
        let snow = Block { name: POWDER_SNOW.to_owned(), properties: String::new() };
        (super::send::block_at(self.world, site.layer, pos)?.name == POWDER_SNOW).then_some(())?;
        let event = BreakEvent::new(self.player.clone(), pos, snow, Vec::new(), Self::allowed(site));
        if crate::plugins::offer(self.world, event).is_cancelled() || !super::update_block(self.world, site.layer, pos, &Block::air()) {
            return Some(false);
        }
        self.hear(site, "bucket.fill.powder_snow");
        if site.mode != GameMode::Creative {
            self.swap_held(SNOW_BUCKET);
        }
        Some(true)
    }

    /// Puts powder snow into the cell on `face`, or the clicked cell when that gives way, as a
    /// block is placed.
    fn pour_snow(&mut self, site: &Site, pos: BlockPos, face: Face) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let gives_way = |[x, y, z]: BlockPos| level.in_height(y) && super::place::gives_way(level.registry(), level.block(x, y, z));
        let target = if gives_way(pos) { pos } else { face.neighbour(pos) };
        if !gives_way(target) || super::site(self.world, self.entity, target).is_none() {
            return false;
        }
        let snow = Block { name: POWDER_SNOW.to_owned(), properties: String::new() };
        let event = PlaceEvent::new(self.player.clone(), target, pos, face, snow.clone(), Self::allowed(site));
        if crate::plugins::offer(self.world, event).is_cancelled() || !super::update_block(self.world, site.layer, target, &snow) {
            return false;
        }
        self.hear(site, "bucket.empty.powder_snow");
        if site.mode != GameMode::Creative {
            self.swap_held(BUCKET);
        }
        true
    }

    fn is_solid(&self, site: &Site, [x, y, z]: BlockPos) -> bool {
        level(self.world, site.layer).is_some_and(|level| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid()))
    }

    /// Lets those near the player hear `name` at its feet, where BDS plays a bucket's sounds.
    fn hear(&mut self, site: &Site, name: &str) {
        let Some(feet) = self.world.get::<Position>(self.entity).map(|position| position.feet) else { return };
        let [x, y, z] = feet;
        send::to_viewers(self.world, site.layer, feet.map(|c| c.floor() as i32), &[send::noise(name, Vec3f { x, y, z }, -1)]);
    }

    /// See [`inventory::swap_held`].
    pub(super) fn swap_held(&mut self, into: &str) {
        inventory::swap_held(self.world, self.entity, into);
    }
}
