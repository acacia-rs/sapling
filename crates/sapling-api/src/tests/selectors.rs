//! Selectors through a registered command: parsed from the line, resolved on the shard.

use crate::command::{Selected, Selector};
use crate::testing::{Fake, player};
use crate::{Command, Commands, EntityId, EntityInfo, Output, Permission, PlayerId, Sender, ShardCtx, Tagged};

#[derive(Command)]
#[command(crate = "crate", name = "who", permission = "operator")]
struct Who {
    target: Selector,
}

impl Command for Who {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let named: Vec<String> = ctx.select(sender, &self.target).into_iter().map(|found| match found {
            Selected::Player(player) => player.name,
            Selected::Entity(entity) => entity.kind,
        }).collect();
        match named.is_empty() {
            true => out.translate("commands.generic.noTargetMatch", Vec::new(), false),
            false => out.print(named.join(", ")),
        }
    }
}

/// Alex (who runs the commands) at 0 0 0, Bob Ross at 9 0 0, a pig at 2 0 0 and a cow at 30 0 0.
fn shard() -> Fake {
    let mut backend = Fake::with(&["Alex", "Bob Ross"]);
    backend.feet.extend([(PlayerId(1), [0.0; 3]), (PlayerId(2), [9.0, 0.0, 0.0])]);
    let entity = |id, kind: &str, x| EntityInfo::new(EntityId(id), kind.into(), [x, 0.0, 0.0], [0.0; 3], true);
    backend.entities = vec![entity(10, "minecraft:pig", 2.0), entity(11, "minecraft:cow", 30.0)];
    backend
}

fn who(backend: &mut Fake, line: &str) -> (String, Vec<String>) {
    let mut commands = Commands::default();
    commands.register::<Who>();
    let out = commands.run(&mut ShardCtx::new(backend), &Sender::Player(player(1, "Alex")), Permission::Operator, line);
    let said = out.messages().last().expect("an answer");
    (said.text.clone(), said.parameters.clone())
}

fn names(line: &str) -> String {
    who(&mut shard(), line).0
}

#[test]
fn commands_resolve_selectors_on_the_shard() {
    assert_eq!(names("/who @a"), "Alex, Bob Ross");
    assert_eq!(names("/who @e"), "Alex, Bob Ross, minecraft:pig, minecraft:cow");
    assert_eq!(names("/who @e[type=!player, c=-1]"), "minecraft:cow");
    assert_eq!(names("/who @n[type=!player]"), "minecraft:pig");
    assert_eq!(names("/who @p[rm=1]"), "Bob Ross");
    assert_eq!(names("/who @a[r=5]"), "Alex");
    assert_eq!(names("/who @e[x=9,r=..7.5,name=!Alex]"), "Bob Ross, minecraft:pig");
    assert_eq!(names("/who \"Bob Ross\""), "Bob Ross");
    assert!(["Alex", "Bob Ross"].contains(&names("/who @r").as_str()));
    assert_eq!(names("/who @initiator"), "commands.generic.noTargetMatch");
    assert_eq!(names("/who @a[m=creative]"), "commands.generic.noTargetMatch");
}

#[test]
fn tags_put_on_the_shard_are_what_tag_tests() {
    let mut backend = shard();
    assert_eq!(who(&mut backend, "/who @s[tag=vip]").0, "commands.generic.noTargetMatch");
    let mut ctx = ShardCtx::new(&mut backend);
    assert!(ctx.add_tag(Tagged::Player(PlayerId(1)), "vip") && ctx.add_tag(Tagged::Entity(EntityId(11)), "vip"));
    assert!(!ctx.add_tag(Tagged::Player(PlayerId(1)), "vip") && ctx.tags(Tagged::Player(PlayerId(1))) == ["vip"]);
    assert_eq!(who(&mut backend, "/who @s[tag=vip]").0, "Alex");
    assert_eq!(who(&mut backend, "/who @e[tag=vip]").0, "Alex, minecraft:cow");
    assert_eq!(who(&mut backend, "/who @e[tag=]").0, "Bob Ross, minecraft:pig");
    assert!(ShardCtx::new(&mut backend).remove_tag(Tagged::Player(PlayerId(1)), "vip"));
    assert_eq!(who(&mut backend, "/who @a[tag=vip]").0, "commands.generic.noTargetMatch");
}

#[test]
fn bad_selectors_answer_with_bds_syntax_errors() {
    let syntax = |parts: [&str; 3]| ("commands.generic.syntax".to_owned(), parts.map(str::to_owned).to_vec());
    assert_eq!(who(&mut shard(), "/who @e[foo=1]"), syntax(["/who @e[", "foo", "=1]"]));
    assert_eq!(who(&mut shard(), "/who @a[m=banana]"), syntax(["/who @a[m=", "banana", "]"]));
    assert_eq!(who(&mut shard(), "/who @q"), syntax(["/who ", "@q", ""]));
    assert_eq!(who(&mut shard(), "/who @e[c=1] extra"), syntax(["o @e[c=1] ", "extra", ""]));
}
