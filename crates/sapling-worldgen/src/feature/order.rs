//! A diagnostic for same-phase decoration order, which BDS takes from its load pattern:
//! `SAPLING_SAME_PHASE_ORDER="x,z;x,z;..."` decorates the listed source chunks in that order,
//! each over the writes of the listed same-phase sources before it. Unset, sources of one phase
//! never see each other (docs/research/worldgen-bds-residue.md, "Same-phase order experiments").

use std::collections::HashMap;
use std::sync::OnceLock;

static RANKS: OnceLock<HashMap<(i32, i32), usize>> = OnceLock::new();

fn ranks() -> &'static HashMap<(i32, i32), usize> {
    RANKS.get_or_init(|| {
        let list = std::env::var("SAPLING_SAME_PHASE_ORDER").unwrap_or_default();
        let chunk = |text: &str| {
            let (x, z) = text.split_once(',')?;
            Some((x.trim().parse().ok()?, z.trim().parse().ok()?))
        };
        list.split([';', ':']).filter_map(chunk).enumerate().map(|(rank, source)| (source, rank)).collect()
    })
}

/// A source's place among those of its phase: unlisted ones first.
pub fn rank(source: (i32, i32)) -> usize {
    ranks().get(&source).map_or(0, |rank| rank + 1)
}

/// The listed same-phase sources two chunks from `source` that run before it, in list order.
pub fn earlier(source: (i32, i32)) -> Vec<(i32, i32)> {
    let ranks = ranks();
    let Some(&rank) = ranks.get(&source) else { return Vec::new() };
    let mut found: Vec<(usize, (i32, i32))> = [-2, 0, 2]
        .iter()
        .flat_map(|&dx| [-2, 0, 2].map(move |dz| (source.0 + dx, source.1 + dz)))
        .filter_map(|other| ranks.get(&other).filter(|&&r| r < rank).map(|&r| (r, other)))
        .collect();
    found.sort();
    found.into_iter().map(|(_, other)| other).collect()
}
