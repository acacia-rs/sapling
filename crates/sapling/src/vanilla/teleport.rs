//! `/tp [victim] <destination> [yaw pitch | facing <place>]`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::{display_name, entities, no_target, origin, players_near, tell_target};

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
    Along([f32; 2]),
    Facing(Coords),
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
        let is_coordinate = |word: &str| word.starts_with('~') || word.parse::<f32>().is_ok();
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
                let angle = |args: &mut Args<'_>| -> Result<Option<f32>, ParseError> {
                    let mark = args.mark();
                    if args.is_empty() { Ok(None) } else { args.word()?.parse().map(Some).map_err(|_| args.error_at(mark)) }
                };
                let looks = match (angle(args)?, angle(args)?) {
                    (Some(yaw), pitch) => Some(Looks::Along([yaw, pitch.unwrap_or(0.0)])),
                    _ => None,
                };
                Self { target, to, looks }
            }
        })
    }
}

impl Command for Teleport {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = origin(ctx, sender);
        self.run_from(ctx, sender, out, None, here);
    }
}

impl Teleport {
    /// Each player moved is told where to, as BDS tells them; into `dimension` of its world when
    /// one is named (`/execute in`), the way BDS changes a player's dimension. `~` and the nearest
    /// are counted from `here`: the sender, or where an `/execute at` runs it.
    pub(crate) fn run_from(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, dimension: Option<i32>, here: [f32; 3]) {
        let (to, at) = match &self.to {
            Destination::Place(coords) => (coords.from(here), None),
            // The nearest of those it names.
            Destination::At(selector) => {
                let named = players_near(ctx, sender, selector, here).into_iter().filter_map(|player| Some((ctx.position(player.id)?.feet, player.name)));
                let all = named.chain(entities(ctx, selector).into_iter().map(|entity| (entity.feet, display_name(&entity))));
                let far = |feet: &[f32; 3]| (0..3).map(|axis| (feet[axis] - here[axis]).powi(2)).sum::<f32>();
                match all.min_by(|a, b| far(&a.0).total_cmp(&far(&b.0))) {
                    Some((feet, name)) => (feet, Some(name)),
                    None => return no_target(out),
                }
            }
        };
        let looks = match &self.looks {
            Some(Looks::Along(looks)) => Some(*looks),
            Some(Looks::Facing(point)) => Some(facing(to, point.from(here))),
            None => None,
        };
        let moved_players = players_near(ctx, sender, self.target.as_ref().unwrap_or(&Selector::Sender), here);
        let moved = self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target));
        let [x, y, z] = to.map(|c| format!("{c:.2}"));
        for player in &moved_players {
            match (dimension, looks) {
                (Some(dimension), _) => drop(ctx.travel(player.id, dimension, to)),
                (None, Some(looks)) => ctx.teleport_facing(player.id, to, looks),
                (None, None) => ctx.teleport(player.id, to),
            }
            tell_target(ctx, player.id, "commands.tp.successVictim", &[at.clone().unwrap_or_else(|| format!("{x}, {y}, {z}"))]);
        }
        for entity in &moved {
            ctx.move_entity(entity.id, to);
        }
        let names: Vec<String> = moved_players.iter().map(|player| player.name.clone()).chain(moved.iter().map(|entity| display_name(&entity))).collect();
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
        let parsed = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).map(|tp| (tp.target, matches!(tp.to, Destination::At(Selector::Entities { .. }))));
        assert_eq!(parsed("~ ~8 ~").ok(), Some((None, false)));
        assert_eq!(parsed("@s 1 2 3").ok(), Some((Some(Selector::Sender), false)));
        assert_eq!(parsed("@s @e[type=iron_golem,c=1]").ok(), Some((Some(Selector::Sender), true)));
        assert_eq!(parsed("@e[type=iron_golem]").ok(), Some((None, true)));
        assert!(parsed("@s").is_ok(), "alone, a target is where to");
        let looks = |line: &str| match Teleport::parse(&mut Args::new(line, 0, &sender, &[])).ok().and_then(|tp| tp.looks) {
            Some(Looks::Along(looks)) => Some(Some(looks)),
            Some(Looks::Facing(_)) => Some(None),
            None => None,
        };
        assert_eq!([looks("~ ~ ~4 -90 10"), looks("@s ~ ~ ~4 45"), looks("~ ~ ~4"), looks("@s ~ ~ ~1.5 facing ~ ~1 ~")], [Some(Some([-90.0, 10.0])), Some(Some([45.0, 0.0])), None, Some(None)]);
    }

    #[test]
    fn facing_a_place_turns_the_eyes_to_it() {
        let [yaw, pitch] = facing([0.0, 64.0, 0.0], [0.0, 64.0 + sapling_data::EYE_HEIGHT, 5.0]);
        assert_eq!((yaw.round(), pitch.round()), (0.0, 0.0));
        let [yaw, pitch] = facing([0.0, 64.0, 0.0], [-3.0, 64.0 + sapling_data::EYE_HEIGHT + 3.0, 0.0]);
        assert_eq!((yaw.round(), pitch.round()), (90.0, -45.0));
    }
}
