use std::sync::mpsc;

use serde_json::{Value as Json, json};

use crate::form::{Element, FormError, Value};
use crate::testing::{Fake, player};
use crate::{Custom, Form, FormResponse, Menu, Modal, PlayerId, ShardCtx};

fn sent<F: Form>(form: &F) -> Json {
    serde_json::from_str(&form.to_json()).unwrap()
}

#[test]
fn a_menu_is_buttons_and_its_answer_an_index_among_them() {
    let menu = Menu::new("Games").body("Pick one").button("Parkour").button_with_image("Spleef", "textures/items/snowball").button_with_image("Web", "https://example.com/a.png");
    let expected = json!({"type": "form", "title": "Games", "content": "Pick one", "buttons": [
        {"text": "Parkour"},
        {"text": "Spleef", "image": {"type": "path", "data": "textures/items/snowball"}},
        {"text": "Web", "image": {"type": "url", "data": "https://example.com/a.png"}},
    ]});
    assert_eq!(sent(&menu), expected);
    assert_eq!((menu.read("0\n"), menu.read("2")), (Ok(0), Ok(2)));
    assert_eq!(menu.read("3\n"), Err(FormError::OutOfRange { slot: 0 }));
    assert_eq!(menu.read("18446744073709551615"), Err(FormError::OutOfRange { slot: 0 }));
    for wrong in ["-1", "1.5", "\"1\"", "true", "[1]"] {
        assert_eq!(menu.read(wrong), Err(FormError::WrongType { slot: 0 }), "{wrong}");
    }
    assert_eq!((menu.read("one"), menu.read("")), (Err(FormError::NotJson), Err(FormError::NotJson)));
}

#[test]
fn a_modal_answers_yes_or_no() {
    let modal = Modal::new("Leave", "Really?").buttons("Leave", "Stay");
    assert_eq!(sent(&modal), json!({"type": "modal", "title": "Leave", "content": "Really?", "button1": "Leave", "button2": "Stay"}));
    assert_eq!(sent(&Modal::new("T", "B"))["button1"], "gui.yes");
    assert_eq!((modal.read("true\n"), modal.read("false\n")), (Ok(true), Ok(false)));
    assert_eq!((modal.read("1"), modal.read("\"true\"")), (Err(FormError::WrongShape), Err(FormError::WrongShape)));
}

fn sheet() -> Custom {
    Custom::new("Sheet")
        .label("About you")
        .element(Element::Input { label: "Name".into(), placeholder: "Steve".into(), default: String::new(), max_len: 5 })
        .toggle("Flying", true)
        .slider("Speed", 1.0..=10.0, 0.5, 2.0)
        .dropdown("Team", ["red", "blue"], 1)
        .step_slider("Size", ["s", "m", "l"], 0)
}

#[test]
fn a_custom_form_is_sent_element_by_element() {
    let expected = json!({"type": "custom_form", "title": "Sheet", "content": [
        {"type": "label", "text": "About you"},
        {"type": "input", "text": "Name", "placeholder": "Steve", "default": ""},
        {"type": "toggle", "text": "Flying", "default": true},
        {"type": "slider", "text": "Speed", "min": 1.0, "max": 10.0, "step": 0.5, "default": 2.0},
        {"type": "dropdown", "text": "Team", "options": ["red", "blue"], "default": 1},
        {"type": "step_slider", "text": "Size", "steps": ["s", "m", "l"], "default": 0},
    ]});
    assert_eq!(sent(&sheet()), expected);
}

#[test]
fn custom_answers_are_checked_against_their_elements() {
    let sheet = sheet();
    let answer = [Value::None, Value::Text("Alex".into()), Value::Toggle(false), Value::Number(5.0), Value::Choice(1), Value::Choice(2)];
    // A whole slider value arrives as an integer, as vanilla writes it.
    assert_eq!(sheet.read("[null,\"Alex\",false,5,1,2]\n").as_deref(), Ok(&answer[..]));
    assert_eq!(sheet.read("[null,\"\",true,9.5,0,0]").unwrap()[3], Value::Number(9.5));

    let refused = |data: &str| sheet.read(data).unwrap_err();
    assert_eq!(refused("[null,\"Alex\",false,5,1]"), FormError::WrongLength { expected: 6, got: 5 });
    assert_eq!(refused("[null,\"Alex\",false,5,1,2,0]"), FormError::WrongLength { expected: 6, got: 7 });
    assert_eq!(refused("{\"0\":null}"), FormError::WrongShape);
    assert_eq!(refused("[null,\"Alex\",false"), FormError::NotJson);
    assert_eq!(refused("[0,\"Alex\",false,5,1,2]"), FormError::WrongType { slot: 0 });
    assert_eq!(refused("[null,7,false,5,1,2]"), FormError::WrongType { slot: 1 });
    assert_eq!(refused("[null,\"Alexander\",false,5,1,2]"), FormError::TooLong { slot: 1 });
    assert_eq!(refused("[null,\"Alex\",1,5,1,2]"), FormError::WrongType { slot: 2 });
    assert_eq!(refused("[null,\"Alex\",false,\"5\",1,2]"), FormError::WrongType { slot: 3 });
    assert_eq!(refused("[null,\"Alex\",false,0.5,1,2]"), FormError::OutOfRange { slot: 3 });
    assert_eq!(refused("[null,\"Alex\",false,10.5,1,2]"), FormError::OutOfRange { slot: 3 });
    assert_eq!(refused("[null,\"Alex\",false,1e999,1,2]"), FormError::NotJson);
    assert_eq!(refused("[null,\"Alex\",false,5,2,2]"), FormError::OutOfRange { slot: 4 });
    assert_eq!(refused("[null,\"Alex\",false,5,-1,2]"), FormError::WrongType { slot: 4 });
    assert_eq!(refused("[null,\"Alex\",false,5,1,3]"), FormError::OutOfRange { slot: 5 });
    assert_eq!(refused("[null,\"Alex\",false,5,1,1.5]"), FormError::WrongType { slot: 5 });
}

#[derive(crate::Form, Debug, Clone, PartialEq)]
#[form(crate = "crate", title = "Settings")]
struct Settings {
    #[form(note = "Shown to others", label = "Nickname", placeholder = "Steve", max_len = 8)]
    nickname: String,
    sidebar: bool,
    #[form(label = "View distance", min = 2, max = 16)]
    view: u32,
    #[form(min = -1.5, max = 1.5, step = 0.5)]
    tilt: f32,
    #[form(label = "Colour", options = ["red", "green"])]
    colour: usize,
    #[form(options = ["slow", "fast"], step_slider)]
    speed: usize,
}

fn settings() -> Settings {
    Settings { nickname: "Al".into(), sidebar: true, view: 8, tilt: 0.5, colour: 1, speed: 0 }
}

#[test]
fn a_derived_form_shows_the_struct_and_reads_one_back() {
    let expected = json!({"type": "custom_form", "title": "Settings", "content": [
        {"type": "label", "text": "Shown to others"},
        {"type": "input", "text": "Nickname", "placeholder": "Steve", "default": "Al"},
        {"type": "toggle", "text": "sidebar", "default": true},
        {"type": "slider", "text": "View distance", "min": 2.0, "max": 16.0, "step": 1.0, "default": 8.0},
        {"type": "slider", "text": "tilt", "min": -1.5, "max": 1.5, "step": 0.5, "default": 0.5},
        {"type": "dropdown", "text": "Colour", "options": ["red", "green"], "default": 1},
        {"type": "step_slider", "text": "speed", "steps": ["slow", "fast"], "default": 0},
    ]});
    assert_eq!(sent(&settings()), expected);
    let answered = Settings { nickname: "Alexandr".into(), sidebar: false, view: 12, tilt: -1.5, colour: 0, speed: 1 };
    assert_eq!(settings().read("[null,\"Alexandr\",false,12,-1.5,0,1]\n"), Ok(answered));
    assert_eq!(settings().read("[null,\"Alexandra\",false,12,-1.5,0,1]"), Err(FormError::TooLong { slot: 1 }));
    assert_eq!(settings().read("[null,\"Al\",false,17,0,0,1]"), Err(FormError::OutOfRange { slot: 3 }));
    assert_eq!(settings().read("[null,\"Al\",false,12,0,2,1]"), Err(FormError::OutOfRange { slot: 5 }));
    assert_eq!(settings().read("[\"Al\",false,12,0,0,1]"), Err(FormError::WrongLength { expected: 7, got: 6 }));
}

#[test]
fn a_close_or_a_bad_answer_is_closed() {
    let menu = Menu::new("M").button("a");
    assert_eq!(FormResponse::of(&menu, Some("0\n")), FormResponse::Submitted(0));
    for closed in [None, Some("null\n"), Some("1"), Some("garbage")] {
        assert_eq!(FormResponse::of(&menu, closed), FormResponse::Closed, "{closed:?}");
    }
    assert_eq!(FormResponse::of(&settings(), Some("[null,\"x\",true,2,0,0,0]")).submitted().map(|s| s.view), Some(2));
}

#[test]
fn the_context_sends_the_form_and_types_the_answer() {
    let mut backend = Fake::with(&["Alex"]);
    let (answers, answered) = mpsc::channel();
    ShardCtx::new(&mut backend).send_form(PlayerId(1), settings(), move |ctx, player, response| {
        ctx.send_message(player.id, "thanks");
        answers.send(response).unwrap();
    });
    let (to, json, on_answer) = backend.forms.pop().expect("the form reached the shard");
    assert_eq!((to, json), (PlayerId(1), settings().to_json()));
    assert!(answered.try_recv().is_err());
    on_answer(&mut ShardCtx::new(&mut backend), &player(1, "Alex"), Some("[null,\"Bo\",true,3,1,1,1]\n"));
    let expected = Settings { nickname: "Bo".into(), sidebar: true, view: 3, tilt: 1.0, colour: 1, speed: 1 };
    assert_eq!(answered.try_recv(), Ok(FormResponse::Submitted(expected)));
    assert_eq!(backend.log, ["message 1 thanks"]);
}
