//! `locate_check <seed> <commands.idx> <bds log> [kind]`: checks structure starts against BDS's
//! `/locate structure` answers (one locate per index line, `<kind> <x> <z>`, answers in log
//! order). Per answer: is BDS's chunk a start of ours, and does our `/locate` give the same chunk.

use std::collections::BTreeMap;

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::structure::StructureKind;

fn answer(line: &str) -> Option<Option<(i32, i32)>> {
    if let Some(rest) = line.split("is at block ").nth(1) {
        let mut parts = rest.split(", ");
        let x = parts.next()?.parse().ok()?;
        let z = parts.nth(1)?.split(' ').next()?.parse().ok()?;
        return Some(Some((x, z)));
    }
    (line.contains("Could not") || line.contains("Unable") || line.contains("could not")).then_some(None)
}

#[derive(Default)]
struct Counts {
    answers: u32,
    ours: u32,
    same_locate: u32,
    located: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, index, log, rest @ ..] = &args[..] else { panic!("usage: locate_check <seed> <commands.idx> <bds log> [kind]") };
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let queries: Vec<(String, i32, i32)> = std::fs::read_to_string(index)
        .unwrap()
        .lines()
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            (p[0].to_string(), p[1].parse().unwrap(), p[2].parse().unwrap())
        })
        .collect();
    let answers: Vec<Option<(i32, i32)>> = std::fs::read_to_string(log).unwrap().lines().filter_map(answer).collect();
    assert_eq!(queries.len(), answers.len(), "one answer per locate");
    let mut tally: BTreeMap<&str, Counts> = BTreeMap::new();
    for ((name, ox, oz), found) in queries.iter().zip(&answers) {
        let Some(kind) = StructureKind::ALL.iter().copied().find(|k| k.name() == name) else { continue };
        if rest.first().is_some_and(|only| only != name) {
            continue;
        }
        let counts = tally.entry(kind.name()).or_default();
        // Jigsaw structures answer with their start piece's position, up to a chunk off the start chunk.
        let slack = if matches!(kind, StructureKind::TrialChambers | StructureKind::TrailRuins) { 1 } else { 0 };
        let bds = found.map(|(x, z)| (x.div_euclid(16), z.div_euclid(16)));
        if let Some(chunk) = bds {
            counts.answers += 1;
            let near = (-slack..=slack).flat_map(|dx| (-slack..=slack).map(move |dz| (chunk.0 + dx, chunk.1 + dz)));
            if near.clone().any(|(x, z)| generator.is_structure_start(kind, x, z)) {
                counts.ours += 1;
            } else {
                println!("{name}: BDS start at chunk {chunk:?} (from {ox},{oz}) is not ours");
            }
        }
        if !matches!(kind, StructureKind::Mineshaft | StructureKind::Stronghold) {
            counts.located += 1;
            let ours = generator.locate_structure(kind, *ox, *oz);
            let agree = match (ours, bds) {
                (Some(a), Some(b)) => (a.0 - b.0).abs() <= slack && (a.1 - b.1).abs() <= slack,
                (a, b) => a == b,
            };
            if agree {
                counts.same_locate += 1;
            } else {
                println!("{name}: from {ox},{oz} ours locates {ours:?}, BDS {bds:?}");
            }
        }
    }
    for (name, c) in tally {
        println!("{name:>16}: {}/{} BDS starts are ours, {}/{} locates agree", c.ours, c.answers, c.same_locate, c.located);
    }
}
