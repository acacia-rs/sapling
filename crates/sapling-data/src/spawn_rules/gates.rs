//! A spawn rule's gates that are not the spot's own: the mob event it waits on, the world's age,
//! its delay and that no player is in a village (`pillager_patrol`'s filters).

/// `minecraft:delay_filter`: once spawned, the rule waits `least` to `most` seconds again, and each
/// time the wait is over it spawns only by `chance` in 100 (`_spawnMobCluster` 1415ed230).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Delay {
    pub least: u32,
    pub most: u32,
    pub chance: u32,
    /// The wait's own name, one per difficulty for patrols.
    pub id: &'static str,
}

/// `minecraft:player_in_village_filter`: no player within `distance` of the spot may be in a
/// village or `tolerance` blocks round its bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Village {
    pub distance: f32,
    pub tolerance: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gates {
    /// The `/mobevent` switch it waits on.
    pub event: Option<&'static str>,
    /// The world's least age, in seconds.
    pub age: Option<u32>,
    pub delay: Option<Delay>,
    pub village: Option<Village>,
}

impl Gates {
    /// Reads a `spawn-rules.tsv` field of these into `self`; false for a field of no gate.
    pub(super) fn read(&mut self, key: &str, value: &'static str) -> bool {
        match key {
            "event" => self.event = Some(value),
            "age" => self.age = value.parse().ok(),
            "delay" => self.delay = delay(value),
            "village" => self.village = value.split_once(':').and_then(|(distance, tolerance)| Some(Village { distance: distance.parse().ok()?, tolerance: tolerance.parse().ok()? })),
            _ => return false,
        }
        true
    }

    pub fn is_open(&self) -> bool {
        *self == Self::default()
    }
}

fn delay(value: &'static str) -> Option<Delay> {
    let mut parts = value.splitn(3, ':');
    let (least, most) = parts.next()?.split_once('~')?;
    Some(Delay { least: least.parse().ok()?, most: most.parse().ok()?, chance: parts.next()?.parse().ok()?, id: parts.next()? })
}

/// A permuted kind as `kind<event>`, the event it spawns with split off.
pub fn kind_and_event(permuted: &str) -> (&str, Option<&str>) {
    match permuted.split_once('<') {
        Some((kind, event)) => (kind, event.strip_suffix('>')),
        None => (permuted, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patrols_gates_read() {
        let mut gates = Gates::default();
        assert!(gates.is_open());
        for (key, value) in [("event", "minecraft:pillager_patrols_event"), ("age", "6000"), ("delay", "600~660:20:minecraft:pillager_patrol_easy"), ("village", "48:32")] {
            assert!(gates.read(key, value));
        }
        assert!(!gates.read("light", "0~7"));
        assert_eq!(gates.delay, Some(Delay { least: 600, most: 660, chance: 20, id: "minecraft:pillager_patrol_easy" }));
        assert_eq!((gates.age, gates.village.map(|village| village.tolerance)), (Some(6000), Some(32.0)));
        assert_eq!(kind_and_event("minecraft:pillager<minecraft:spawn_as_patrol_follower>"), ("minecraft:pillager", Some("minecraft:spawn_as_patrol_follower")));
        assert_eq!(kind_and_event("minecraft:zombie"), ("minecraft:zombie", None));
    }
}
