use crate::command::{Args, CommandSyntax, ParamKind, ParameterSpec, ParseError, Target, Text};
use crate::testing::{Fake, player};
use crate::{Command, CommandEnum, Commands, GameMode, Output, Permission, Sender, ShardCtx};

#[derive(CommandEnum, Debug, Clone, Copy, PartialEq)]
#[command(crate = "crate")]
enum Colour {
    Red,
    #[command(name = "navy")]
    DarkBlue,
}

#[derive(Command, Debug, PartialEq)]
#[command(crate = "crate", name = "give", aliases = ["g"], description = "Gives things", permission = "operator")]
struct Give {
    amount: i32,
    ratio: f32,
    loud: bool,
    item: String,
    mode: GameMode,
    colour: Colour,
    to: Target,
    #[command(name = "note")]
    reason: Option<Text>,
}

#[derive(Command, Debug, PartialEq)]
#[command(crate = "crate")]
enum Warp {
    List,
    Set { name: String, radius: Option<u8> },
    #[command(name = "del")]
    Delete { name: String },
}

#[derive(Command, Debug, PartialEq)]
#[command(crate = "crate")]
struct Ping;

macro_rules! echo {
    ($($command:ty),*) => {$(
        impl Command for $command {
            fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
                out.print(format!("{} at tick {}: {self:?}", sender.name(), ctx.tick()));
            }
        }
    )*};
}
echo!(Give, Warp, Ping);

fn parse<C: CommandSyntax>(line: &str, sender: &Sender) -> Result<C, ParseError> {
    let players = [player(1, "Alex"), player(2, "Bob Ross")];
    let mut args = Args::new(line, line.find(' ').unwrap_or(line.len()), sender, &players);
    let parsed = C::parse(&mut args)?;
    args.finish().map(|()| parsed)
}

fn alex() -> Sender {
    Sender::Player(player(1, "Alex"))
}

fn syntax(before: &str, token: &str, after: &str) -> ParseError {
    ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() }
}

#[test]
fn every_parameter_kind_parses() {
    let give: Give = parse(r#"give -5 0.5 true "gold ingot" Creative navy "bob ross" for  being nice "#, &alex()).unwrap();
    let expected = Give {
        amount: -5,
        ratio: 0.5,
        loud: true,
        item: "gold ingot".into(),
        mode: GameMode::Creative,
        colour: Colour::DarkBlue,
        to: Target(player(2, "Bob Ross")),
        reason: Some(Text("for  being nice".into())),
    };
    assert_eq!(give, expected);
    let own: Give = parse("give 1 2 false x survival red @s", &alex()).unwrap();
    assert_eq!((own.to, own.reason, own.colour, own.ratio), (Target(player(1, "Alex")), None, Colour::Red, 2.0));
    assert_eq!(parse::<Ping>("ping", &alex()), Ok(Ping));
}

#[test]
fn sub_commands_pick_a_variant_by_its_word() {
    assert_eq!(parse::<Warp>("warp list", &alex()), Ok(Warp::List));
    assert_eq!(parse::<Warp>("warp SET home", &alex()), Ok(Warp::Set { name: "home".into(), radius: None }));
    assert_eq!(parse::<Warp>("warp set home 9", &alex()), Ok(Warp::Set { name: "home".into(), radius: Some(9) }));
    assert_eq!(parse::<Warp>("warp del home", &alex()), Ok(Warp::Delete { name: "home".into() }));
    assert_eq!(parse::<Warp>("warp delete home", &alex()), Err(syntax("/warp ", "delete", " home")));
    assert_eq!(parse::<Warp>("warp", &alex()), Err(syntax("/warp", "", "")));
}

#[test]
fn bad_input_is_reported_where_it_stands() {
    // The shapes BDS gives `/gamemode banana` and a bare `/gamemode`.
    assert_eq!(parse::<Give>("give lots", &alex()), Err(syntax("/give ", "lots", "")));
    assert_eq!(parse::<Give>("give", &alex()), Err(syntax("/give", "", "")));
    assert_eq!(parse::<Give>("give 1 nan true x survival red @s", &alex()), Err(syntax("/give 1 ", "nan", " true x su")));
    assert_eq!(parse::<Give>("give 1 2 yes x survival red @s", &alex()), Err(syntax("/give 1 2 ", "yes", " x surviva")));
    // BDS keeps the last ten characters before the token and the first ten after it.
    assert_eq!(parse::<Give>("give 1 2 true x hardcore red @s", &alex()), Err(syntax(" 2 true x ", "hardcore", " red @s")));
    assert_eq!(parse::<Give>("give 1 2 true x survival green @s", &alex()), Err(syntax(" survival ", "green", " @s")));
    assert_eq!(parse::<Give>("give 1 2 true \"x survival red @s", &alex()), Err(syntax(" 1 2 true ", "\"x", " survival ")));
    assert_eq!(parse::<Give>("give 1 2 true x survival red", &alex()), Err(syntax("rvival red", "", "")));
    assert_eq!(parse::<Give>("give 1 2 true x survival red Nobody", &alex()), Err(ParseError::NoTarget));
    assert_eq!(parse::<Give>("give 1 2 true x survival red @s", &Sender::Console), Err(ParseError::NoTarget));
    assert_eq!(parse::<Give>("give 1 2 true x survival red @e", &alex()), Err(syntax("vival red ", "@e", "")));
    assert_eq!(parse::<Warp>("warp set home 300", &alex()), Err(syntax(" set home ", "300", "")));
    assert_eq!(parse::<Warp>("warp list now", &alex()), Err(syntax("warp list ", "now", "")));
    assert_eq!(parse::<Ping>("ping pong", &alex()), Err(syntax("/ping ", "pong", "")));
}

#[test]
fn specs_describe_the_syntax_for_clients() {
    let give = Give::spec();
    assert_eq!((give.name.as_str(), give.aliases.as_slice(), give.description.as_str(), give.permission), ("give", &["g".to_owned()][..], "Gives things", Permission::Operator));
    let parameters = &give.overloads[0].parameters;
    let names: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["amount", "ratio", "loud", "item", "mode", "colour", "to", "note"]);
    assert_eq!(parameters.iter().map(|p| p.optional).collect::<Vec<_>>(), [false, false, false, false, false, false, false, true]);
    let colour = ParamKind::Enum { name: "Colour".into(), values: vec!["red".into(), "navy".into()] };
    let kinds: Vec<ParamKind> = parameters.iter().map(|p| p.kind.clone()).collect();
    assert_eq!(kinds[..2], [ParamKind::Int, ParamKind::Float]);
    assert_eq!(kinds[2], ParamKind::Enum { name: "Boolean".into(), values: vec!["true".into(), "false".into()] });
    assert_eq!(kinds[3..], [ParamKind::String, <GameMode as crate::command::Parameter>::kind(), colour, ParamKind::Target, ParamKind::Text]);

    let warp = Warp::spec();
    assert_eq!((warp.name.as_str(), warp.permission, warp.overloads.len()), ("warp", Permission::Member, 3));
    assert_eq!(warp.overloads[0].parameters, [ParameterSpec::literal("warp", "list")]);
    let set: Vec<(&str, bool)> = warp.overloads[1].parameters.iter().map(|p| (p.name.as_str(), p.optional)).collect();
    assert_eq!(set, [("set", false), ("name", false), ("radius", true)]);
    assert_eq!(warp.overloads[2].parameters[0].kind, ParamKind::Enum { name: "warp_del".into(), values: vec!["del".into()] });
    assert!(Ping::spec().overloads[0].parameters.is_empty());
}

fn commands() -> Commands {
    let mut commands = Commands::default();
    commands.register::<Give>();
    commands.register::<Warp>();
    commands.register::<Ping>();
    commands
}

fn run(permission: Permission, line: &str) -> Vec<(String, Vec<String>, bool)> {
    let mut backend = Fake::with(&["Alex", "Bob Ross"]);
    let out = commands().run(&mut ShardCtx::new(&mut backend), &alex(), permission, line);
    assert_eq!(out.succeeded(), out.messages().iter().all(|m| m.success));
    out.messages().iter().map(|m| (m.text.clone(), m.parameters.clone(), m.success)).collect()
}

#[test]
fn the_registry_runs_commands_and_answers_as_bds_does() {
    assert_eq!(run(Permission::Member, "/ping"), [("Alex at tick 40: Ping".to_owned(), vec![], true)]);
    assert_eq!(run(Permission::Member, "  warp list "), [("Alex at tick 40: List".to_owned(), vec![], true)]);
    let ran = run(Permission::Operator, "/G 1 2 true x survival red Alex");
    assert!(ran[0].0.starts_with("Alex at tick 40: Give { amount: 1") && ran[0].2, "{ran:?}");

    assert_eq!(run(Permission::Member, "/nonexistent foo"), [("commands.generic.unknown".to_owned(), vec!["nonexistent".to_owned()], false)]);
    assert_eq!(run(Permission::Member, "/"), [("commands.generic.unknown".to_owned(), vec![String::new()], false)]);
    let syntax = |parts: [&str; 3]| vec![("commands.generic.syntax".to_owned(), parts.map(str::to_owned).to_vec(), false)];
    assert_eq!(run(Permission::Member, "/warp banana"), syntax(["/warp ", "banana", ""]));
    assert_eq!(run(Permission::Member, "/warp"), syntax(["/warp", "", ""]));
    assert_eq!(run(Permission::Operator, "/give 1 2 true x banana"), syntax([" 2 true x ", "banana", ""]), "the last ten characters before");
    assert_eq!(run(Permission::Operator, "/give 1 2 true x survival red Nobody"), [("commands.generic.noTargetMatch".to_owned(), vec![], false)]);
}

#[test]
fn a_member_is_refused_commands_above_it_and_selectors_after_its_line_is_read() {
    let commands = commands();
    let names = |permission| commands.visible(permission).map(|spec| spec.name.as_str()).collect::<Vec<_>>();
    assert_eq!((names(Permission::Member), names(Permission::Operator)), (vec!["warp", "ping"], vec!["give", "warp", "ping"]));
    // As BDS answered a member (research/captures/bds-member.txt): a syntax error first, then any
    // selector, then the command by its own name.
    let answer = |key: &str, parameters: &[&str]| vec![(key.to_owned(), parameters.iter().map(|p| p.to_string()).collect(), false)];
    assert_eq!(run(Permission::Member, "/g"), answer("commands.generic.syntax", &["/g", "", ""]));
    assert_eq!(run(Permission::Member, "/give 1 2 true x survival red @s"), answer("commands.generic.permission.selector", &[]));
    assert_eq!(run(Permission::Member, "/g 1 2 true x survival red Nobody"), answer("commands.generic.error.permissions", &["give"]));
}

#[derive(Command)]
#[command(crate = "crate")]
struct Backwards {
    first: Option<i32>,
    second: i32,
}

impl Command for Backwards {
    fn run(self, _: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        out.print(format!("{:?} {}", self.first, self.second));
    }
}

#[test]
#[should_panic(expected = "a required parameter follows an optional one")]
fn a_required_parameter_after_an_optional_one_is_refused() {
    Commands::default().register::<Backwards>();
}

#[test]
fn registering_a_name_again_replaces_the_command() {
    let mut commands = commands();
    commands.register::<Ping>();
    assert_eq!(commands.visible(Permission::Operator).count(), 3);
}
