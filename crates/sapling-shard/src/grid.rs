//! Things by where they are, so a view finds what it covers without looking at everything in
//! the shard: cells of `CELL` chunks a side, and a view asks only the cells it reaches into.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::view::View;

const CELL: i32 = 8;

pub(crate) struct Grid<T> {
    cells: HashMap<(Entity, i32, i32), Vec<(ChunkPos, T)>>,
}

impl<T> Default for Grid<T> {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }
}

fn cell(layer: Entity, pos: ChunkPos) -> (Entity, i32, i32) {
    (layer, pos.x.div_euclid(CELL), pos.z.div_euclid(CELL))
}

impl<T: PartialEq> Grid<T> {
    pub fn insert(&mut self, layer: Entity, pos: ChunkPos, item: T) {
        self.cells
            .entry(cell(layer, pos))
            .or_default()
            .push((pos, item));
    }

    pub fn remove(&mut self, layer: Entity, pos: ChunkPos, item: &T) {
        let key = cell(layer, pos);
        let Some(items) = self.cells.get_mut(&key) else {
            return;
        };
        items.retain(|(_, other)| other != item);
        if items.is_empty() {
            self.cells.remove(&key);
        }
    }

    /// What `view` covers in `layer`.
    pub fn covered<'a>(&'a self, layer: Entity, view: &'a View) -> impl Iterator<Item = &'a T> {
        let cells = view.center().into_iter().flat_map(move |center| {
            let (_, x, z) = cell(layer, center);
            let reach = (view.radius() + CELL - 1) / CELL;
            (x - reach..=x + reach)
                .flat_map(move |x| (z - reach..=z + reach).map(move |z| (layer, x, z)))
        });
        cells
            .filter_map(|key| self.cells.get(&key))
            .flatten()
            .filter(|(pos, _)| view.covers(*pos))
            .map(|(_, item)| item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_finds_what_it_covers_and_nothing_else() {
        let mut world = World::new();
        let (here, elsewhere) = (world.spawn_empty().id(), world.spawn_empty().id());
        let mut grid = Grid::default();
        let at = |x, z| ChunkPos::new(x, z);
        for (layer, pos, name) in [
            (here, at(0, 0), "centre"),
            (here, at(-9, 0), "edge"),
            (here, at(-10, 0), "outside"),
            (here, at(7, 7), "corner"),
            (elsewhere, at(0, 0), "other world"),
        ] {
            grid.insert(layer, pos, name);
        }
        let mut view = View::default();
        assert_eq!(
            grid.covered(here, &view).count(),
            0,
            "a view without a centre covers nothing"
        );
        view.set_radius(9);
        view.recenter(at(0, 0));
        let mut found: Vec<_> = grid.covered(here, &view).copied().collect();
        found.sort();
        assert_eq!(found, ["centre", "edge"]);

        grid.remove(here, at(-9, 0), &"edge");
        grid.remove(here, at(3, 3), &"nothing there");
        assert_eq!(
            grid.covered(here, &view).copied().collect::<Vec<_>>(),
            ["centre"]
        );
        view.recenter(at(-16, 0));
        assert_eq!(
            grid.covered(here, &view).copied().collect::<Vec<_>>(),
            ["outside"]
        );
    }
}
