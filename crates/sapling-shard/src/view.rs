use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

/// The chunk columns a player has been sent: a circle of `radius` chunks around `center`.
#[derive(Component, Default)]
pub(crate) struct View {
    radius: i32,
    center: Option<ChunkPos>,
    sent: HashSet<ChunkPos>,
}

impl View {
    pub fn radius(&self) -> i32 {
        self.radius
    }

    /// Takes effect at the next [`View::recenter`], even without moving.
    pub fn set_radius(&mut self, radius: i32) {
        if self.radius != radius {
            self.radius = radius;
            self.center = None;
        }
    }

    /// Moves the view to `center`. `None` when nothing changed, else the columns that came into
    /// view, nearest first; columns left behind are forgotten.
    pub fn recenter(&mut self, center: ChunkPos) -> Option<Vec<ChunkPos>> {
        if self.center == Some(center) {
            return None;
        }
        self.center = Some(center);
        let radius = self.radius;
        let distance = |p: &ChunkPos| (p.x - center.x).pow(2) + (p.z - center.z).pow(2);
        self.sent.retain(|p| distance(p) <= radius * radius);
        let square = (-radius..=radius).flat_map(|dx| (-radius..=radius).map(move |dz| ChunkPos::new(center.x + dx, center.z + dz)));
        let mut entered: Vec<ChunkPos> = square.filter(|p| distance(p) <= radius * radius && !self.sent.contains(p)).collect();
        entered.sort_by_key(|p| (distance(p), p.x, p.z));
        self.sent.extend(&entered);
        Some(entered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_a_circle_once_then_only_what_enters() {
        let mut view = View::default();
        view.set_radius(2);
        let first = view.recenter(ChunkPos::new(0, 0)).unwrap();
        assert_eq!(first.len(), 13, "the 5x5 square without its corners and edge neighbours");
        assert_eq!(first[0], ChunkPos::new(0, 0));
        assert!(!first.contains(&ChunkPos::new(2, 1)));
        assert_eq!(view.recenter(ChunkPos::new(0, 0)), None);

        let at = |points: [(i32, i32); 5]| points.map(|(x, z)| ChunkPos::new(x, z));
        let east = view.recenter(ChunkPos::new(1, 0)).unwrap();
        assert_eq!(east, at([(2, -1), (2, 1), (1, -2), (1, 2), (3, 0)]));
        // Coming back re-sends what the move east dropped.
        let back = view.recenter(ChunkPos::new(0, 0)).unwrap();
        assert_eq!(back, at([(-1, -1), (-1, 1), (-2, 0), (0, -2), (0, 2)]));
    }

    #[test]
    fn a_new_radius_applies_without_moving() {
        let mut view = View::default();
        view.set_radius(1);
        assert_eq!(view.recenter(ChunkPos::new(4, 4)).unwrap().len(), 5);
        view.set_radius(1);
        assert_eq!(view.recenter(ChunkPos::new(4, 4)), None);
        view.set_radius(2);
        assert_eq!(view.recenter(ChunkPos::new(4, 4)).unwrap().len(), 8);
        view.set_radius(1);
        assert!(view.recenter(ChunkPos::new(4, 4)).unwrap().is_empty(), "shrinking sends nothing");
        assert_eq!(view.sent.len(), 5);
    }
}
