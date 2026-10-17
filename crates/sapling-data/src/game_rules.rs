//! Game rules: every rule BDS has, under its own name (`doDayLightCycle`) and in its order, with
//! the values of the StartGame it was recorded sending.

use std::fmt;
use std::sync::LazyLock;

use acacia_proto::packets::GameRulesChanged;
use acacia_proto::types::{GameRuleI32, GameRuleI32Value};

use crate::start_game::TEMPLATE;

pub const DAYLIGHT_CYCLE: &str = "doDayLightCycle";

/// A game rule's value: a switch or a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleValue {
    Bool(bool),
    Int(i32),
}

impl RuleValue {
    fn of(wire: &GameRuleI32Value) -> Option<Self> {
        match *wire {
            GameRuleI32Value::Bool(on) => Some(Self::Bool(on)),
            GameRuleI32Value::Int(n) => Some(Self::Int(n)),
            _ => None,
        }
    }

    fn wire(self) -> GameRuleI32Value {
        match self {
            Self::Bool(on) => GameRuleI32Value::Bool(on),
            Self::Int(n) => GameRuleI32Value::Int(n),
        }
    }

    fn same_kind(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// As `/gamerule` shows it: `true`, `5`.
impl fmt::Display for RuleValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(on) => write!(f, "{on}"),
            Self::Int(n) => write!(f, "{n}"),
        }
    }
}

static DEFAULTS: LazyLock<Vec<GameRuleI32>> = LazyLock::new(|| TEMPLATE.gamerules.iter().filter(|rule| RuleValue::of(&rule.value).is_some()).cloned().collect());

/// A world's game rules, BDS's when nothing set them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRules(Vec<RuleValue>);

impl Default for GameRules {
    fn default() -> Self {
        let mut rules = Self(DEFAULTS.iter().filter_map(|rule| RuleValue::of(&rule.value)).collect());
        // The recording stopped the clock; a world runs it unless it says otherwise.
        rules.set(DAYLIGHT_CYCLE, RuleValue::Bool(true));
        rules
    }
}

fn index(name: &str) -> Option<usize> {
    DEFAULTS.iter().position(|rule| rule.name.eq_ignore_ascii_case(name))
}

impl GameRules {
    /// Every rule's own name and value, in BDS's order.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, RuleValue)> + '_ {
        DEFAULTS.iter().map(|rule| rule.name.as_str()).zip(self.0.iter().copied())
    }

    /// The rule `name`, in any case: its own name and value.
    pub fn get(&self, name: &str) -> Option<(&'static str, RuleValue)> {
        index(name).map(|at| (DEFAULTS[at].name.as_str(), self.0[at]))
    }

    /// Whether the switch `name` is on; false for a number or no such rule.
    pub fn on(&self, name: &str) -> bool {
        self.get(name).is_some_and(|(_, value)| value == RuleValue::Bool(true))
    }

    /// Sets the rule `name`, in any case: whether that changed it. `None`, with nothing changed,
    /// for no such rule or a value of the other kind.
    pub fn set(&mut self, name: &str, value: RuleValue) -> Option<bool> {
        let at = index(name).filter(|&at| self.0[at].same_kind(value))?;
        Some(std::mem::replace(&mut self.0[at], value) != value)
    }

    /// All of them, as BDS sends them after StartGame.
    pub fn packet(&self) -> GameRulesChanged {
        GameRulesChanged { rules: self.start_game() }
    }

    /// The rule `name` alone, as BDS tells a world's players of a change; `None` for no such rule.
    pub fn changed(&self, name: &str) -> Option<GameRulesChanged> {
        let at = index(name)?;
        Some(GameRulesChanged { rules: vec![GameRuleI32 { value: self.0[at].wire(), ..DEFAULTS[at].clone() }] })
    }

    /// StartGame's list: the recorded one, with this world's values.
    pub(crate) fn start_game(&self) -> Vec<GameRuleI32> {
        let mut rules = TEMPLATE.gamerules.clone();
        for rule in &mut rules {
            if let Some((_, value)) = self.get(&rule.name) {
                rule.value = value.wire();
            }
        }
        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What BDS's `/gamerule` listed, in its order (research/captures/bds-gamerule.txt).
    const LISTED: &str = "commandBlockOutput doDayLightCycle doEntityDrops doFireTick recipesUnlock doLimitedCrafting doMobLoot doMobSpawning doTileDrops doWeatherCycle drowningDamage fallDamage fireDamage keepInventory mobGriefing pvp showCoordinates playerWaypoints locatorbar showDaysPlayed naturalRegeneration tntExplodes sendCommandFeedback maxCommandChainLength doInsomnia commandBlocksEnabled randomTickSpeed doImmediateRespawn showDeathMessages functionCommandLimit spawnRadius showTags freezeDamage respawnBlocksExplode showBorderEffect showRecipeMessages playersSleepingPercentage projectilesCanBreakBlocks tntExplosionDropDecay";

    #[test]
    fn the_recorded_rules_are_the_ones_bds_lists() {
        let names: Vec<&str> = GameRules::default().iter().map(|(name, _)| name).collect();
        assert_eq!(names.join(" "), LISTED);
    }

    #[test]
    fn a_rule_is_found_in_any_case_and_keeps_its_kind() {
        let mut rules = GameRules::default();
        assert_eq!(rules.get("DODAYLIGHTCYCLE"), Some((DAYLIGHT_CYCLE, RuleValue::Bool(true))));
        assert_eq!(rules.set("dodaylightcycle", RuleValue::Int(3)), None);
        assert_eq!((rules.set("dodaylightcycle", RuleValue::Bool(false)), rules.set("dodaylightcycle", RuleValue::Bool(false))), (Some(true), Some(false)));
        assert!(!rules.on(DAYLIGHT_CYCLE) && rules.set("nosuchrule", RuleValue::Bool(true)).is_none());
        let changed = rules.changed(DAYLIGHT_CYCLE).unwrap().rules;
        assert_eq!((changed.len(), changed[0].name.as_str(), &changed[0].value), (1, DAYLIGHT_CYCLE, &GameRuleI32Value::Bool(false)));
        let all = rules.packet().rules;
        assert!(all.iter().any(|rule| rule.name == DAYLIGHT_CYCLE && rule.value == GameRuleI32Value::Bool(false)), "{all:?}");
    }
}
