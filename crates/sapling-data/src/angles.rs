//! The byte BDS sends a turn in, and the turn an announcement carries
//! (research/bds-actor-move-sync.md, "Turn bytes").

/// The byte BDS tells a turn of `degrees` in: wrapped to -180..180, then cut toward zero
/// (research/captures/bds-g8rot.txt: -29.5 and 330.5 are 236, -0.7 is 0, 200 is 143).
pub fn angle_byte(degrees: f32) -> u8 {
    (((degrees + 180.0).rem_euclid(360.0) - 180.0) / 1.40625) as i32 as u8
}

/// The turn of `degrees` as an `AddEntity` carries it: its byte back in degrees, -180 to 180.
/// Every angle of 1045 BDS announcements sat on a byte (an armour stand summoned at 30 is told 29.53125).
pub fn told_angle(degrees: f32) -> f32 {
    f32::from(angle_byte(degrees) as i8) * 1.40625
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_announced_turn_is_its_byte_in_degrees() {
        assert_eq!([30.0, -29.5, 200.0, 0.0, 90.0].map(told_angle), [29.53125, -28.125, -158.90625, 0.0, 90.0]);
    }
}
