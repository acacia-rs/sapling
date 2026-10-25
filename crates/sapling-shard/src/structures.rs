//! Structure templates by name for `/structure` and plugins: those saved to memory last until the
//! server stops, those saved to disk go with the world's level (`Level::structures`), and those of
//! the folder's `structures` directory are read from it. Each realm keeps its own, on its overworld.

mod capture;
mod place;
mod turn;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, StructureBackend, StructureError, StructureLoad};
use sapling_world::{StructureTemplate, structure_name};

use crate::components::Layer;
use crate::handlers::Ctx;
use crate::{dimensions, worlds};

/// The structures a realm keeps in memory, and the names a load failed to find: BDS answered a
/// delete of such a name as though it had one, once (research/captures/bds-structures.txt).
#[derive(Component, Default)]
struct InMemory {
    templates: BTreeMap<String, Vec<u8>>,
    missed: BTreeSet<String>,
}

fn memory(world: &mut World, home: Entity) -> Mut<'_, InMemory> {
    if world.get::<InMemory>(home).is_none() {
        world.entity_mut(home).insert(InMemory::default());
    }
    world.get_mut::<InMemory>(home).expect("inserted above")
}

/// The template saved under `name` (namespaced), memory first, then disk, then the folder.
fn template_bytes(world: &World, home: Entity, name: &str) -> Option<Vec<u8>> {
    let in_memory = world.get::<InMemory>(home).and_then(|memory| memory.templates.get(name));
    let level = world.get::<Layer>(home).map(|layer| layer.world.level());
    in_memory.or_else(|| level.and_then(|level| level.structures.get(name).or_else(|| level.folder_structures.get(name)))).cloned()
}

fn keep(world: &mut World, home: Entity, name: String, template: Vec<u8>, to_disk: bool) {
    let mut memory = memory(world, home);
    memory.missed.remove(&name);
    if to_disk {
        memory.templates.remove(&name);
        if let Some(mut layer) = world.get_mut::<Layer>(home) {
            Arc::make_mut(&mut layer.world.level_mut().structures).insert(name, template);
        }
    } else {
        memory.templates.insert(name.clone(), template);
        if let Some(mut layer) = world.get_mut::<Layer>(home) {
            let structures = &mut layer.world.level_mut().structures;
            if structures.contains_key(&name) {
                Arc::make_mut(structures).remove(&name);
            }
        }
    }
}

impl Ctx<'_> {
    fn structure_home(&self) -> Entity {
        dimensions::home(self.world, worlds::current(self.world))
    }

    fn saved_template(&mut self, name: &str) -> Result<StructureTemplate, StructureError> {
        let (home, name) = (self.structure_home(), structure_name(name));
        let Some(bytes) = template_bytes(self.world, home, &name) else {
            memory(self.world, home).missed.insert(name);
            return Err(StructureError::NotFound);
        };
        StructureTemplate::decode(&bytes).map_err(|_| StructureError::Unreadable)
    }
}

impl StructureBackend for Ctx<'_> {
    fn save_structure(&mut self, name: &str, from: BlockPos, to: BlockPos, include_entities: bool, include_blocks: bool, to_disk: bool) -> Result<(), StructureError> {
        let layer = worlds::current(self.world);
        let low: BlockPos = std::array::from_fn(|axis| from[axis].min(to[axis]));
        let size = std::array::from_fn(|axis| (from[axis] - to[axis]).abs() + 1);
        let template = capture::capture(self.world, layer, low, size, include_entities, include_blocks).ok_or(StructureError::Unloaded)?;
        let home = self.structure_home();
        keep(self.world, home, structure_name(name), template.encode(), to_disk);
        Ok(())
    }

    fn load_structure(&mut self, name: &str, at: BlockPos, how: &StructureLoad) -> Result<(), StructureError> {
        let template = self.saved_template(name)?;
        let layer = worlds::current(self.world);
        place::place(self.world, layer, &template, at, how);
        Ok(())
    }

    fn structure_size(&mut self, name: &str) -> Option<[i32; 3]> {
        self.saved_template(name).ok().map(|template| template.size)
    }

    fn delete_structure(&mut self, name: &str) -> bool {
        let (home, name) = (self.structure_home(), structure_name(name));
        let mut memory = memory(self.world, home);
        let (kept, missed) = (memory.templates.remove(&name).is_some(), memory.missed.remove(&name));
        let Some(mut layer) = self.world.get_mut::<Layer>(home) else { return kept || missed };
        let level = layer.world.level_mut();
        let mut on_disk = false;
        for templates in [&mut level.structures, &mut level.folder_structures] {
            if templates.contains_key(&name) {
                on_disk |= Arc::make_mut(templates).remove(&name).is_some();
            }
        }
        kept || missed || on_disk
    }

    fn structures(&self) -> Vec<String> {
        let home = self.structure_home();
        let mut names: BTreeSet<String> = self.world.get::<InMemory>(home).map(|memory| memory.templates.keys().cloned().collect()).unwrap_or_default();
        if let Some(level) = self.world.get::<Layer>(home).map(|layer| layer.world.level()) {
            names.extend(level.structures.keys().chain(level.folder_structures.keys()).cloned());
        }
        names.into_iter().collect()
    }

    fn structure_template(&mut self, name: &str) -> Option<Vec<u8>> {
        template_bytes(self.world, self.structure_home(), &structure_name(name))
    }

    fn set_structure_template(&mut self, name: &str, template: Vec<u8>, to_disk: bool) -> Result<(), StructureError> {
        StructureTemplate::decode(&template).map_err(|_| StructureError::Unreadable)?;
        let home = self.structure_home();
        keep(self.world, home, structure_name(name), template, to_disk);
        Ok(())
    }
}
