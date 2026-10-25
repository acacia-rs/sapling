//! The candidates a selector's base looks at, with what its filters ask of them read from the
//! shard (and only that: inventories, tags and families are read when a filter tests them).

use super::Selected;
use super::filters::Filters;
use super::items::Inventory;
use super::matching::Candidate;
use super::Base;
use crate::{Player, Sender, ShardCtx, Slot, Tagged};

const PLAYER: &str = "minecraft:player";

/// Players first, then (for `@e`, `@n`, and `@r` with `type=`) the other entities, oldest first.
pub(super) fn candidates(ctx: &mut ShardCtx<'_>, sender: &Sender, base: Base, filters: &Filters) -> Vec<Candidate> {
    let players: Vec<Player> = match base {
        Base::Initiator => return Vec::new(),
        Base::Sender => sender.player().cloned().into_iter().collect(),
        _ => ctx.players().collect(),
    };
    // Unverified on BDS: `@r` takes entities once any `type=` is given, `!` too.
    let entities = match base {
        Base::Entities | Base::NearestEntity => ctx.entities(),
        Base::RandomPlayer if !filters.types.is_empty() => ctx.entities(),
        _ => Vec::new(),
    };
    let mut found = Vec::new();
    for player in players {
        let Some(position) = ctx.position(player.id) else { continue };
        let candidate = Candidate {
            kind: PLAYER.into(),
            name: player.name.clone(),
            feet: position.feet,
            rotation: [position.yaw, position.pitch],
            families: families(ctx, filters, PLAYER),
            tags: tags(ctx, filters, Tagged::Player(player.id)),
            mode: ctx.game_mode(player.id),
            level: ctx.experience(player.id).map(|(level, _)| level),
            inventory: (!filters.items.is_empty()).then(|| inventory(ctx, &player)),
            target: Selected::Player(player),
        };
        found.push(candidate);
    }
    for entity in entities {
        let candidate = Candidate {
            kind: entity.kind.clone(),
            name: String::new(),
            feet: entity.feet,
            rotation: [entity.yaw, entity.pitch],
            families: families(ctx, filters, &entity.kind),
            tags: tags(ctx, filters, Tagged::Entity(entity.id)),
            mode: None,
            level: None,
            inventory: None,
            target: Selected::Entity(entity),
        };
        found.push(candidate);
    }
    found
}

fn families(ctx: &ShardCtx<'_>, filters: &Filters, kind: &str) -> Vec<String> {
    if filters.families.is_empty() { Vec::new() } else { ctx.families(kind) }
}

fn tags(ctx: &mut ShardCtx<'_>, filters: &Filters, of: Tagged) -> Vec<String> {
    if filters.tags.is_empty() { Vec::new() } else { ctx.tags(of) }
}

fn inventory(ctx: &ShardCtx<'_>, player: &Player) -> Inventory {
    let slots = (0..36).map(Slot::Main).chain((0..4).map(Slot::Armor)).chain([Slot::Offhand]);
    let items = slots.filter_map(|slot| Some((slot, ctx.item(player.id, slot)?))).collect();
    Inventory { items, held: ctx.held_slot(player.id).unwrap_or(0) }
}
