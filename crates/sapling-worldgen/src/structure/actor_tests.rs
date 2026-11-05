//! Template actors against where BDS 1.26 first showed them for seed 20261003
//! (research/captures/bds-t12actors2.txt; docs/research/structure-spawns.md "Template actors").
//! BDS's at fractions of a cell stood pushed off a wall or a neighbour by first sight.

use crate::OverworldGenerator;

const SEED: i64 = 20261003;

/// Kind, event and cell of every actor the chunks `x0..=x1`, `z0..=z1` hold, sorted.
fn actors(x: (i32, i32), z: (i32, i32)) -> Vec<(&'static str, &'static str, [i32; 3])> {
    let generator = OverworldGenerator::new(SEED);
    let mut placed: Vec<_> = (x.0..=x.1)
        .flat_map(|cx| (z.0..=z.1).map(move |cz| (cx, cz)))
        .flat_map(|(cx, cz)| generator.chunk_blocks(cx, cz).actors().to_vec())
        .map(|a| (a.kind.trim_start_matches("minecraft:"), a.event.trim_start_matches("minecraft:"), a.feet.map(|c| c.floor() as i32)))
        .collect();
    placed.sort();
    placed
}

fn sorted(mut list: Vec<(&'static str, &'static str, [i32; 3])>) -> Vec<(&'static str, &'static str, [i32; 3])> {
    list.sort();
    list
}

#[test]
fn outpost_tower_places_its_pillagers_where_bds_did() {
    let pillager = |feet| ("pillager", "entity_spawned", feet);
    let bds = [[1878, 134, -1130], [1878, 134, -1124], [1880, 138, -1127], [1881, 129, -1127], [1881, 134, -1127], [1881, 142, -1123], [1884, 134, -1124]].map(pillager);
    let mut bds = bds.to_vec();
    bds.push(("pillager", "spawn_as_illager_captain", [1881, 142, -1131]));
    // The cage's golem was falling at first sight (y 135.6), to rest at 135.
    bds.push(("iron_golem", "entity_spawned", [1868, 136, -1144]));
    assert_eq!(actors((116, 117), (-72, -71)), sorted(bds));
}

#[test]
fn village_markers_place_villagers_cats_and_a_golem_where_bds_did() {
    let villager = |feet| ("villager_v2", "spawn_from_village", feet);
    let cat = |feet| ("cat", "spawn_from_village", feet);
    let mut bds: Vec<_> = [[1766, 122, -1028], [1731, 120, -1010], [1733, 122, -1023], [1761, 122, -1043], [1733, 124, -1031], [1761, 151, -1068]].map(villager).to_vec();
    bds.extend([[1748, 122, -1026], [1746, 122, -1026], [1751, 122, -1029]].map(cat));
    bds.push(("iron_golem", "from_village", [1748, 122, -1031]));
    assert_eq!(actors((107, 111), (-68, -63)), sorted(bds));
}

#[test]
fn mansion_markers_place_its_illagers_where_bds_did() {
    let vindicator = |feet| ("vindicator", "entity_spawned", feet);
    let evoker = |feet| ("evocation_illager", "entity_spawned", feet);
    let mut bds: Vec<_> = [[-8203, 139, 15947], [-8187, 139, 15923], [-8188, 139, 15931], [-8192, 139, 15905], [-8179, 139, 15899], [-8194, 120, 15932], [-8211, 139, 15946], [-8217, 120, 15937], [-8186, 139, 15909]]
        .map(vindicator)
        .to_vec();
    bds.extend([[-8208, 139, 15947], [-8187, 139, 15928], [-8191, 139, 15898]].map(evoker));
    // BDS showed none east of chunk x -512 (x -8177), where sapling has five more.
    let near = |(_, _, [x, _, z]): &(&str, &str, [i32; 3])| (-8230..=-8177).contains(x) && (15890..=15960).contains(z);
    let placed: Vec<_> = actors((-515, -511), (993, 997)).into_iter().filter(near).collect();
    assert_eq!(placed, sorted(bds));
}
