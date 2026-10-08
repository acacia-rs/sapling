//! BDS's jigsaw structures: the pools, elements and processors its C++ registers for the
//! hardcoded ones (1480bbe90 villages, 1480fe430 ancient cities) in pools.txt, the behaviour
//! packs' v2 ones in pools_v2.txt, and the assemblers that grow pieces from them.
//! docs/research/worldgen-bds-jigsaw.md.

mod assemble;
mod draw;
mod element;
mod junction;
mod piece;
mod v2;

use std::collections::HashMap;
use std::sync::OnceLock;

use super::template::{Capped, Predicate, Rule, Template, named, try_named};
use crate::block::Block;

/// A pane's or fence's states before `template::rejoin` joins it.
const UNJOINED: &str = "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0";

pub use assemble::{Assembler, Variant, middle};
pub use v2::{StartHeight, V2Start};
pub use element::{Element, ElementKind, Projection};
pub use piece::{AncientCityClass, Piece, VillageClass};

pub struct Pool {
    pub name: String,
    pub fallback: Option<String>,
    /// Each element repeated by its weight, in registration order (StructureTemplatePool ctor 145e22c70).
    pub list: Vec<&'static Element>,
}

pub struct Data {
    pub elements: &'static HashMap<String, Element>,
    pub pools: HashMap<String, Pool>,
    /// Alias set name -> (pool, replacement) pairs.
    pub aliases: HashMap<String, Vec<(String, String)>>,
    /// Structure sets of several v2 structures, each in the set's order.
    pub sets: HashMap<String, Vec<SetEntry>>,
}

/// One structure of a v2 structure set (the pack's `structure_sets` and `structures` JSON).
pub struct SetEntry {
    pub name: String,
    pub weight: i32,
    pub pool: String,
    pub max_depth: i32,
    pub horizontal: i32,
    pub padding: i32,
    /// The biomes its `biome_filters` accept.
    pub biomes: Vec<u32>,
}

impl Data {
    pub fn pool(&self, name: &str) -> Option<&Pool> {
        self.pools.get(name.strip_prefix("minecraft:").unwrap_or(name))
    }
}

/// The parsed pools.txt.
pub fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| parse(&[include_str!("pools.txt"), include_str!("pools_v2.txt")].concat()))
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn output(name: &str) -> Block {
    match name.split_once('[') {
        Some((name, states)) => named(name, states.trim_end_matches(']'), false),
        None if name == "air" => Block::Air,
        None if name == "water" => Block::Water,
        None => try_named(name, "", false).unwrap_or_else(|| named(name, UNJOINED, false)),
    }
}

fn predicate(text: &str, lists: &HashMap<String, Vec<String>>) -> Predicate {
    let chance = |s: &str| s.parse::<f32>().unwrap();
    match text.split_once('=') {
        None => Predicate::Always,
        Some(("b", block)) => Predicate::Block(block.to_string()),
        Some(("rb", rest)) => {
            let (block, p) = rest.split_once('@').unwrap();
            Predicate::RandomBlock(block.to_string(), chance(p))
        }
        Some(("nl", list)) => Predicate::NotInList(lists[list].clone()),
        Some(("lr", rest)) => {
            let (list, p) = rest.split_once('@').unwrap();
            Predicate::RandomInList(lists[list].clone(), chance(p))
        }
        _ => panic!("bad predicate {text}"),
    }
}

/// `<rule>;...` (or `-` for none).
fn rule_list(text: &str, lists: &HashMap<String, Vec<String>>) -> &'static [Rule] {
    let parsed = text
        .split(';')
        .filter(|r| *r != "-")
        .map(|r| {
            let parts: Vec<&str> = r.split('|').collect();
            Rule { input: predicate(parts[0], lists), location: predicate(parts[1], lists), output: output(parts[2]) }
        })
        .collect::<Vec<_>>();
    Box::leak(parsed.into_boxed_slice())
}

fn parse(text: &str) -> Data {
    let mut lists = HashMap::new();
    let mut rules: HashMap<String, &'static [Rule]> = HashMap::new();
    let mut capped: HashMap<String, Vec<Capped>> = HashMap::new();
    let (mut elements, mut pools, mut aliases, mut sets) = (HashMap::new(), Vec::new(), HashMap::new(), HashMap::new());
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split(' ').collect();
        match fields[0] {
            "l" => {
                lists.insert(fields[1].to_string(), fields[2].split(',').map(str::to_string).collect::<Vec<_>>());
            }
            "r" => {
                rules.insert(fields[1].to_string(), rule_list(fields[2], &lists));
            }
            "c" => {
                let limit = fields[2].parse().unwrap();
                capped.entry(fields[1].to_string()).or_default().push(Capped { limit, rules: rule_list(fields[3], &lists) });
            }
            "e" => {
                elements.insert(fields[1].to_string(), fields[2..].iter().map(|s| s.to_string()).collect::<Vec<_>>());
            }
            "p" => pools.push(fields[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>()),
            "a" => {
                let pairs = fields[2..].iter().map(|p| p.split_once('=').unwrap()).map(|(a, b)| (a.to_string(), b.to_string())).collect();
                aliases.insert(fields[1].to_string(), pairs);
            }
            "s" => {
                let int = |i: usize| fields[i].parse().unwrap();
                let biomes = fields[8].split(',').filter(|b| *b != "-").map(|b| b.parse().unwrap()).collect();
                let entry = SetEntry { name: fields[2].into(), weight: int(3), pool: fields[4].into(), max_depth: int(5), horizontal: int(6), padding: int(7), biomes };
                sets.entry(fields[1].to_string()).or_insert_with(Vec::new).push(entry);
            }
            _ => panic!("bad pools.txt line {line}"),
        }
    }
    let aliases: HashMap<String, Vec<(String, String)>> = aliases;
    let elements: HashMap<String, Element> = elements
        .into_iter()
        .map(|(name, f)| {
            let kind = match (f[0].as_str(), Template::find(&f[1])) {
                ("f", _) => ElementKind::Feature(f[1].clone()),
                (_, None) => ElementKind::Missing,
                (_, Some(template)) => ElementKind::Template {
                    template,
                    rules: if f[2] == "-" { &[] } else { rules[&f[2]] },
                    projection: if f[3] == "1" { Projection::TerrainMatching } else { Projection::Rigid },
                    post: f[4].parse().unwrap(),
                    aliases: (f[5] != "-").then(|| leak(aliases[&f[5]].clone())),
                    java: (f[0] == "j").then(|| capped.get(&f[2]).map_or(&[][..], |c| leak(c.clone()).as_slice())),
                },
            };
            (name.clone(), Element { name, kind })
        })
        .collect();
    let elements_ref: &'static HashMap<String, Element> = leak(elements);
    let empty: &'static Element = leak(Element { name: "empty".to_string(), kind: ElementKind::Empty });
    let pools = pools
        .into_iter()
        .map(|f| {
            let list = f[2..]
                .iter()
                .flat_map(|item| {
                    let (name, weight) = item.rsplit_once('*').unwrap();
                    let element = if name == "empty" { empty } else { &elements_ref[name] };
                    std::iter::repeat_n(element, weight.parse().unwrap())
                })
                .collect();
            let fallback = (f[1] != "-").then(|| f[1].clone());
            (f[0].clone(), Pool { name: f[0].clone(), fallback, list })
        })
        .collect();
    Data { elements: elements_ref, pools, aliases, sets }
}
