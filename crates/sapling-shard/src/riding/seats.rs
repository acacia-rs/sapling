//! Who may ride what and where each rider sits: the `minecraft:rideable` of BDS's entity
//! definitions (docs/research/vehicles.md "Seats"), and what BDS adds for the rider's own height.

use sapling_data::mob_families;

pub(crate) const PLAYER: &str = "minecraft:player";

/// One seat: taken by the rider at this index while the vehicle has this many riders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Seat {
    /// Against the vehicle's wire position, in its frame: x across, z along its yaw.
    pub position: [f32; 3],
    /// How far the rider may turn from the vehicle's facing, when it is held to it.
    pub lock: Option<f32>,
    /// What the rider's facing is turned by against the vehicle's.
    pub turn: f32,
    /// The seat's third person camera radius and relax smoothing, told instead of its turn.
    pub camera: Option<[f32; 2]>,
}

/// Who rides a kind, and how.
pub(crate) struct Layout {
    /// Seats by how many ride: `seats[count - 1]` places `count` riders.
    seats: &'static [&'static [Seat]],
    /// Families that may ride it; empty for any.
    pub riders: &'static [&'static str],
    /// It takes riders only with a saddle on (a happy ghast: a harness, riding/harness.rs).
    pub saddled: bool,
    /// Only tamed riders: a player's shoulders take a tamed parrot (its `parrot_tame` family).
    pub tamed_riders: bool,
    /// It pulls in mobs that bump into it.
    pub pulls_in: bool,
    /// Riders get off onto its top, at its middle (`dismount_mode` `on_top_center`, exit.rs).
    pub on_top: bool,
}

const fn seat(position: [f32; 3]) -> Seat {
    Seat { position, lock: None, turn: 0.0, camera: None }
}

const fn boat_seat(x: f32) -> Seat {
    Seat { position: [x, -0.2, 0.0], lock: Some(90.0), turn: -90.0, camera: None }
}

/// A player's shoulders (player.json `rideable`, `lock_rider_rotation` 0).
const fn shoulder(x: f32) -> Seat {
    Seat { position: [x, -0.2, -0.1], lock: Some(0.0), turn: 0.0, camera: None }
}

/// A harnessed happy ghast's seats, in the order riders take them (`adult_harnessed` `rideable`).
const fn ghast_seat(position: [f32; 3]) -> Seat {
    Seat { position, lock: None, turn: 0.0, camera: Some([8.0, 6.0]) }
}

const GHAST_SEATS: [Seat; 4] = [ghast_seat([0.0, 3.8, 1.7]), ghast_seat([-1.7, 3.8, 0.0]), ghast_seat([0.0, 3.8, -1.7]), ghast_seat([1.7, 3.8, 0.0])];

const BOAT: Layout = Layout { seats: &[&[boat_seat(0.0)], &[boat_seat(0.2), boat_seat(-0.6)]], riders: &[], saddled: false, tamed_riders: false, pulls_in: true, on_top: false };
const CHEST_BOAT: Layout = Layout { seats: &[&[Seat { position: [0.2, -0.2, 0.0], lock: Some(90.0), turn: 0.0, camera: None }]], ..BOAT };
const MINECART: Layout = Layout { seats: &[&[seat([0.0, -0.2, 0.0])]], ..BOAT };
const PLAYER_SHOULDERS: Layout = Layout { seats: &[&[shoulder(0.4)], &[shoulder(0.4), shoulder(-0.4)]], riders: &["parrot_tame"], tamed_riders: true, pulls_in: false, ..BOAT };
const HORSE: Layout = Layout { seats: &[&[seat([0.0, 1.1, -0.2])]], riders: &["player"], saddled: false, tamed_riders: false, pulls_in: false, on_top: false };
const HAPPY_GHAST: Layout = Layout {
    seats: &[&[GHAST_SEATS[0]], &[GHAST_SEATS[0], GHAST_SEATS[1]], &[GHAST_SEATS[0], GHAST_SEATS[1], GHAST_SEATS[2]], &GHAST_SEATS],
    saddled: true,
    on_top: true,
    ..HORSE
};
const SADDLED: Layout = Layout { saddled: true, ..HORSE };
const PIG: Layout = Layout { seats: &[&[seat([0.0, 0.63, 0.0])]], ..SADDLED };
const STRIDER: Layout = Layout { seats: &[&[seat([0.0, 1.7, -0.2])]], ..SADDLED };
const SKELETON_HORSE: Layout = Layout { riders: &["player", "skeleton", "zombie"], ..HORSE };
const ZOMBIE_HORSE: Layout = Layout { riders: &["player", "zombie"], ..HORSE };
const DONKEY: Layout = Layout { seats: &[&[seat([0.0, 0.925, -0.2])]], ..HORSE };
const MULE: Layout = Layout { seats: &[&[seat([0.0, 0.975, -0.2])]], ..HORSE };
const LLAMA: Layout = Layout { seats: &[&[seat([0.0, 1.17, -0.3])]], ..HORSE };
const CAMEL: Layout = Layout { seats: &[&[seat([0.0, 1.905, 0.5])], &[seat([0.0, 1.905, 0.5]), seat([0.0, 1.905, -0.5])]], pulls_in: true, ..HORSE };
const CAMEL_HUSK: Layout = Layout { riders: &["player", "parched", "husk"], pulls_in: false, ..CAMEL };
const CHICKEN: Layout = Layout { seats: &[&[seat([0.0, 0.48, 0.0])]], riders: &["baby_undead"], ..HORSE };
const ZOMBIE_NAUTILUS: Layout =Layout { seats: &[&[seat([0.0, 0.925, -0.2])]], riders: &["drowned"], pulls_in: true, ..HORSE };

/// The layout of a kind that takes riders (the adult's; babies are not told apart).
pub(crate) fn layout(kind: &str) -> Option<&'static Layout> {
    Some(match kind {
        "minecraft:boat" => &BOAT,
        "minecraft:chest_boat" => &CHEST_BOAT,
        "minecraft:minecart" => &MINECART,
        "minecraft:pig" => &PIG,
        "minecraft:chicken" => &CHICKEN,
        "minecraft:strider" => &STRIDER,
        "minecraft:horse" => &HORSE,
        "minecraft:skeleton_horse" => &SKELETON_HORSE,
        "minecraft:zombie_horse" => &ZOMBIE_HORSE,
        "minecraft:donkey" => &DONKEY,
        "minecraft:mule" => &MULE,
        "minecraft:llama" | "minecraft:trader_llama" => &LLAMA,
        "minecraft:camel" => &CAMEL,
        "minecraft:camel_husk" => &CAMEL_HUSK,
        "minecraft:zombie_nautilus" => &ZOMBIE_NAUTILUS,
        "minecraft:happy_ghast" => &HAPPY_GHAST,
        PLAYER => &PLAYER_SHOULDERS,
        _ => return None,
    })
}

impl Layout {
    pub fn capacity(&self) -> usize {
        self.seats.len()
    }

    /// The seat of the rider at `index` among `count`.
    pub fn seat(&self, count: usize, index: usize) -> Option<Seat> {
        self.seats.get(count.checked_sub(1)?)?.get(index).copied()
    }

    /// Whether the rider at `index` among `count` sat elsewhere at `before` (its index among how many).
    pub fn moved(&self, count: usize, index: usize, (before_count, before_index): (usize, usize)) -> bool {
        self.seat(count, index) != self.seat(before_count, before_index)
    }

    /// Whether a rider of `kind` may sit on it: a `baby` undead is also `baby_undead` (the zombies'
    /// baby groups).
    pub fn takes(&self, kind: &str, baby: bool) -> bool {
        let families = mob_families(kind);
        let baby_undead = baby && families.contains(&"undead") && self.riders.contains(&"baby_undead");
        self.riders.is_empty() || baby_undead || families.iter().any(|family| self.riders.contains(family))
    }
}

/// How far above its seat a rider's wire position is: a player's is at its eyes (research/
/// captures/bds-ride.txt: 1.02001 in a boat's -0.2), a zombie's 0.5 below (bds-ride.txt).
// TODO: only the zombie and the cow (0, bds-boats.txt) were seen; other humanoids are taken as the zombie.
pub(crate) fn rider_height(kind: &str) -> f32 {
    const HUMANOID: &[&str] = &["zombie", "skeleton", "villager", "illager", "piglin", "zombie_pigman", "witch"];
    match kind {
        PLAYER => 1.22001,
        _ if mob_families(kind).iter().any(|family| HUMANOID.contains(family)) => -0.5,
        _ => 0.0,
    }
}

/// Where a rider's wire position is, against the vehicle's, at `seat` of a vehicle facing `yaw`.
pub(crate) fn offset(seat: &Seat, rider: &str, yaw: f32) -> [f32; 3] {
    let [x, y, z] = seat.position;
    let (sin, cos) = yaw.to_radians().sin_cos();
    [x * cos - z * sin, y + rider_height(rider), x * sin + z * cos]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boat_seats_two_and_a_player_sits_at_its_eyes() {
        let boat = layout("minecraft:boat").unwrap();
        assert_eq!(boat.capacity(), 2);
        assert_eq!(boat.seat(2, 1).map(|seat| seat.position), Some([-0.6, -0.2, 0.0]));
        assert!((offset(&boat.seat(1, 0).unwrap(), PLAYER, 0.0)[1] - 1.02001).abs() < 1e-6);
        assert!((offset(&boat.seat(1, 0).unwrap(), "minecraft:zombie", 0.0)[1] + 0.7).abs() < 1e-6);
        assert!(layout("minecraft:pig").unwrap().saddled && !layout("minecraft:pig").unwrap().takes("minecraft:zombie", false));
        assert!(boat.takes("minecraft:cow", false) && layout("minecraft:cow").is_none());
        let chicken = layout("minecraft:chicken").unwrap();
        assert!(chicken.takes("minecraft:zombie", true) && !chicken.takes("minecraft:zombie", false) && !chicken.takes("minecraft:cow", true));
    }

    #[test]
    fn a_seat_turns_with_its_vehicle() {
        let [x, _, z] = offset(&seat([0.2, 0.0, 0.0]), "minecraft:cow", 90.0);
        assert!(x.abs() < 1e-6 && (z - 0.2).abs() < 1e-6, "{x} {z}");
    }
}
