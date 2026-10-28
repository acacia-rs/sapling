use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use crate::components::{InLayer, Outgoing, Stage};

/// Hands `packets` to every player of `layer` whose view covers the column of `pos`, joined or
/// still loading: a client that was sent the column must hear what happens in it.
pub(crate) fn to_viewers(world: &mut World, layer: Entity, pos: BlockPos, packets: &[Bytes]) {
    to_viewers_but(world, layer, pos, None, packets);
}

/// [`to_viewers`], but for the player `but`.
pub(crate) fn to_viewers_but(world: &mut World, layer: Entity, pos: BlockPos, but: Option<Entity>, packets: &[Bytes]) {
    let packets = &*crate::world_notices::heard(world, layer, packets);
    for player in viewers(world, layer, pos) {
        if Some(player) != but
            && let Some(mut out) = world.get_mut::<Outgoing>(player)
        {
            out.0.extend_from_slice(packets);
        }
    }
}

/// The players [`to_viewers`] hands packets about `pos` to.
pub(crate) fn viewers(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<Entity> {
    let column = ChunkPos::of_block(pos[0], pos[2]);
    let mut players = world.query::<(Entity, &InLayer, &Stage, &View)>();
    players.iter(world).filter(|(_, in_layer, stage, view)| in_layer.0 == layer && **stage >= Stage::Chunks && view.covers(column)).map(|(player, ..)| player).collect()
}

/// Sends `columns` again to the players of `layer` still waiting on blobs: an update reaching a
/// client before the section's blob is lost, and the blob was taken before the write.
pub(crate) fn resend_to_streaming(world: &mut World, layer: Entity, columns: &[ChunkPos]) {
    let mut players = world.query::<(&InLayer, &crate::blobs::BlobCache, &mut View)>();
    for (in_layer, blobs, mut view) in players.iter_mut(world) {
        if in_layer.0 == layer && blobs.awaits_any() {
            columns.iter().for_each(|&column| view.resend(column));
        }
    }
}

/// The chunk columns a player sees: a circle of `radius` chunks around `center`, split into
/// those already sent and those still queued.
#[derive(Component, Default)]
pub(crate) struct View {
    radius: i32,
    center: Option<ChunkPos>,
    moves: u64,
    sent: HashSet<ChunkPos>,
    /// Columns in the circle not sent yet, nearest first.
    queue: Vec<ChunkPos>,
}

fn distance_sq(a: ChunkPos, b: ChunkPos) -> i32 {
    (a.x - b.x).pow(2) + (a.z - b.z).pow(2)
}

impl View {
    pub fn radius(&self) -> i32 {
        self.radius
    }

    pub fn center(&self) -> Option<ChunkPos> {
        self.center
    }

    /// How often the view moved or changed size: what it covers is the same while this is.
    pub fn moves(&self) -> u64 {
        self.moves
    }

    /// Takes effect at the next [`View::recenter`], even without moving.
    pub fn set_radius(&mut self, radius: i32) {
        if self.radius != radius {
            self.radius = radius;
            self.center = None;
            self.moves += 1;
        }
    }

    /// Moves the view to `center`; false when nothing changed. Columns left behind are
    /// forgotten, columns that came into view are queued.
    pub fn recenter(&mut self, center: ChunkPos) -> bool {
        if self.center == Some(center) {
            return false;
        }
        self.center = Some(center);
        self.moves += 1;
        let radius = self.radius;
        let inside = |p: &ChunkPos| distance_sq(*p, center) <= radius * radius;
        self.sent.retain(inside);
        let square = (-radius..=radius).flat_map(|dx| (-radius..=radius).map(move |dz| ChunkPos::new(center.x + dx, center.z + dz)));
        self.queue = square.filter(|p| inside(p) && !self.sent.contains(p)).collect();
        self.queue.sort_by_key(|p| (distance_sq(*p, center), p.x, p.z));
        true
    }

    /// Forgets every column: the player is in another world, where all of them are to be sent
    /// and nothing queued for the old one is. Returns the columns the client has that a view
    /// around `to` will not send again, in order.
    pub fn leave(&mut self, to: ChunkPos) -> Vec<ChunkPos> {
        let radius = self.radius;
        let mut left: Vec<ChunkPos> = self.sent.drain().filter(|pos| distance_sq(*pos, to) > radius * radius).collect();
        left.sort();
        self.center = None;
        self.queue.clear();
        left
    }

    /// Queues a column the client has, ahead of the rest: what it was sent is outdated.
    pub fn resend(&mut self, pos: ChunkPos) {
        if self.sent.remove(&pos) {
            self.queue.insert(0, pos);
        }
    }

    /// Whether the client was sent the column and still has it.
    pub fn has_sent(&self, pos: ChunkPos) -> bool {
        self.sent.contains(&pos)
    }

    /// Whether the column is in view, sent or not.
    pub fn covers(&self, pos: ChunkPos) -> bool {
        self.center.is_some_and(|center| distance_sq(pos, center) <= self.radius * self.radius)
    }

    /// Whether a column within `radius` chunks of the centre is still queued.
    pub fn is_waiting_within(&self, radius: i32) -> bool {
        let Some(center) = self.center else { return true };
        self.queue.first().is_some_and(|nearest| distance_sq(*nearest, center) <= radius * radius)
    }

    /// Takes up to `budget` queued columns that are `ready`, nearest first, and marks them sent.
    /// Only the nearest `2 * budget` are asked about, which bounds the loads one player queues.
    pub fn take_ready(&mut self, budget: usize, mut ready: impl FnMut(ChunkPos) -> bool) -> Vec<ChunkPos> {
        let mut taken = Vec::new();
        let mut asked = 0;
        self.queue.retain(|&pos| {
            if asked == budget * 2 || taken.len() == budget {
                return true;
            }
            asked += 1;
            let is_ready = ready(pos);
            if is_ready {
                taken.push(pos);
            }
            !is_ready
        });
        self.sent.extend(&taken);
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at<const N: usize>(points: [(i32, i32); N]) -> Vec<ChunkPos> {
        points.map(|(x, z)| ChunkPos::new(x, z)).to_vec()
    }

    #[test]
    fn queues_a_circle_once_then_only_what_enters() {
        let mut view = View::default();
        view.set_radius(2);
        assert!(view.recenter(ChunkPos::new(0, 0)));
        let first = view.take_ready(100, |_| true);
        assert_eq!(first.len(), 13, "the 5x5 square without its corners and edge neighbours");
        assert_eq!(first[0], ChunkPos::new(0, 0));
        assert!(!first.contains(&ChunkPos::new(2, 1)) && view.covers(ChunkPos::new(0, 2)) && !view.covers(ChunkPos::new(2, 1)));
        assert!(!view.recenter(ChunkPos::new(0, 0)));

        assert!(view.recenter(ChunkPos::new(1, 0)));
        assert_eq!(view.take_ready(100, |_| true), at([(2, -1), (2, 1), (1, -2), (1, 2), (3, 0)]));
        // Coming back re-sends what the move east dropped.
        view.recenter(ChunkPos::new(0, 0));
        assert_eq!(view.take_ready(100, |_| true), at([(-1, -1), (-1, 1), (-2, 0), (0, -2), (0, 2)]));
        assert!(view.take_ready(100, |_| true).is_empty());
    }

    #[test]
    fn a_new_radius_applies_without_moving() {
        let mut view = View::default();
        view.set_radius(1);
        view.recenter(ChunkPos::new(4, 4));
        assert_eq!(view.take_ready(100, |_| true).len(), 5);
        view.set_radius(1);
        assert!(!view.recenter(ChunkPos::new(4, 4)));
        view.set_radius(2);
        assert!(view.recenter(ChunkPos::new(4, 4)));
        assert_eq!(view.take_ready(100, |_| true).len(), 8);
        view.set_radius(1);
        view.recenter(ChunkPos::new(4, 4));
        assert!(view.take_ready(100, |_| true).is_empty(), "shrinking sends nothing");
        assert_eq!(view.sent.len(), 5);
    }

    #[test]
    fn sending_is_budgeted_nearest_first_and_skips_what_is_not_ready() {
        let mut view = View::default();
        view.set_radius(2);
        view.recenter(ChunkPos::new(0, 0));
        assert!(view.is_waiting_within(0));
        assert_eq!(view.take_ready(3, |_| true), at([(0, 0), (-1, 0), (0, -1)]));
        assert!(!view.is_waiting_within(0) && view.is_waiting_within(1));

        // (0, 1) is not loaded yet: it keeps its place, later columns go first, and only
        // 2 x budget columns are asked about.
        let mut asked = Vec::new();
        let ready = |pos: ChunkPos| {
            asked.push(pos);
            pos != ChunkPos::new(0, 1)
        };
        assert_eq!(view.take_ready(2, ready), at([(1, 0), (-1, -1)]));
        assert_eq!(asked, at([(0, 1), (1, 0), (-1, -1)]));
        assert!(view.is_waiting_within(1));
        let mut asked = 0;
        assert!(view.take_ready(2, |_| { asked += 1; false }).is_empty());
        assert_eq!(asked, 4);
        assert_eq!(view.take_ready(1, |_| true), at([(0, 1)]));

        // Moving on drops what was queued behind and keeps what was sent and still in view.
        view.recenter(ChunkPos::new(1, 0));
        let rest = view.take_ready(100, |_| true);
        assert!(!rest.contains(&ChunkPos::new(0, 0)) && rest.contains(&ChunkPos::new(3, 0)) && !rest.contains(&ChunkPos::new(-2, 0)));
        assert_eq!(view.sent.len(), 13);
    }
}
