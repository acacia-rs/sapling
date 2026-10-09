//! `decorate_trace <seed> <rule substring> <x0> <z0> <x1> <z1>...`: decorates every source chunk of the regions
//! and their ring (a thread per region) and prints the matching rules' features as TSV for
//! scripts/dripstone_floor.py: `FEAT` (chunk, rule, try, position), then that feature's `CLUSTER` radii,
//! `POINTED` positions and `W` writes.
use std::fmt::Write;

use sapling_worldgen::OverworldGenerator;

fn numbers(text: &str) -> Vec<i32> {
    text.split(|c: char| !c.is_ascii_digit() && c != '-').filter_map(|v| v.parse().ok()).collect()
}

fn tab(values: &[i32]) -> String {
    values.iter().map(i32::to_string).collect::<Vec<_>>().join("\t")
}

/// One try's lines: the feature's own lines come before the rule's.
fn emit(out: &mut String, inner: &[String], line: &str, wanted: &str) {
    let Some((chunk, rest)) = line.split_once(") ") else { return };
    let mut words = rest.split(' ');
    let (Some(rule), Some("try"), Some(index)) = (words.next(), words.next(), words.next()) else { return };
    if !rule.contains(wanted) || rest.contains("in another biome") {
        return;
    }
    let (chunk, at) = (numbers(chunk), numbers(rest.split_once(" at ").map_or("", |(_, at)| at.split(']').next().unwrap_or(""))));
    writeln!(out, "FEAT\t{}\t{rule}\t{index}\t{}", tab(&chunk), tab(&at)).unwrap();
    for inner in inner {
        let inner = inner.trim_start();
        if let Some(radii) = inner.strip_prefix("cluster ") {
            writeln!(out, "CLUSTER\t{}", tab(&numbers(radii)[..2])).unwrap();
        } else if let Some(at) = inner.strip_prefix("pointed ") {
            writeln!(out, "POINTED\t{}", tab(&numbers(at)[..3])).unwrap();
        } else if let Some(write) = inner.strip_prefix("write ") {
            writeln!(out, "W\t{}", tab(&numbers(write)[..3])).unwrap();
        }
    }
}

fn region(seed: i64, wanted: &str, r: &[i32]) -> String {
    let generator = OverworldGenerator::new(seed);
    let mut out = String::new();
    // Lower phases first and one chunk further out, so no traced source runs inside another's trace.
    let mut chunks: Vec<(i32, i32)> = (r[0] - 2..=r[2] + 2).flat_map(|x| (r[1] - 2..=r[3] + 2).map(move |z| (x, z))).collect();
    chunks.sort_by_key(|&(x, z)| [0, 1, 3, 2][((x & 1) | (z & 1) << 1) as usize]);
    for (cx, cz) in chunks {
        let lines = generator.source_feature_trace(cx, cz);
        if !(r[0] - 1..=r[2] + 1).contains(&cx) || !(r[1] - 1..=r[3] + 1).contains(&cz) {
            continue;
        }
        let mut inner = Vec::new();
        for line in lines {
            if line.starts_with(' ') {
                inner.push(line);
            } else {
                emit(&mut out, &inner, &line, wanted);
                inner.clear();
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: i64 = args[0].parse().unwrap();
    let wanted = &args[1];
    let bounds: Vec<i32> = args[2..].iter().map(|v| v.parse().unwrap()).collect();
    let threads = std::thread::available_parallelism().map_or(4, usize::from).min(8);
    let regions: Vec<&[i32]> = bounds.chunks(4).collect();
    for batch in regions.chunks(threads) {
        let outputs: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = batch.iter().map(|r| scope.spawn(move || region(seed, wanted, r))).collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        outputs.iter().for_each(|out| print!("{out}"));
    }
}
