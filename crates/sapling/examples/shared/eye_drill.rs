//! The eye of ender's drills for the portal probe, on a default (not flat) world of seed 20261018:
//! testbox `~/work/bds-eye`, port 19210. Lines as shared/portal_drill.rs reads them.

/// Packets printed in full in an eye drill, though the probe otherwise tallies some.
pub const SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "SetEntityMotion", "MoveEntityDelta", "MoveEntity", "EntityEvent", "Animate", "RemoveEntity", "InventorySlot", "TakeItemEntity", "SetEntityData",
];

/// Where the eye drills stand: a glass floor at y 150 over the origin.
const STAND: [&str; 3] = ["fill -2 150 -2 2 150 2 glass", "tp @s 0.5 151 0.5 #3000", "!where"];
const EYES: &str = "replaceitem entity @s slot.weapon.mainhand 0 ender_eye 16";

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// The lines of the eye drill `name`, after the common start.
pub fn drill(name: &str) -> Option<Vec<String>> {
    let mut lines = owned(&STAND);
    match name {
        // Throws in creative (looking two ways), then in survival: flight, outcome, the item spent.
        "eye-throw" => {
            lines.extend(owned(&["locate structure stronghold", EYES]));
            lines.extend(owned(&["!throw 0 0 5000", "!throw 90 -45 5000", "!throw 0 0 5000", "!throw 0 0 5000", "!throw 0 0 5000"]));
            lines.extend(owned(&["gamemode survival", "!throw 0 0 5000", "!throw 0 0 5000", "gamemode creative"]));
        }
        // No throw in the Nether or the End; an eye on a frame fills it and is not thrown.
        "eye-dims" => {
            lines.extend(owned(&[EYES, "execute in nether run tp @s 0.5 100 0.5 #9000", "!where", "!throw 0 0 3000", "gamemode survival", "!throw 0 0 3000", "gamemode creative"]));
            lines.extend(owned(&["execute in the_end run tp @s 100.5 60 0.5 #9000", "!where", "!throw 0 0 3000"]));
            lines.extend(owned(&["execute in overworld run tp @s 0.5 151 0.5 #9000", "!where", EYES]));
            lines.extend(owned(&["setblock 2 151 0 end_portal_frame [\"minecraft:cardinal_direction\"=\"south\"]", "!light 2 151 0 up 3000"]));
        }
        // Within 12 blocks of the stronghold BDS located (-1454, -2542): the eye goes to its own block.
        "eye-near" => lines.extend(owned(&[EYES, "tp @s -1449.5 36 -2539.5 #6000", "!where", "!throw 0 0 5000"])),
        _ => return None,
    }
    Some(lines)
}
