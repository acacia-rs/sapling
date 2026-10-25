//! A world's border, which Bedrock does not have (sapling_api::WorldBorder): a player whose feet
//! leave its square is put back inside. And whether a world is hardcore, which its level says.

use bevy_ecs::prelude::*;
use sapling_api::{BorderBackend, WorldBorder};

use crate::components::{InLayer, Layer, Position, in_world};
use crate::handlers::Ctx;
use crate::{movement, worlds};

/// How far inside the border a player put back lands, so its next step does not cross again.
const INSET: f32 = 0.5;

#[derive(Component, Clone, Copy)]
struct Border(WorldBorder);

/// Whether `layer`'s world is hardcore (level.dat `IsHardcore`).
pub(crate) fn is_hardcore(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().hardcore)
}

/// Puts each player outside its world's border back inside.
pub(crate) fn keep_inside(world: &mut World) {
    for player in in_world(world) {
        let (Some(layer), Some(feet)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|position| position.feet)) else { continue };
        let Some(Border(border)) = world.get::<Border>(layer).copied() else { continue };
        if border.holds([feet[0], feet[2]]) {
            continue;
        }
        let inside = WorldBorder { radius: (border.radius - INSET).max(0.0), ..border }.clamp([feet[0], feet[2]]);
        movement::teleport(world, player, [inside[0], feet[1], inside[1]]);
    }
}

impl BorderBackend for Ctx<'_> {
    fn world_border(&self) -> Option<WorldBorder> {
        self.world.get::<Border>(worlds::current(self.world)).map(|border| border.0)
    }

    fn set_world_border(&mut self, border: Option<WorldBorder>) {
        let layer = worlds::current(self.world);
        match border.filter(|border| border.radius.is_finite() && border.centre.iter().all(|c| c.is_finite())) {
            Some(border) => drop(self.world.entity_mut(layer).insert(Border(border))),
            None => drop(self.world.entity_mut(layer).remove::<Border>()),
        }
    }

    fn is_hardcore(&self) -> bool {
        is_hardcore(self.world, worlds::current(self.world))
    }
}
