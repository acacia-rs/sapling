//! `/ride`, answering as BDS did (research/captures/bds-ride.txt): riders and rides are named in
//! the answer, and a stop or an eviction that changed nothing says nothing.
// TODO: a summoned rider's or ride's name tag is taken and not given.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector, parse_choice};
use sapling_api::{Command, EntityId, Mount, Output, Permission, Sender, ShardCtx, Target};

use super::{display_name, no_target, targets};

#[derive(Clone, Copy, PartialEq)]
enum Fill {
    IfGroupFits,
    UntilFull,
}

#[derive(Clone, Copy, PartialEq)]
enum RideRule {
    NoRideChange,
    ReassignRides,
    SkipRiders,
}

enum Action {
    Start { ride: Selector, mount: Mount, fill: Fill },
    Stop,
    Evict,
    SummonRider { kind: String, event: Option<String> },
    SummonRide { kind: String, rule: RideRule, event: Option<String> },
}

pub(crate) struct Ride {
    target: Selector,
    action: Action,
}

fn param(name: &str, kind: ParamKind, optional: bool) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind, optional }
}

fn words(name: &str, values: &[&str], optional: bool) -> ParameterSpec {
    let kind = ParamKind::Enum { name: name.into(), values: values.iter().map(|v| (*v).to_owned()).collect() };
    param(&name.to_lowercase(), kind, optional)
}

const SUMMON_TAIL: [(&str, ParamKind); 2] = [("spawnEvent", ParamKind::String), ("nameTag", ParamKind::String)];

impl CommandSyntax for Ride {
    fn spec() -> CommandSpec {
        let target = |name: &str| param(name, ParamKind::Target, false);
        let kind = || param("entityType", ParamKind::String, false);
        let tail = || SUMMON_TAIL.map(|(name, kind)| param(name, kind, true));
        let overloads = vec![
            Overload {
                parameters: vec![
                    target("riders"),
                    words("StartRiding", &["start_riding"], false),
                    target("ride"),
                    words("TeleportRules", &["teleport_ride", "teleport_rider"], true),
                    words("FillType", &["if_group_fits", "until_full"], true),
                ],
            },
            Overload { parameters: vec![target("riders"), words("StopRiding", &["stop_riding"], false)] },
            Overload { parameters: vec![target("rides"), words("EvictRiders", &["evict_riders"], false)] },
            Overload { parameters: [target("rides"), words("SummonRider", &["summon_rider"], false), kind()].into_iter().chain(tail()).collect() },
            Overload {
                parameters: [target("riders"), words("SummonRide", &["summon_ride"], false), kind(), words("RideRules", &["no_ride_change", "reassign_rides", "skip_riders"], true)]
                    .into_iter()
                    .chain(tail())
                    .collect(),
            },
        ];
        CommandSpec { name: "ride".into(), aliases: Vec::new(), description: "Makes entities ride other entities, stops entities from riding, makes rides evict their riders, or summons rides or riders.".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let target = Selector::parse(args)?;
        let verbs = [("start_riding", 0), ("stop_riding", 1), ("evict_riders", 2), ("summon_rider", 3), ("summon_ride", 4)];
        let optional = |args: &mut Args<'_>| if args.is_empty() { Ok(None) } else { String::parse(args).map(Some) };
        let action = match parse_choice(args, &verbs)? {
            0 => {
                let ride = Selector::parse(args)?;
                let mount = if args.is_empty() { Mount::RiderToVehicle } else { parse_choice(args, &[("teleport_rider", Mount::RiderToVehicle), ("teleport_ride", Mount::VehicleToRider)])? };
                let fill = if args.is_empty() { Fill::UntilFull } else { parse_choice(args, &[("if_group_fits", Fill::IfGroupFits), ("until_full", Fill::UntilFull)])? };
                Action::Start { ride, mount, fill }
            }
            1 => Action::Stop,
            2 => Action::Evict,
            3 => Action::SummonRider { kind: String::parse(args)?, event: optional(args)? },
            _ => {
                let kind = String::parse(args)?;
                let rules = [("no_ride_change", RideRule::NoRideChange), ("reassign_rides", RideRule::ReassignRides), ("skip_riders", RideRule::SkipRiders)];
                let rule = if args.peek().is_some_and(|word| rules.iter().any(|(name, _)| name.eq_ignore_ascii_case(word))) { parse_choice(args, &rules)? } else { RideRule::ReassignRides };
                Action::SummonRide { kind, rule, event: optional(args)? }
            }
        };
        // The name tag.
        let _ = optional(args)?;
        args.finish()?;
        Ok(Self { target, action })
    }
}

fn namespaced(kind: &str) -> String {
    if kind.contains(':') { kind.to_owned() } else { format!("minecraft:{kind}") }
}

/// Everything `selector` names, as riders: each with its name and where it is.
fn found(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: &Selector) -> Vec<(Target, String, [f32; 3], f32)> {
    let (players, entities) = targets(ctx, sender, Some(selector));
    let mut all: Vec<_> = players.into_iter().filter_map(|player| Some((ctx.position(player.id)?, player))).map(|(at, player)| (Target::Player(player.clone()), player.name, at.feet, at.yaw)).collect();
    all.extend(entities.iter().map(|entity| (Target::Entity(entity.id), display_name(entity), entity.feet, entity.yaw)));
    all
}

fn names(named: &[String]) -> Vec<String> {
    vec![named.join(", ")]
}

impl Command for Ride {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = found(ctx, sender, &self.target);
        if found.is_empty() {
            return no_target(out);
        }
        match self.action {
            Action::Start { ride, mount, fill } => start(ctx, sender, out, &found, &ride, mount, fill),
            Action::Stop => {
                let stopped: Vec<String> = found.into_iter().filter(|(rider, ..)| ctx.dismount(rider)).map(|(_, name, ..)| name).collect();
                if !stopped.is_empty() {
                    out.translate("commands.ride.stopRiding.success", names(&stopped), true);
                }
            }
            Action::Evict => {
                let evicted: Vec<String> = found.into_iter().filter(|(ride, ..)| matches!(ride, Target::Entity(id) if ctx.evict_riders(*id) > 0)).map(|(_, name, ..)| name).collect();
                if !evicted.is_empty() {
                    out.translate("commands.ride.evictRiders.success", names(&evicted), true);
                }
            }
            Action::SummonRider { kind, event } => summon_riders(ctx, out, &found, &namespaced(&kind), event.as_deref()),
            Action::SummonRide { kind, rule, event } => summon_rides(ctx, out, &found, &namespaced(&kind), rule, event.as_deref()),
        }
    }
}

/// The checks and answers of BDS's `start_riding` (docs/research/vehicles.md "/ride").
fn start(ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, riders: &[(Target, String, [f32; 3], f32)], ride: &Selector, mount: Mount, fill: Fill) {
    let rides = found(ctx, sender, ride);
    let [(vehicle, ..)] = rides.as_slice() else {
        return if rides.is_empty() { no_target(out) } else { out.translate("commands.ride.startRiding.tooManyRides", Vec::new(), false) };
    };
    let all_names: Vec<String> = riders.iter().map(|(_, name, ..)| name.clone()).collect();
    let Target::Entity(vehicle) = *vehicle else { return out.translate("commands.ride.startRiding.notRideable", Vec::new(), false) };
    if mount == Mount::VehicleToRider && riders.len() > 1 {
        return out.translate("commands.ride.startRiding.tooManyRiders", Vec::new(), false);
    }
    let (left, on) = (ctx.seats_left(vehicle), ctx.riders(vehicle));
    match () {
        _ if left == 0 && on.is_empty() => return out.translate("commands.ride.startRiding.notRideable", Vec::new(), false),
        _ if left == 0 => return out.translate("commands.ride.startRiding.rideFull", Vec::new(), false),
        _ if fill == Fill::IfGroupFits && left < riders.iter().filter(|(rider, ..)| !on.contains(rider)).count() => {
            return out.translate("commands.ride.startRiding.groupDoesntFit", Vec::new(), false);
        }
        _ => {}
    }
    let (mut seated, mut refused, mut left_over) = (Vec::new(), Vec::new(), false);
    for (rider, name, ..) in riders.iter().filter(|(rider, ..)| *rider != Target::Entity(vehicle) && !on.contains(rider)) {
        if ctx.seats_left(vehicle) == 0 {
            left_over = true;
            break;
        }
        if ctx.ride(rider, vehicle, mount) { seated.push(name.clone()) } else { refused.push(name.clone()) }
    }
    if !seated.is_empty() {
        out.translate("commands.ride.startRiding.success", names(&seated), true);
    }
    if !refused.is_empty() {
        out.translate("commands.ride.startRiding.failure", names(&refused), false);
    } else if left_over {
        out.translate("commands.ride.startRiding.rideFullRidersLeft", Vec::new(), false);
    } else if seated.is_empty() {
        out.translate("commands.ride.startRiding.failure", names(&all_names), false);
    }
}

/// A new rider of `kind` for each ride, which is taken away again where the ride refuses it.
fn summon_riders(ctx: &mut ShardCtx<'_>, out: &mut Output, rides: &[(Target, String, [f32; 3], f32)], kind: &str, event: Option<&str>) {
    let (mut done, mut failed) = (Vec::new(), Vec::new());
    for (ride, name, at, yaw) in rides {
        let Target::Entity(vehicle) = ride else {
            failed.push(name.clone());
            continue;
        };
        let rider = (ctx.seats_left(*vehicle) > 0).then(|| ctx.summon_as(None, kind, *at, *yaw, event)).flatten();
        match rider {
            Some(rider) if ctx.ride(&Target::Entity(rider), *vehicle, Mount::RiderToVehicle) => done.push(name.clone()),
            Some(rider) => {
                ctx.remove_entity(rider);
                failed.push(name.clone());
            }
            None => failed.push(name.clone()),
        }
    }
    match done.is_empty() {
        true => out.translate("commands.ride.summonRider.failed", names(&failed), false),
        false => out.translate("commands.ride.summonRider.success", names(&done), true),
    }
}

/// A new ride of `kind` under each rider; one that already rides is skipped, or taken off first
/// with `reassign_rides`.
fn summon_rides(ctx: &mut ShardCtx<'_>, out: &mut Output, riders: &[(Target, String, [f32; 3], f32)], kind: &str, rule: RideRule, event: Option<&str>) {
    let (mut done, mut skipped, mut failed) = (Vec::new(), Vec::new(), Vec::new());
    for (rider, name, at, yaw) in riders {
        if ctx.vehicle_of(rider).is_some() && rule != RideRule::ReassignRides {
            skipped.push(name.clone());
            continue;
        }
        let Some(vehicle): Option<EntityId> = ctx.summon_as(None, kind, *at, *yaw, event) else {
            failed.push(name.clone());
            continue;
        };
        if ctx.ride(rider, vehicle, Mount::RiderToVehicle) {
            done.push(name.clone());
        } else {
            ctx.remove_entity(vehicle);
            failed.push(name.clone());
        }
    }
    if !done.is_empty() {
        out.translate("commands.ride.summonRide.success", names(&done), true);
    } else if !skipped.is_empty() {
        out.translate("commands.ride.summonRide.skipped", names(&skipped), true);
    } else {
        out.translate("commands.ride.summonRide.failed", names(&failed), false);
    }
}
