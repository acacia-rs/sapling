//! How the dragon flies (research/dragon-ai-decompile.md): BDS's goals by their groups and
//! priorities, the flying ones (takeoff, charge, strafe, holding pattern), the sitting ones
//! (landing, flaming, scanning) and its last flight; each tick its stored motion carries it, its
//! goal picks where to go, and aiStep takes it there (motion.rs). Draws come from the fight's
//! random where BDS draws on the level's.

mod flying;
mod goals;
mod motion;
mod nodes;
mod sitting;
mod trail;

pub(crate) use goals::{Ctx, begin_dying, take_off, think};
pub(crate) use motion::Motion;
pub(crate) use motion::wrap;
use nodes::Node;

/// The goals' groups, which the dragon's events switch between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Flying,
    Sitting,
    Dying,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Goal {
    Takeoff,
    Charge,
    Strafe,
    Holding,
    Landing,
    Flaming,
    Scanning,
    Death,
}

impl Goal {
    /// The group's goals, first first.
    fn of(group: Group) -> &'static [Goal] {
        match group {
            Group::Flying => &[Goal::Takeoff, Goal::Charge, Goal::Strafe, Goal::Holding],
            Group::Sitting => &[Goal::Landing, Goal::Flaming, Goal::Scanning],
            Group::Dying => &[Goal::Death],
        }
    }
}

/// The dragon's flight: its body in the air, its goal and what its goals keep.
#[derive(Clone, Debug)]
pub(crate) struct Flight {
    pub motion: Motion,
    /// Its pitch, which eases to level by ten degrees a tick.
    pub pitch: f32,
    pub sitting: bool,
    pub group: Group,
    pub goal: Option<Goal>,
    nodes: Vec<Node>,
    nodes_final: bool,
    path: Option<(Vec<Node>, usize)>,
    /// The player it means to strafe, charge or flame.
    pub prey: Option<bevy_ecs::entity::Entity>,
    clockwise: bool,
    /// Ticks until it growls in flight.
    growl: i32,
    /// The goals' own timers: the landing's last leg, scanning's time, flaming's roar and breaths,
    /// takeoff's first tick, strafing's aim, charging's count.
    landing: bool,
    timer: f32,
    roar: u32,
    breath: u32,
    flames: u32,
    first: bool,
    aim: u32,
    fired: bool,
    count: u32,
    /// Damage taken since it sat down.
    pub sat_harm: f32,
    pub(crate) healer: Option<bevy_ecs::entity::Entity>,
    pub trail: trail::Trail,
}

const PITCH_EASE: f32 = 10.0;
const GROWL_FROM: i32 = 200;
const GROWL_SPREAD: u32 = 200;
/// A new dragon's first growl is 100 ticks on (bds-dragon-watch.txt, bds-dragon-again.txt).
const FIRST_GROWL: i32 = 100;

/// Whether the 8 x 8 columns round the middle its nodes stand on are loaded (`areChunksFullyLoaded`
/// within 60, research/dragon-extras-decompile.md 5.5); those that are not are asked for, standing in
/// for BDS's wider player views, which had them all loaded.
fn nodes_loaded(level: &mut sapling_world::World) -> bool {
    let reach = nodes::BUILT_WITHIN.div_euclid(16) + 1;
    let columns = (-reach..reach).flat_map(|x| (-reach..reach).map(move |z| sapling_world::ChunkPos::new(x, z)));
    columns.fold(true, |all, column| level.request(column) && all)
}

impl Flight {
    pub fn new(at: [f32; 3], pitch: f32) -> Self {
        Self {
            motion: Motion::new(at),
            pitch,
            sitting: false,
            group: Group::Flying,
            goal: None,
            nodes: Vec::new(),
            nodes_final: false,
            path: None,
            prey: None,
            clockwise: false,
            growl: FIRST_GROWL,
            landing: false,
            timer: 0.0,
            roar: 0,
            breath: 0,
            flames: 0,
            first: false,
            aim: 0,
            fired: false,
            count: 0,
            sat_harm: 0.0,
            healer: None,
            trail: trail::Trail::default(),
        }
    }

    /// BDS's node upkeep: built at the first call however little is loaded, built again the first
    /// time all its columns are, then kept.
    pub fn keep_nodes(&mut self, level: &mut sapling_world::World) {
        if self.nodes_final {
            return;
        }
        let loaded = nodes_loaded(level);
        if loaded || self.nodes.is_empty() {
            (self.nodes, self.nodes_final) = (nodes::build(level), loaded);
        }
    }

    /// Whether it growls this tick, in flight every 200 to 399 ticks.
    pub fn growls(&mut self, random: &mut crate::dragon::random::Random) -> bool {
        self.growl -= 1;
        if self.growl >= 0 {
            return false;
        }
        self.growl = GROWL_FROM + random.next_int(GROWL_SPREAD) as i32;
        true
    }

    /// Eases its pitch a tick toward level.
    pub fn level_out(&mut self) {
        let pitch = wrap(self.pitch);
        self.pitch = if pitch.abs() <= PITCH_EASE { 0.0 } else { pitch - PITCH_EASE * pitch.signum() };
    }

    /// Sits down or gets up; either stops it where it is (`setSitting` 145998920).
    pub fn sit(&mut self, sitting: bool) {
        self.sitting = sitting;
        self.motion.delta = [0.0; 3];
    }

    /// The next node of its path becomes its target, up to twenty over it; false at the path's end.
    fn next_node(&mut self, random: &mut super::random::Random) -> bool {
        let Some((path, index)) = &mut self.path else { return false };
        let Some(&[x, y, z]) = path.get(*index) else { return false };
        *index += 1;
        self.motion.target = Some([x as f32, y as f32 + random.next_float() * 20.0, z as f32]);
        true
    }

    fn path_done(&self) -> bool {
        self.path.as_ref().is_some_and(|(path, index)| *index >= path.len())
    }

    /// Its target is near enough and not too near to keep (the holding pattern's and landing's test).
    fn keeps_target(&self) -> bool {
        self.motion.target.is_some_and(|t| (100.0..=22500.0).contains(&(0..3).map(|axis| (t[axis] - self.motion.at[axis]).powi(2)).sum::<f32>()))
    }
}
