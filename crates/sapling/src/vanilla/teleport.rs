//! `/tp [victim] <destination> [yaw pitch | facing <place>]`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::{display_name, no_target, targets, tell_target};

/// `/tp [victim] <destination>`: the first word is a target unless it is a coordinate, and the
/// destination a place or whoever a target names (research/captures/bds-golem.txt). A place may
/// be followed by where to look: yaw, then pitch, or `facing` a place.
#[derive(Clone)]
pub(crate) struct Teleport {
    target: Option<Selector>,
    to: Destination,
    looks: Option<Looks>,
}

#[derive(Clone)]
enum Destination {
    Place(Coords),
    At(Selector),
}

#[derive(Clone)]
enum Looks {
    /// Yaw and pitch, each `true` when counted from the sender's (`~`).
    Along([(f32, bool); 2]),
    Facing(Coords),
}

/// The sender's yaw and pitch, which `~` turns count from (bds-g8rot.txt: a stand sent to `~-60` by
/// an actor facing 0 was told -59.06).
fn rotation(ctx: &ShardCtx<'_>, sender: &Sender) -> [f32; 2] {
    match sender {
        Sender::Executed(executed) => executed.rotation,
        _ => sender.player().and_then(|player| ctx.position(player.id)).map_or([0.0; 2], |position| [position.yaw, position.pitch]),
    }
}

/// The yaw and pitch of a player at `feet` whose eyes look at `point`.
fn facing(feet: [f32; 3], point: [f32; 3]) -> [f32; 2] {
    let [dx, dy, dz] = [point[0] - feet[0], point[1] - feet[1] - sapling_data::EYE_HEIGHT, point[2] - feet[2]];
    [(-dx).atan2(dz).to_degrees(), (-dy).atan2(dx.hypot(dz)).to_degrees()]
}

impl CommandSyntax for Teleport {
    fn spec() -> CommandSpec {
        let destination = ParameterSpec::of::<Coords>("destination");
        CommandSpec {
            name: "tp".into(),
            aliases: vec!["teleport".into()],
            description: "Teleports entities".into(),
            permission: Permission::Operator,
            overloads: vec![
                Overload { parameters: vec![destination.clone()] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("victim"), destination] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("destination")] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("victim"), ParameterSpec::of::<Selector>("destination")] },
            ],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let is_coordinate = |word: &str| word.starts_with(['~', '^']) || word.parse::<f32>().is_ok();
        let first = match args.peek() {
            Some(word) if !is_coordinate(word) => Some(Selector::parse(args)?),
            _ => None,
        };
        Ok(match (first, args.peek().map(is_coordinate)) {
            (Some(to), None) => Self { target: None, to: Destination::At(to), looks: None },
            (target, Some(false)) => Self { target, to: Destination::At(Selector::parse(args)?), looks: None },
            (target, _) => {
                let to = Destination::Place(Coords::parse(args)?);
                if args.peek().is_some_and(|word| word.eq_ignore_ascii_case("facing")) {
                    args.word()?;
                    return Ok(Self { target, to, looks: Some(Looks::Facing(Coords::parse(args)?)) });
                }
                let angle = |args: &mut Args<'_>| -> Result<Option<(f32, bool)>, ParseError> {
                    let mark = args.mark();
                    if args.is_empty() {
                        return Ok(None);
                    }
                    let word = args.word()?.to_owned();
                    let (number, relative) = word.strip_prefix('~').map_or((word.as_str(), false), |rest| (rest, true));
                    match (number, relative) {
                        ("", true) => Ok(Some((0.0, true))),
                        _ => number.parse().map(|angle| Some((angle, relative))).map_err(|_| args.error_at(mark)),
                    }
                };
                let looks = match (angle(args)?, angle(args)?) {
                    (Some(yaw), pitch) => Some(Looks::Along([yaw, pitch.unwrap_or((0.0, false))])),
                    _ => None,
                };
                Self { target, to, looks }
            }
        })
    }
}

impl Command for Teleport {
    /// Each player moved is told where to, as BDS tells them; into the dimension an `/execute in`
    /// names, the way BDS changes a player's dimension, and a mob's. `~`, `^` and the nearest are counted from
    /// the sender, or where an `/execute` runs it.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        let dimension = match sender {
            Sender::Executed(executed) => Some(executed.dimension),
            _ => None,
        };
        let (to, at) = match &self.to {
            Destination::Place(coords) => (coords.resolve(ctx, sender), None),
            // The nearest of those it names.
            Destination::At(selector) => {
                let (people, others) = targets(ctx, sender, Some(selector));
                let named = people.into_iter().filter_map(|player| Some((ctx.position(player.id)?.feet, player.name)));
                let all = named.chain(others.into_iter().map(|entity| (entity.feet, display_name(&entity))));
                let far = |feet: &[f32; 3]| (0..3).map(|axis| (feet[axis] - here[axis]).powi(2)).sum::<f32>();
                match all.min_by(|a, b| far(&a.0).total_cmp(&far(&b.0))) {
                    Some((feet, name)) => (feet, Some(name)),
                    None => return no_target(out),
                }
            }
        };
        let looks = match &self.looks {
            Some(Looks::Along(looks)) => {
                let from = rotation(ctx, sender);
                Some(std::array::from_fn(|side| looks[side].0 + if looks[side].1 { from[side] } else { 0.0 }))
            }
            Some(Looks::Facing(point)) => Some(facing(to, point.resolve(ctx, sender))),
            None => None,
        };
        let (moved_players, moved) = targets(ctx, sender, Some(self.target.as_ref().unwrap_or(&Selector::sender())));
        let [x, y, z] = to.map(|c| format!("{c:.2}"));
        for player in &moved_players {
            match (dimension.filter(|dimension| ctx.dimension_of(player.id) != Some(*dimension)), looks) {
                (Some(dimension), _) => drop(ctx.travel(player.id, dimension, to)),
                (None, Some(looks)) => ctx.teleport_facing(player.id, to, looks),
                (None, None) => ctx.teleport(player.id, to),
            }
            tell_target(ctx, player.id, "commands.tp.successVictim", &[at.clone().unwrap_or_else(|| format!("{x}, {y}, {z}"))]);
        }
        for entity in &moved {
            if let Some([yaw, _]) = looks {
                ctx.turn_entity(entity.id, yaw);
            }
            if !dimension.is_some_and(|dimension| ctx.move_entity_into(entity.id, dimension, to)) {
                ctx.move_entity(entity.id, to);
            }
        }
        let names: Vec<String> = moved_players.iter().map(|player| player.name.clone()).chain(moved.iter().map(|entity| display_name(&entity))).collect();
        // BDS counts each one moved, and the one gone to as one more (bds-s3misc.txt).
        out.done_to((names.len() + usize::from(at.is_some())) as u32);
        match (names.is_empty(), at) {
            (true, _) => no_target(out),
            (false, Some(at)) => out.translate("commands.tp.success", vec![names.join(", "), at], true),
            (false, None) => out.translate("commands.tp.success.coordinates", vec![names.join(", "), x, y, z], true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tp_takes_a_place_or_a_target_to_go_to_with_or_without_a_victim() {
        let sender = Sender::Console;
        let parsed = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).map(|tp| (tp.target, matches!(tp.to, Destination::At(Selector::Variable(sapling_api::command::Base::Entities, _)))));
        assert_eq!(parsed("~ ~8 ~").ok(), Some((None, false)));
        assert_eq!(parsed("@s 1 2 3").ok(), Some((Some(Selector::sender()), false)));
        assert_eq!(parsed("@s @e[type=iron_golem,c=1]").ok(), Some((Some(Selector::sender()), true)));
        assert_eq!(parsed("@e[type=iron_golem]").ok(), Some((None, true)));
        assert!(parsed("@s").is_ok(), "alone, a target is where to");
        let looks = |line: &str| match Teleport::parse(&mut Args::new(line, 0, &sender, &[])).ok().and_then(|tp| tp.looks) {
            Some(Looks::Along(looks)) => Some(Some(looks)),
            Some(Looks::Facing(_)) => Some(None),
            None => None,
        };
        let fixed = |yaw, pitch| Some(Some([(yaw, false), (pitch, false)]));
        assert_eq!([looks("~ ~ ~4 -90 10"), looks("@s ~ ~ ~4 45"), looks("~ ~ ~4"), looks("@s ~ ~ ~1.5 facing ~ ~1 ~")], [fixed(-90.0, 10.0), fixed(45.0, 0.0), None, Some(None)]);
        assert_eq!(looks("@s 1 2 3 ~-60 ~"), Some(Some([(-60.0, true), (0.0, true)])), "a turn counted from the sender's");
    }

    #[test]
    fn facing_a_place_turns_the_eyes_to_it() {
        let [yaw, pitch] = facing([0.0, 64.0, 0.0], [0.0, 64.0 + sapling_data::EYE_HEIGHT, 5.0]);
        assert_eq!((yaw.round(), pitch.round()), (0.0, 0.0));
        let [yaw, pitch] = facing([0.0, 64.0, 0.0], [-3.0, 64.0 + sapling_data::EYE_HEIGHT + 3.0, 0.0]);
        assert_eq!((yaw.round(), pitch.round()), (90.0, -45.0));
    }
}
