//! The families of mobs some rules single out: the undead, which instant health harms and Smite
//! hurts more, and the arthropods Bane of Arthropods hurts more. As the pack's `type_family`
//! names them.

const UNDEAD: &[&str] = &[
    "minecraft:zombie", "minecraft:husk", "minecraft:drowned", "minecraft:zombie_villager", "minecraft:zombie_villager_v2", "minecraft:zombie_pigman", "minecraft:zoglin",
    "minecraft:skeleton", "minecraft:stray", "minecraft:wither_skeleton", "minecraft:bogged", "minecraft:phantom", "minecraft:wither", "minecraft:skeleton_horse", "minecraft:zombie_horse",
];
const ARTHROPODS: &[&str] = &["minecraft:spider", "minecraft:cave_spider", "minecraft:silverfish", "minecraft:endermite", "minecraft:bee"];

/// Whether a mob of `kind` (namespaced) is undead.
pub fn is_undead(kind: &str) -> bool {
    UNDEAD.contains(&kind)
}

/// Whether a mob of `kind` (namespaced) is an arthropod.
pub fn is_arthropod(kind: &str) -> bool {
    ARTHROPODS.contains(&kind)
}
