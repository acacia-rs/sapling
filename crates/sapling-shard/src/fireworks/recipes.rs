//! The firework recipes, which the game lists as special ones (`CraftingData`'s multi recipes)
//! with no cells: what lies in the grid makes the result. A rocket is a paper, one to three
//! gunpowder (its `Flight`) and any stars, three to a craft; a star is a gunpowder, dyes, and at
//! most one each of a shape, a diamond (trail) and glowstone dust (flicker); a star and dyes fade it.

use sapling_data::CreativeItem;
use sapling_world::ItemStack;

use super::item::{self, Explosion, ROCKET, STAR};

const PAPER: &str = "minecraft:paper";
const GUNPOWDER: &str = "minecraft:gunpowder";
const DIAMOND: &str = "minecraft:diamond";
const GLOWSTONE: &str = "minecraft:glowstone_dust";
const ROCKETS_MADE: u8 = 3;
const MOST_GUNPOWDER: usize = 3;
/// What each shape item makes a star burst as.
const SHAPES: &[(&str, u8)] = &[("minecraft:fire_charge", 1), ("minecraft:gold_nugget", 2), ("minecraft:feather", 4)];
const HEAD_SHAPE: u8 = 3;
/// The dyes by dye colour number, and the older items that dye as one.
const DYES: [&str; 16] = ["black", "red", "green", "brown", "blue", "purple", "cyan", "light_gray", "gray", "pink", "lime", "yellow", "light_blue", "magenta", "orange", "white"];
const OLD_DYES: &[(&str, u8)] = &[("minecraft:ink_sac", 0), ("minecraft:cocoa_beans", 3), ("minecraft:lapis_lazuli", 4), ("minecraft:bone_meal", 15)];

/// The dye colour number of a dye item, banners' numbering too.
pub(crate) fn dye(name: &str) -> Option<u8> {
    let named = name.strip_prefix("minecraft:").and_then(|bare| bare.strip_suffix("_dye")).and_then(|colour| DYES.iter().position(|dye| *dye == colour));
    named.map(|colour| colour as u8).or_else(|| OLD_DYES.iter().find(|(old, _)| *old == name).map(|(_, colour)| *colour))
}

fn shape(name: &str) -> Option<u8> {
    let is_head = name.ends_with("_skull") || name.ends_with("_head");
    SHAPES.iter().find(|(item, _)| *item == name).map(|(_, shape)| *shape).or_else(|| is_head.then_some(HEAD_SHAPE))
}

fn count(grid: &[&ItemStack], name: &str) -> usize {
    grid.iter().filter(|stack| stack.name == name).count()
}

fn made(name: &'static str, count: u8, tag: acacia_proto::nbt::Value) -> (CreativeItem, u8) {
    (CreativeItem { name, damage: 0, block: None, tag: Some(tag) }, count)
}

fn rocket(grid: &[&ItemStack]) -> Option<(CreativeItem, u8)> {
    let gunpowder = count(grid, GUNPOWDER);
    let fits = count(grid, PAPER) == 1 && (1..=MOST_GUNPOWDER).contains(&gunpowder) && grid.iter().all(|stack| [PAPER, GUNPOWDER, STAR].contains(&stack.name.as_str()));
    let bursts: Vec<Explosion> = grid.iter().filter(|stack| stack.name == STAR).filter_map(|stack| item::star(stack)).collect();
    fits.then(|| made(ROCKET, ROCKETS_MADE, item::rocket_tag(gunpowder as u8, &bursts)))
}

fn star(grid: &[&ItemStack]) -> Option<(CreativeItem, u8)> {
    let colours: Vec<u8> = grid.iter().filter_map(|stack| dye(&stack.name)).collect();
    let shapes: Vec<u8> = grid.iter().filter_map(|stack| shape(&stack.name)).collect();
    let known = |stack: &&&ItemStack| dye(&stack.name).is_some() || shape(&stack.name).is_some() || [GUNPOWDER, DIAMOND, GLOWSTONE].contains(&stack.name.as_str());
    let fits = count(grid, GUNPOWDER) == 1 && !colours.is_empty() && shapes.len() <= 1 && count(grid, DIAMOND) <= 1 && count(grid, GLOWSTONE) <= 1 && grid.iter().all(|stack| known(&stack));
    let burst = Explosion { shape: shapes.first().copied().unwrap_or(0), colours, fades: Vec::new(), flicker: count(grid, GLOWSTONE) == 1, trail: count(grid, DIAMOND) == 1 };
    fits.then(|| made(STAR, 1, item::star_tag(&burst)))
}

fn faded(grid: &[&ItemStack]) -> Option<(CreativeItem, u8)> {
    let fades: Vec<u8> = grid.iter().filter_map(|stack| dye(&stack.name)).collect();
    let fits = count(grid, STAR) == 1 && !fades.is_empty() && fades.len() + 1 == grid.len();
    let burst = grid.iter().find(|stack| stack.name == STAR).and_then(|stack| item::star(stack)).filter(|_| fits)?;
    Some(made(STAR, 1, item::star_tag(&Explosion { fades, ..burst })))
}

/// What one craft of the grid's items (each cell one of them) makes by a firework recipe.
pub(crate) fn crafted(grid: &[&ItemStack]) -> Option<(CreativeItem, u8)> {
    rocket(grid).or_else(|| star(grid)).or_else(|| faded(grid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(name: &str) -> ItemStack {
        ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None }
    }

    fn of((made, count): (CreativeItem, u8)) -> ItemStack {
        ItemStack { name: made.name.into(), count, damage: 0, block: None, tag: made.tag }
    }

    fn craft(names: &[&str]) -> Option<ItemStack> {
        let stacks: Vec<ItemStack> = names.iter().map(|name| stack(name)).collect();
        crafted(&stacks.iter().collect::<Vec<_>>()).map(of)
    }

    #[test]
    fn paper_and_gunpowder_make_three_rockets_of_that_flight() {
        let made = craft(&[PAPER, GUNPOWDER, GUNPOWDER]).unwrap();
        assert_eq!((made.name.as_str(), made.count, item::flight(&made)), (ROCKET, 3, 2));
        assert!(craft(&[PAPER, GUNPOWDER, GUNPOWDER, GUNPOWDER, GUNPOWDER]).is_none());
        assert!(craft(&[PAPER, PAPER, GUNPOWDER]).is_none());
        assert!(craft(&[PAPER, GUNPOWDER, "minecraft:stone"]).is_none());
    }

    #[test]
    fn a_star_takes_its_shape_trail_and_colours_and_goes_into_a_rocket() {
        let star = craft(&[GUNPOWDER, "minecraft:red_dye", "minecraft:lapis_lazuli", "minecraft:creeper_head", DIAMOND]).unwrap();
        let burst = item::star(&star).unwrap();
        assert_eq!((burst.shape, burst.colours.clone(), burst.trail, burst.flicker), (3, vec![1, 4], true, false));
        assert!(craft(&[GUNPOWDER, "minecraft:red_dye", "minecraft:feather", "minecraft:gold_nugget"]).is_none(), "two shapes");
        let faded = of(crafted(&[&star, &stack("minecraft:white_dye")]).unwrap());
        assert_eq!(item::star(&faded).unwrap().fades, vec![15]);
        let rocket = of(crafted(&[&stack(PAPER), &stack(GUNPOWDER), &faded]).unwrap());
        assert_eq!(item::explosions(&rocket).len(), 1);
    }
}
