//! What a selector names, as vanilla works it out: the players and the other entities its pick and
//! arguments take, nearness counted from where the command runs or the selector's `x y z`.
// TODO: @r draws from the tick, not from the world's random.

use sapling_api::command::{Filter, Pick, Selector};
use sapling_api::{EntityInfo, Player, Sender, ShardCtx};

enum Found {
    Player(Player),
    Entity(EntityInfo),
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// A random order of `count` things, drawn from `seed`.
fn shuffled(count: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut state = seed | 1;
    for i in (1..count).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        order.swap(i, (state % (i as u64 + 1)) as usize);
    }
    order
}

/// The players and the other entities `selector` names, counted from `here`.
pub(super) fn targets(ctx: &ShardCtx<'_>, sender: &Sender, selector: &Selector, here: [f32; 3]) -> (Vec<Player>, Vec<EntityInfo>) {
    let (pick, filter) = match selector {
        Selector::Pick(pick, filter) => (*pick, filter),
        Selector::Named(name) => return (ctx.players().filter(|player| player.name.eq_ignore_ascii_case(name)).collect(), Vec::new()),
        _ => return (Vec::new(), Vec::new()),
    };
    let from: [f32; 3] = std::array::from_fn(|axis| filter.at[axis].map_or(here[axis], |at| at.from(here[axis])));
    let candidates: Vec<Player> = match pick {
        Pick::Sender => sender.player().cloned().into_iter().collect(),
        _ => ctx.players().collect(),
    };
    let mut found: Vec<(f32, Found)> = Vec::new();
    for player in candidates {
        let Some(feet) = ctx.position(player.id).map(|position| position.feet) else { continue };
        let mode_taken = ctx.game_mode(player.id).is_none_or(|mode| filter.takes_mode(mode));
        if Filter::matches(filter.kind.as_deref(), "minecraft:player") && Filter::matches(filter.name.as_deref(), &player.name) && mode_taken && filter.within(distance(feet, from)) {
            found.push((distance(feet, from), Found::Player(player)));
        }
    }
    // Other entities have no name and no game mode: a name or a mode asked for leaves them out.
    let unnamed = filter.name.as_deref().is_none_or(|name| name.starts_with('!'));
    if pick == Pick::Entities && unnamed && filter.game_mode.is_none() {
        for entity in ctx.entities() {
            if Filter::matches(filter.kind.as_deref(), &entity.kind) && filter.within(distance(entity.feet, from)) {
                found.push((distance(entity.feet, from), Found::Entity(entity)));
            }
        }
    }
    let one = matches!(pick, Pick::Nearest | Pick::Random).then_some(1);
    match (pick, filter.count.or(one)) {
        (Pick::Random, Some(count)) => {
            let mut drawn: Vec<Option<(f32, Found)>> = found.into_iter().map(Some).collect();
            found = shuffled(drawn.len(), ctx.tick()).into_iter().filter_map(|i| drawn[i].take()).take(count.unsigned_abs() as usize).collect();
        }
        (_, Some(count)) => {
            found.sort_by(|a, b| if count < 0 { b.0.total_cmp(&a.0) } else { a.0.total_cmp(&b.0) });
            found.truncate(count.unsigned_abs() as usize);
        }
        (_, None) => {}
    }
    let (mut players, mut entities) = (Vec::new(), Vec::new());
    for (_, one) in found {
        match one {
            Found::Player(player) => players.push(player),
            Found::Entity(entity) => entities.push(entity),
        }
    }
    (players, entities)
}

#[cfg(test)]
mod tests {
    use super::shuffled;

    #[test]
    fn a_shuffle_keeps_every_index_once() {
        let mut order = shuffled(10, 12345);
        order.sort_unstable();
        assert_eq!(order, (0..10).collect::<Vec<_>>());
        assert_ne!(shuffled(10, 1), shuffled(10, 2));
    }
}
