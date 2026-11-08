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
fn swamp_hut_places_its_witch_and_black_cat_where_bds_did() {
    // research/captures/bds-h3keep.txt: both at (3842.5, 72, -1818.5) at first sight.
    let bds = vec![("cat", "spawn_midnight_cat", [3842, 72, -1819]), ("witch", "entity_spawned", [3842, 72, -1819])];
    assert_eq!(actors((239, 241), (-115, -113)), bds);
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

#[test]
fn bastion_mob_pieces_place_their_saved_actors_where_bds_did() {
    // research/captures/bds-az3bastion.txt (and bds-t12actors2.txt, the same 51): kind, feet, yaw as
    // AddEntity showed them, its yaw a byte (1.40625 a step).
    let bds = [("hoglin", [-311.48413, 39.0, 252.55], -90.0), ("hoglin", [-311.48413, 45.0, 276.55], -90.0), ("hoglin", [-312.48413, 33.0, 252.55], -90.0), ("hoglin", [-312.48413, 51.0, 275.55], -90.0), ("hoglin", [-313.45, 33.0, 283.48413], -180.0), ("hoglin", [-313.45, 39.0, 243.48415], -180.0), ("hoglin", [-313.45, 45.0, 283.48413], -180.0), ("hoglin", [-313.48413, 51.0, 253.55], -90.0), ("hoglin", [-328.51587, 45.0, 284.45], 90.0), ("hoglin", [-329.51587, 45.0, 276.45], 90.0), ("hoglin", [-329.51587, 51.0, 242.45], 90.0), ("hoglin", [-330.45, 33.0, 251.48415], -180.0), ("hoglin", [-330.48413, 45.0, 251.55], -90.0), ("hoglin", [-331.51587, 39.0, 246.45], 90.0), ("piglin", [-310.33652, 51.0, 243.37209], 90.0), ("piglin", [-311.6043, 33.0, 274.447], -180.0), ("piglin", [-312.33652, 33.0, 244.37209], 90.0), ("piglin", [-313.3721, 51.0, 245.6635], -180.0), ("piglin", [-313.66348, 33.0, 246.62791], -90.0), ("piglin", [-328.553, 33.0, 281.6043], 90.0), ("piglin", [-328.6279, 39.0, 282.33652], 0.0), ("piglin", [-329.447, 33.0, 283.3957], -90.0), ("piglin", [-330.3957, 39.0, 277.553], 0.0), ("piglin", [-331.3957, 51.0, 274.553], 0.0), ("piglin", [-331.66348, 33.0, 244.62791], -90.0), ("piglin", [-331.66348, 39.0, 284.6279], -90.0), ("piglin", [-332.33652, 39.0, 251.37209], 90.0), ("piglin", [-332.447, 39.0, 254.39569], -90.0), ("piglin", [-341.66348, 58.0, 263.6279], -90.0), ("piglin", [-342.33652, 58.0, 284.3721], 90.0), ("piglin", [-342.553, 37.0, 280.6043], 90.0), ("piglin", [-343.6279, 59.0, 246.3365], 0.0), ("piglin", [-345.3957, 33.0, 276.553], 0.0), ("piglin", [-345.553, 33.0, 244.60431], 90.0), ("piglin", [-346.33652, 37.0, 279.3721], 90.0), ("piglin", [-346.6279, 61.0, 243.3365], 0.0), ("piglin", [-346.66348, 72.0, 249.62791], -90.0), ("piglin", [-350.3957, 66.0, 251.55302], 0.0), ("piglin_brute", [-310.30182, 51.0, 253.37201], 40.78125), ("piglin_brute", [-312.69818, 51.0, 251.62799], -137.8125), ("piglin_brute", [-329.30182, 51.0, 276.372], 40.78125), ("piglin_brute", [-337.69818, 35.0, 269.628], -137.8125), ("piglin_brute", [-342.628, 37.0, 248.30182], -47.8125), ("piglin_brute", [-343.30182, 72.0, 247.37201], 40.78125), ("piglin_brute", [-344.30182, 58.0, 263.372], 40.78125), ("piglin_brute", [-344.30182, 72.0, 251.37201], 40.78125), ("piglin_brute", [-344.372, 58.0, 283.69818], 130.78125), ("piglin_brute", [-346.69818, 37.0, 247.62799], -137.8125), ("piglin_brute", [-346.69818, 58.0, 285.628], -137.8125), ("piglin_brute", [-349.30182, 74.0, 251.37201], 40.78125), ("piglin_brute", [-350.69818, 66.0, 252.62799], -137.8125)];
    let generator = crate::NetherGenerator::new(SEED);
    let mut placed: Vec<_> = (-22..=-20).flat_map(|cx| (15..=17).map(move |cz| (cx, cz))).flat_map(|(cx, cz)| generator.chunk_blocks(cx, cz).actors().to_vec()).collect();
    placed.sort_by(|a, b| (a.kind.trim_start_matches("minecraft:"), a.feet).partial_cmp(&(b.kind.trim_start_matches("minecraft:"), b.feet)).unwrap());
    let mut bds = bds.to_vec();
    bds.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    assert_eq!(placed.len(), bds.len());
    for (ours, (kind, feet, yaw)) in placed.iter().zip(bds) {
        let our_yaw = ours.saved.unwrap_or_else(|| panic!("{ours:?} has no saved compound")).rotation[0];
        assert_eq!(ours.kind.trim_start_matches("minecraft:"), kind);
        assert!(ours.feet.iter().zip(feet).all(|(a, b)| (a - b).abs() < 1e-3), "{ours:?} against {feet:?}");
        assert!((our_yaw - yaw).abs() < 1.40625, "{ours:?} against yaw {yaw}");
    }
    // The brutes' HomePos as BDS's world saved them a minute on (12 of its 13), turned as their feet.
    let homes: Vec<[f32; 3]> = placed.iter().filter_map(|actor| actor.saved?.home).collect();
    let bds_homes = [[-313, 52, 251], [-310, 52, 254], [-344, 73, 252], [-329, 52, 277], [-338, 36, 269], [-351, 67, 252], [-344, 59, 264], [-347, 38, 247], [-347, 59, 285], [-343, 73, 248], [-342, 38, 248], [-349, 75, 252]];
    assert_eq!(homes.len(), 13);
    assert!(bds_homes.iter().all(|home| homes.contains(&home.map(|c| c as f32))), "{homes:?}");
}
