use super::filters::{Bounds, Input, Not};
use super::items::Inventory;
use super::matching::Candidate;
use super::resolve::pick;
use super::{Base, Filters, Selected, Selector};
use crate::command::{Args, Axis, Parameter, ParseError};
use crate::testing::player;
use crate::{EntityId, EntityInfo, GameMode, Item, Sender, Slot};

fn parsed(line: &str) -> Result<Selector, ParseError> {
    let sender = Sender::Console;
    let mut args = Args::new(line, 0, &sender, &[]);
    let selector = Selector::parse(&mut args)?;
    args.finish().map(|()| selector)
}

fn filters(line: &str) -> Filters {
    match parsed(line) {
        Ok(Selector::Variable(_, filters)) => filters,
        other => panic!("{line}: {other:?}"),
    }
}

fn syntax(before: &str, token: &str, after: &str) -> ParseError {
    ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() }
}

fn not<T>(value: T, negated: bool) -> Not<T> {
    Not { value, negated }
}

#[test]
fn every_base_and_names_parse() {
    let base = |line: &str| match parsed(line) {
        Ok(Selector::Variable(base, filters)) if filters == Filters::default() => base,
        other => panic!("{line}: {other:?}"),
    };
    let bases = ["@s", "@p", "@a", "@r", "@e", "@n", "@initiator", "@e[]"].map(base);
    let wanted = [Base::Sender, Base::NearestPlayer, Base::Players, Base::RandomPlayer, Base::Entities, Base::NearestEntity, Base::Initiator, Base::Entities];
    assert_eq!(bases, wanted);
    assert_eq!([parsed("Alex"), parsed("\"Bob Ross\"")], [Ok(Selector::Named("Alex".into())), Ok(Selector::Named("Bob Ross".into()))]);
    assert_eq!(parsed("@x"), Err(syntax("/", "@x", "")));
    assert_eq!(parsed("@initiators[c=1]"), Err(syntax("/", "@initiators", "[c=1]")));
    assert_eq!(Selector::sender(), Selector::Variable(Base::Sender, Filters::default()));
}

#[test]
fn every_filter_reads_as_bedrock_writes_it() {
    let line = r#"@e[x=1,y=~2,z=~,dx=3,dy=-1,r=..5,rm=1,c=-2,name=!"Bob Ross",type=pig,type=!minecraft:cow,family=Monster,tag=,tag=!red,m=c,m=!default,l=3..,rx=10,rxm=-10,ry=-170,rym=170,scores={kills=1..,deaths=!0},haspermission={camera=disabled,movement=enabled}]"#;
    let f = filters(line);
    assert_eq!(f.origin, [Some(Axis::At(1.0)), Some(Axis::Off(2.0)), Some(Axis::Off(0.0))]);
    assert_eq!((f.volume, f.distance, f.count), ([Some(3.0), Some(-1.0), None], Bounds { min: Some(1.0), max: Some(5.0) }, Some(-2)));
    assert_eq!(f.names, [not("Bob Ross".to_owned(), true)]);
    assert_eq!(f.types, [not("minecraft:pig".to_owned(), false), not("minecraft:cow".to_owned(), true)]);
    assert_eq!((f.families, f.tags), (vec![not("monster".to_owned(), false)], vec![not(String::new(), false), not("red".to_owned(), true)]));
    assert_eq!(f.modes, [not(Some(GameMode::Creative), false), not(None, true)]);
    assert_eq!((f.level, f.pitch, f.yaw), (Bounds { min: Some(3), max: None }, Bounds { min: Some(-10.0), max: Some(10.0) }, Bounds { min: Some(170.0), max: Some(-170.0) }));
    let scores = [("kills".to_owned(), not(Bounds { min: Some(1), max: None }, false)), ("deaths".to_owned(), not(Bounds { min: Some(0), max: Some(0) }, true))];
    assert_eq!((f.scores, f.permissions), (scores.to_vec(), vec![(Input::Camera, false), (Input::Movement, true)]));
    assert_eq!(filters("@a[l=5,lm=2]").level, Bounds { min: Some(2), max: Some(5) }, "a bare `l` is the most");
    assert_eq!(filters("@a[r=2..4]").distance, Bounds { min: Some(2.0), max: Some(4.0) });
    let modes = ["s", "survival", "0", "C", "1", "a", "2", "spectator", "6", "d", "5"].map(|m| filters(&format!("@a[m={m}]")).modes[0].value);
    let (s, c, a, sp) = (Some(GameMode::Survival), Some(GameMode::Creative), Some(GameMode::Adventure), Some(GameMode::Spectator));
    assert_eq!(modes, [s, s, s, c, c, a, a, sp, sp, None, None]);
}

#[test]
fn brackets_may_hold_spaces_and_end_the_argument() {
    let sender = Sender::Console;
    let mut args = Args::new(r#"@e[ type = pig , name = "a b" , c = 1 ] next"#, 0, &sender, &[]);
    let Selector::Variable(_, f) = Selector::parse(&mut args).unwrap() else { panic!() };
    assert_eq!((f.types[0].value.as_str(), f.names[0].value.as_str(), f.count, args.word()), ("minecraft:pig", "a b", Some(1), Ok("next")));
}

#[test]
fn bad_filters_are_syntax_errors_at_the_key_or_value() {
    let cases = [
        ("@e[foo=1]", ("/@e[", "foo", "=1]")),
        ("@e[c=abc]", ("/@e[c=", "abc", "]")),
        ("@e[c=0]", ("/@e[c=", "0", "]")),
        ("@e[r=1,r=2]", ("/@e[r=1,", "r", "=2]")),
        ("@e[c=!1]", ("/@e[c=!", "1", "]")),
        ("@e[m=banana]", ("/@e[m=", "banana", "]")),
        ("@e[type=]", ("/@e[type=", "", "]")),
        ("@e[type=pig", ("e[type=pig", "", "")),
        ("@e[name=a,]", ("@e[name=a,", "", "]")),
        ("@e[scores={a=x}]", ("scores={a=", "x", "}]")),
        ("@e[scores=3]", ("@e[scores=", "3", "]")),
        ("@e[haspermission={fly=enabled}]", ("rmission={", "fly", "=enabled}]")),
        ("@e[haspermission={camera=on}]", ("n={camera=", "on", "}]")),
        ("@e[hasitem={quantity=1}]", ("e[hasitem=", "{quantity=1}", "]")),
        ("@e[x=north]", ("/@e[x=", "north", "]")),
        ("@e[ry=..]", ("/@e[ry=", "..", "]")),
    ];
    for (line, (before, token, after)) in cases {
        assert_eq!(parsed(line), Err(syntax(before, token, after)), "{line}");
    }
}

fn mob(id: u64, kind: &str, feet: [f32; 3]) -> Candidate {
    let entity = EntityInfo::new(EntityId(id), format!("minecraft:{kind}"), feet, [0.0; 3], true);
    Candidate { target: Selected::Entity(entity), kind: format!("minecraft:{kind}"), name: String::new(), feet, rotation: [0.0; 2], families: Vec::new(), tags: Vec::new(), mode: None, level: None, inventory: None }
}

fn someone(id: u64, name: &str, feet: [f32; 3]) -> Candidate {
    let player = Selected::Player(player(id, name));
    let (families, inventory) = (vec!["player".to_owned()], Some(Inventory::default()));
    Candidate { target: player, kind: "minecraft:player".into(), name: name.into(), feet, rotation: [0.0; 2], families, tags: Vec::new(), mode: Some(GameMode::Survival), level: Some(0), inventory }
}

/// Alex, Bob and Cara, two pigs and a zombie, around 0 0 0.
fn world() -> Vec<Candidate> {
    let mut alex = someone(1, "Alex", [3.0, 0.0, 0.0]);
    alex.inventory = Some(Inventory { items: vec![(Slot::Main(0), Item::new("minecraft:apple", 2))], held: 0 });
    let mut bob = someone(2, "Bob", [10.0, 0.0, 0.0]);
    (bob.mode, bob.level, bob.tags) = (Some(GameMode::Creative), Some(5), vec!["red".into()]);
    let mut cara = someone(3, "Cara", [-1.0, 0.0, 0.0]);
    cara.rotation = [90.0, 30.0];
    let mut pigs = [mob(4, "pig", [1.5, 0.0, 0.0]), mob(5, "pig", [20.0, 0.0, 0.0])];
    pigs[1].rotation = [175.0, 0.0];
    for pig in &mut pigs {
        pig.families = vec!["pig".into(), "mob".into()];
    }
    let mut zombie = mob(6, "zombie", [5.0, 0.0, 5.0]);
    (zombie.families, zombie.tags, zombie.rotation) = (["zombie", "undead", "monster", "mob"].map(String::from).to_vec(), vec!["boss".into()], [185.0, 0.0]);
    [alex, bob, cara].into_iter().chain(pigs).chain([zombie]).collect()
}

fn select(line: &str, seed: u64) -> Vec<String> {
    let Ok(Selector::Variable(base, filters)) = parsed(line) else { panic!("{line}") };
    let origin = std::array::from_fn(|axis| filters.origin[axis].map_or(0.0, |at| at.from(0.0)));
    let players_only = matches!(base, Base::NearestPlayer | Base::Players) || (base == Base::RandomPlayer && filters.types.is_empty());
    let candidates = world().into_iter().filter(|candidate| !players_only || candidate.is_player()).collect();
    let label = |candidate: Candidate| match candidate.target {
        Selected::Player(player) => player.name,
        Selected::Entity(entity) => format!("{}{}", entity.kind.trim_start_matches("minecraft:"), entity.id.0),
    };
    pick(base, &filters, origin, candidates, seed).into_iter().map(label).collect()
}

fn picks(line: &str) -> Vec<String> {
    select(line, 7)
}

#[test]
fn order_and_count_follow_the_base_and_c() {
    assert_eq!([picks("@p"), picks("@p[c=-1]"), picks("@p[c=2]")], [vec!["Cara"], vec!["Bob"], vec!["Cara", "Alex"]]);
    assert_eq!([picks("@a"), picks("@a[c=2]"), picks("@a[c=-2]")], [vec!["Alex", "Bob", "Cara"], vec!["Cara", "Alex"], vec!["Bob", "Alex"]]);
    assert_eq!(picks("@e"), ["Alex", "Bob", "Cara", "pig4", "pig5", "zombie6"], "players, then entities oldest first");
    assert_eq!([picks("@n"), picks("@n[type=pig]"), picks("@e[type=pig,c=-1]"), picks("@n[c=3]")], [vec!["Cara"], vec!["pig4"], vec!["pig5"], vec!["Cara", "pig4", "Alex"]]);
    assert_eq!(picks("@p[x=21,y=0,z=0]"), ["Bob"], "counted from x y z");
}

#[test]
fn r_picks_at_random_and_takes_entities_only_with_a_type() {
    let mut seen: Vec<String> = (0..64).flat_map(|seed| select("@r", seed)).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen, ["Alex", "Bob", "Cara"]);
    for seed in 0..16 {
        let two = select("@r[c=2]", seed);
        assert!(two.len() == 2 && two[0] != two[1], "{two:?}");
        assert_eq!(select("@r[c=-2]", seed).len(), 2);
        assert!(select("@r[type=pig]", seed)[0].starts_with("pig"));
    }
    assert_eq!(select("@r[c=9]", 1).len(), 3);
}

#[test]
fn place_filters_measure_from_the_origin() {
    assert_eq!(picks("@e[r=3]"), ["Alex", "Cara", "pig4"], "r takes its edge");
    assert_eq!(picks("@e[rm=4,r=..10]"), ["Bob", "zombie6"]);
    assert_eq!(picks("@e[x=10,r=0]"), ["Bob"]);
    assert_eq!(picks("@e[x=0,y=0,z=0,dx=5,dy=0,dz=5]"), ["Alex", "pig4", "zombie6"], "whole blocks from x y z to x+dx y+dy z+dz");
    assert_eq!(picks("@e[dx=-2]"), ["Cara"], "the other way, dy and dz 0");
    assert_eq!(picks("@e[rxm=20]"), ["Cara"]);
    assert_eq!(picks("@e[rym=45,ry=135]"), ["Cara"]);
    assert_eq!(picks("@e[rym=170,ry=-170]"), ["pig5", "zombie6"], "round past 180");
}

#[test]
fn who_filters_test_names_kinds_tags_and_players() {
    assert_eq!([picks("@a[name=Bob]"), picks("@e[name=!Bob,type=player]")], [vec!["Bob"], vec!["Alex", "Cara"]]);
    assert_eq!([picks("@e[type=!player,type=!pig]"), picks("@e[type=minecraft:zombie]")], [vec!["zombie6"], vec!["zombie6"]]);
    assert_eq!([picks("@e[family=monster]"), picks("@e[family=mob,family=!undead]")], [vec!["zombie6"], vec!["pig4", "pig5"]]);
    assert_eq!(picks("@e[family=player]"), ["Alex", "Bob", "Cara"]);
    assert_eq!([picks("@e[tag=red]"), picks("@e[tag=!]"), picks("@e[tag=!boss,type=!player]")], [vec!["Bob"], vec!["Bob", "zombie6"], vec!["pig4", "pig5"]]);
    assert_eq!(picks("@e[tag=]"), ["Alex", "Cara", "pig4", "pig5"], "an empty tag is having none");
    assert_eq!([picks("@e[m=creative]"), picks("@e[m=!c]"), picks("@a[m=d]")], [vec!["Bob"], vec!["Alex", "Cara"], vec!["Alex", "Cara"]], "m is for players");
    assert_eq!([picks("@a[lm=1]"), picks("@e[l=..4]")], [vec!["Bob"], vec!["Alex", "Cara"]]);
    assert_eq!([picks("@e[haspermission={camera=enabled}]"), picks("@a[haspermission={jump=disabled}]")], [vec!["Alex", "Bob", "Cara"], vec![]]);
    assert_eq!([picks("@e[scores={}]").len(), picks("@e[scores={kills=0..}]").len()], [6, 0], "no scoreboard: nobody has a score");
    assert_eq!(picks("@a[hasitem={item=apple}]"), ["Alex"]);
    assert_eq!(picks("@e[hasitem={item=apple,quantity=0}]"), ["Bob", "Cara", "pig4", "pig5", "zombie6"]);
}

#[test]
fn a_type_that_rules_players_out_is_known() {
    let excludes = |line: &str| parsed(line).unwrap().excludes_players();
    assert_eq!(["@e[type=pig]", "@e[type=!player]", "@e[type=player]", "@e[type=!pig]", "@a", "Alex"].map(excludes), [true, true, false, false, false, false]);
}
