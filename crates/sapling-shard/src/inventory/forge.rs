//! The server's side of a craft at a workstation (enchanting table, anvil, grindstone, smithing
//! table, loom (loom.rs), a beacon's payment): the server works out the result from the screen's own slots, as BDS's `_screenSimCraft`
//! does, and the request must use up exactly what that result takes. The rules:
//! `sapling_data::enchant` and `sapling_data::smithed`; the request shapes and BDS's numbers:
//! docs/research/workstations.md.

use acacia_proto::types::ContainerSlotType;
use sapling_data::Made;
use sapling_data::enchant::{Offer, anvil, grind, table};
use sapling_data::trades::ANY_DAMAGE;
use sapling_world::ItemStack;

use super::{Held, Inventory, Place, stack};

mod loom;

const ANVIL_INPUT: usize = 1;
const ANVIL_MATERIAL: usize = 2;
const ENCHANTING_INPUT: usize = 14;
const LAPIS: usize = 15;
const GRINDSTONE_INPUT: usize = 16;
const GRINDSTONE_ADDITIONAL: usize = 17;
const SMITHING_INPUT: usize = 51;
const SMITHING_MATERIAL: usize = 52;
const SMITHING_TEMPLATE: usize = 53;
/// The trading screen's two payment slots.
const PAYMENT: [usize; 2] = [4, 5];
const BEACON_PAYMENT: usize = 27;

/// The `Ui` slot a request names a workstation's slot of the kind `kind` by.
pub(super) fn ui_slot(kind: ContainerSlotType) -> Option<usize> {
    Some(match kind {
        ContainerSlotType::AnvilInput => ANVIL_INPUT,
        ContainerSlotType::AnvilMaterial => ANVIL_MATERIAL,
        ContainerSlotType::EnchantingInput => ENCHANTING_INPUT,
        ContainerSlotType::EnchantingLapis => LAPIS,
        ContainerSlotType::GrindstoneInput => GRINDSTONE_INPUT,
        ContainerSlotType::GrindstoneAdditional => GRINDSTONE_ADDITIONAL,
        ContainerSlotType::SmithingTableInput => SMITHING_INPUT,
        ContainerSlotType::SmithingTableMaterial => SMITHING_MATERIAL,
        ContainerSlotType::SmithingTableTemplate => SMITHING_TEMPLATE,
        ContainerSlotType::Trade2Ingredient1 | ContainerSlotType::TradeIngredient1 => PAYMENT[0],
        ContainerSlotType::Trade2Ingredient2 | ContainerSlotType::TradeIngredient2 => PAYMENT[1],
        ContainerSlotType::LoomInput => loom::LOOM_INPUT,
        ContainerSlotType::LoomDye => loom::LOOM_DYE,
        ContainerSlotType::LoomMaterial => loom::LOOM_MATERIAL,
        ContainerSlotType::BeaconPayment => BEACON_PAYMENT,
        _ => return None,
    })
}

/// What lies in the enchanting table's input slot.
pub(crate) fn enchanting_input(inventory: &Inventory) -> Option<&ItemStack> {
    held(inventory, ENCHANTING_INPUT).map(|held| &held.stack)
}

/// The workstation whose screen a request is made at.
#[derive(Clone)]
pub(crate) enum Bench {
    /// With what it offers, each under the recipe id a request names it by.
    Enchanting(Vec<(u32, Offer)>),
    Anvil,
    Grindstone,
    Smithing,
    /// A trader's screen, with its offers.
    Trade(Vec<Bargain>),
    Loom,
    /// A beacon's screen, with the beacon's level.
    Beacon(i32),
}

/// An offer of an open trading screen as a request pays for it: the recipe id it is named by,
/// what it wants at its price now (a `Damage` of [`ANY_DAMAGE`] takes any), what it gives.
#[derive(Clone, Debug)]
pub(crate) struct Bargain {
    pub net_id: u32,
    pub wants: [Option<ItemStack>; 2],
    pub gives: ItemStack,
    pub sold_out: bool,
}

/// What a done craft leaves the world to do (workstations.rs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Forged {
    Enchanted,
    /// At an anvil, which may wear.
    Repaired,
    /// Experience let out, from the first to the second.
    Ground([u32; 2]),
    Smithed,
    /// The trader's offer of this place was traded once.
    Traded(usize),
    Loomed,
    /// A beacon paid for its primary and secondary effects.
    Beaconed([i32; 2]),
}

/// A craft at a workstation, and what the request used up so far.
pub(super) struct Forge {
    takes: Vec<(Place, u8)>,
    /// A slot a creative player may leave alone: lapis.
    spared: Option<Place>,
    used: Vec<(Place, u8)>,
    pub done: Forged,
}

fn held(inventory: &Inventory, slot: usize) -> Option<&Held> {
    inventory.slot(Place::Ui(slot)).and_then(Option::as_ref)
}

/// Lays `made` in the created-output slot, `count` of it, as `from` was.
fn lay(inventory: &mut Inventory, from: &ItemStack, made: Made, count: u8) -> Result<(), ()> {
    if inventory.slot(Place::Created).is_some_and(Option::is_some) {
        return Err(());
    }
    inventory.put(Place::Created, Some(ItemStack { name: made.name, count, damage: from.damage, block: from.block, tag: made.tag }));
    Ok(())
}

/// Takes `levels` from a player out of creative, which it must have.
fn pay(inventory: &mut Inventory, levels: i32, needs: i32, creative: bool) -> Result<(), ()> {
    if creative {
        return Ok(());
    }
    if inventory.experience.level < needs.max(levels) {
        return Err(());
    }
    inventory.experience.level -= levels;
    Ok(())
}

impl Forge {
    fn new(takes: Vec<(Place, u8)>, done: Forged) -> Self {
        Self { takes, spared: None, used: Vec::new(), done }
    }

    /// The craft a `CraftRecipe` names at an enchanting table (one of its offers) or a smithing
    /// table (a smithing recipe).
    pub fn by_recipe(inventory: &mut Inventory, bench: &Bench, id: u32, creative: bool) -> Result<Self, ()> {
        match bench {
            Bench::Enchanting(offers) => Self::enchant(inventory, offers, id, creative),
            Bench::Smithing => Self::smith(inventory, id),
            Bench::Trade(bargains) => Self::trade(inventory, bargains, id),
            Bench::Anvil | Bench::Grindstone | Bench::Loom | Bench::Beacon(_) => Err(()),
        }
    }

    /// A beacon's payment (`BeaconPayment`), which the request then destroys: one of a beacon's
    /// payment items, for effects a beacon of `level` offers.
    pub fn beacon(inventory: &Inventory, level: i32, primary: i32, secondary: i32) -> Result<Self, ()> {
        let paid = held(inventory, BEACON_PAYMENT).is_some_and(|held| crate::beacons::is_payment(&held.stack.name));
        if !paid || !crate::beacons::offers(level, primary, secondary) {
            return Err(());
        }
        Ok(Self::new(vec![(Place::Ui(BEACON_PAYMENT), 1)], Forged::Beaconed([primary, secondary])))
    }

    fn enchant(inventory: &mut Inventory, offers: &[(u32, Offer)], id: u32, creative: bool) -> Result<Self, ()> {
        let index = offers.iter().position(|(offered, _)| *offered == id).ok_or(())?;
        let offer = &offers[index].1;
        let item = held(inventory, ENCHANTING_INPUT).ok_or(())?.stack.clone();
        let lapis = table::lapis(index);
        let has_lapis = held(inventory, LAPIS).is_some_and(|held| held.stack.name == table::LAPIS && held.stack.count >= lapis);
        if !creative && !has_lapis {
            return Err(());
        }
        let made = table::enchanted(&stack(&item), offer).ok_or(())?;
        lay(inventory, &item, made, 1)?;
        pay(inventory, table::levels_taken(index), offer.cost.into(), creative)?;
        let mut forge = Self::new(vec![(Place::Ui(ENCHANTING_INPUT), 1), (Place::Ui(LAPIS), lapis)], Forged::Enchanted);
        forge.spared = creative.then_some(Place::Ui(LAPIS));
        Ok(forge)
    }

    /// An anvil's craft (`CraftRecipeOptional`), renaming to `name` when the request names one.
    pub fn repair(inventory: &mut Inventory, name: Option<&str>, creative: bool) -> Result<Self, ()> {
        let input = held(inventory, ANVIL_INPUT).ok_or(())?.stack.clone();
        let material = held(inventory, ANVIL_MATERIAL).map(|held| held.stack.clone());
        let repair = anvil::repair(&stack(&input), material.as_ref().map(stack).as_ref(), name).ok_or(())?;
        if !anvil::allowed(repair.cost, creative) {
            return Err(());
        }
        lay(inventory, &input, repair.made, input.count)?;
        pay(inventory, repair.cost, repair.cost, creative)?;
        let mut takes = vec![(Place::Ui(ANVIL_INPUT), input.count)];
        takes.extend((repair.material > 0).then_some((Place::Ui(ANVIL_MATERIAL), repair.material)));
        Ok(Self::new(takes, Forged::Repaired))
    }

    /// A grindstone's craft (`CraftGrindstoneRequest`), which names one of its inputs by stack id
    /// and the prior work the result carries as its cost.
    pub fn grind(inventory: &mut Inventory, named: i32, cost: i32) -> Result<Self, ()> {
        let [first, second] = [GRINDSTONE_INPUT, GRINDSTONE_ADDITIONAL].map(|slot| held(inventory, slot).cloned());
        if ![&first, &second].into_iter().flatten().any(|held| held.id == named) {
            return Err(());
        }
        let [first, second] = [first, second].map(|held| held.map(|held| held.stack));
        let from = first.clone().or_else(|| second.clone()).ok_or(())?;
        let ground = grind::grind(first.as_ref().map(stack), second.as_ref().map(stack)).filter(|ground| ground.work == cost).ok_or(())?;
        lay(inventory, &from, ground.made, 1)?;
        let takes = [(GRINDSTONE_INPUT, &first), (GRINDSTONE_ADDITIONAL, &second)].into_iter().filter(|(_, held)| held.is_some()).map(|(slot, _)| (Place::Ui(slot), 1)).collect();
        Ok(Self::new(takes, Forged::Ground(ground.experience)))
    }

    fn smith(inventory: &mut Inventory, id: u32) -> Result<Self, ()> {
        let [base, addition, template] = [SMITHING_INPUT, SMITHING_MATERIAL, SMITHING_TEMPLATE].map(|slot| held(inventory, slot).map(|held| held.stack.clone()));
        let (base, addition, template) = (base.ok_or(())?, addition.ok_or(())?, template.ok_or(())?);
        let made = sapling_data::smithed(id, &stack(&template), &stack(&base), &stack(&addition)).ok_or(())?;
        lay(inventory, &base, made, 1)?;
        let takes = [SMITHING_TEMPLATE, SMITHING_INPUT, SMITHING_MATERIAL].map(|slot| (Place::Ui(slot), 1)).to_vec();
        Ok(Self::new(takes, Forged::Smithed))
    }

    /// A trade of the offer a `CraftRecipe` names, paid from the payment slots (BDS's
    /// `CraftHandlerTrade::_handleTrade2`: a sold-out offer is refused).
    fn trade(inventory: &mut Inventory, bargains: &[Bargain], id: u32) -> Result<Self, ()> {
        let index = bargains.iter().position(|bargain| bargain.net_id == id).ok_or(())?;
        let bargain = &bargains[index];
        if bargain.sold_out {
            return Err(());
        }
        let mut takes = Vec::new();
        for (slot, want) in PAYMENT.into_iter().zip(&bargain.wants) {
            let Some(want) = want else { continue };
            let paid = held(inventory, slot).is_some_and(|held| held.stack.name == want.name && (want.damage == ANY_DAMAGE || held.stack.damage == want.damage) && held.stack.count >= want.count);
            if !paid {
                return Err(());
            }
            takes.push((Place::Ui(slot), want.count));
        }
        let gives = bargain.gives.clone();
        lay(inventory, &gives, Made { name: gives.name.clone(), tag: gives.tag.clone() }, gives.count)?;
        Ok(Self::new(takes, Forged::Traded(index)))
    }

    /// Counts `count` taken from `from` as used up.
    pub fn use_up(&mut self, from: Place, count: u8) {
        self.used.push((from, count));
    }

    /// Whether the request used up what the craft takes, from where it takes it, and nothing else.
    pub fn is_paid(&self) -> bool {
        let used = |place: Place| self.used.iter().filter(|(at, _)| *at == place).map(|(_, count)| u32::from(*count)).sum::<u32>();
        let takes_all = self.takes.iter().all(|(place, count)| used(*place) == u32::from(*count) || (self.spared == Some(*place) && used(*place) == 0));
        takes_all && self.used.iter().all(|(place, _)| self.takes.iter().any(|(taken, _)| taken == place))
    }
}
