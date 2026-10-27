//! What an armour stand wears and holds, and its pose, as BDS keeps them: `Armor` (head to feet),
//! `Mainhand` and `Offhand` (a stack each) and `Pose` with `PoseIndex` and `LastSignal`
//! (BDSRE: the stand's save at 1429b4d80).

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use crate::Error;
use crate::actors::with_fields;
use crate::item::ItemStack;
use crate::player::parts::stacks;

/// A stand's pose (`PoseIndex`); `None` for a compound without one.
pub fn actor_pose(compound: &[u8]) -> Option<i32> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    match actor.get("Pose")?.get("PoseIndex")? {
        Value::Int(index) => Some(*index),
        _ => None,
    }
}

/// `compound` wearing `armor` (head, chest, legs, feet), holding `hands` (main, off) and in `pose`.
pub fn geared_actor(compound: &[u8], armor: &[Option<ItemStack>; 4], hands: &[Option<ItemStack>; 2], pose: i32, registry: &BlockRegistry) -> Result<Vec<u8>, Error> {
    let pose = Value::Compound(vec![("LastSignal".into(), Value::Int(0)), ("PoseIndex".into(), Value::Int(pose))]);
    with_fields(compound, [
        ("Armor", stacks(armor, registry)),
        ("Mainhand", stacks(&hands[..1], registry)),
        ("Offhand", stacks(&hands[1..], registry)),
        ("Pose", pose),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::{actors, new_actor};

    #[test]
    fn a_stand_reads_back_what_it_was_given() {
        let registry = BlockRegistry::vanilla();
        let helmet = ItemStack { name: "minecraft:diamond_helmet".into(), count: 1, damage: 0, block: None, tag: None };
        let sword = ItemStack { name: "minecraft:iron_sword".into(), count: 1, damage: 0, block: None, tag: None };
        let stand = new_actor(-3, "minecraft:armor_stand", [3.5, 64.0, 0.5], [0.0; 3]);
        let geared = geared_actor(&stand, &[Some(helmet.clone()), None, None, None], &[Some(sword.clone()), None], 7, &registry).unwrap();
        let actor = actors(&geared).unwrap().remove(0);
        assert_eq!((actor.armor[0].as_ref(), actor.armor[1].as_ref(), actor.mainhand.as_ref(), actor.offhand.as_ref()), (Some(&helmet), None, Some(&sword), None));
        assert_eq!(actor_pose(&geared), Some(7));
        assert_eq!(actor_pose(&stand), None);
    }
}
