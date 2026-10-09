//! Forms on a shard: ids, answers, and what happens to answers nobody asked for (harness: common/mod.rs).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{
    ModalFormRequest, ModalFormResponse, ModalFormResponseContent,
    ModalFormResponseContentCancelReason as CancelReason, Text,
};
use common::{Harness, all};
use sapling_api::{Form, Menu, Modal, PlayerId};

type Answers = Arc<Mutex<Vec<String>>>;

#[derive(Form, Debug, Clone, PartialEq)]
#[form(crate = "sapling_api", title = "Settings")]
struct Settings {
    nickname: String,
    #[form(min = 2, max = 16)]
    view: u32,
}

fn menu() -> Menu {
    Menu::new("Games").button("Parkour").button("Spleef")
}

/// Sends `form` to `conn` on the next tick; its answer lands in `answers` and is confirmed in chat.
fn send<F>(h: &Harness, conn: u64, form: F, answers: &Answers)
where
    F: Form + Send + 'static,
    F::Response: std::fmt::Debug,
{
    let answers = answers.clone();
    h.remote.run(move |ctx| {
        ctx.send_form(PlayerId(conn), form, move |ctx, player, response| {
            answers
                .lock()
                .unwrap()
                .push(format!("{}: {response:?}", player.name));
            ctx.send_message(player.id, "noted");
        });
    });
}

fn submit(form_id: u32, data: &str) -> ModalFormResponse {
    ModalFormResponse {
        form_id,
        has_response_data: true,
        data: Some(data.into()),
        has_cancel_reason: false,
        content: None,
    }
}

fn close(form_id: u32) -> ModalFormResponse {
    ModalFormResponse {
        form_id,
        has_response_data: false,
        data: None,
        has_cancel_reason: true,
        content: Some(ModalFormResponseContent {
            cancel_reason: CancelReason::Closed,
        }),
    }
}

fn taken(answers: &Answers) -> Vec<String> {
    std::mem::take(&mut *answers.lock().unwrap())
}

#[test]
fn a_form_is_sent_and_its_answer_reaches_the_callback_typed() {
    let (mut h, answers) = (Harness::new(), Answers::default());
    h.enter(1, "Alex");
    send(&h, 1, menu(), &answers);
    let requests = all::<ModalFormRequest>(&h.tick(1));
    assert_eq!(
        (
            requests.len(),
            requests[0].form_id,
            requests[0].data.as_str()
        ),
        (1, 0, menu().to_json().as_str())
    );
    assert!(taken(&answers).is_empty());

    h.receive(1, &submit(0, "1\n"));
    // The callback runs with the shard's context: its message leaves in the same tick.
    assert_eq!(all::<Text>(&h.tick(1)).len(), 1);
    assert_eq!(taken(&answers), ["Alex: Submitted(1)"]);

    send(
        &h,
        1,
        Settings {
            nickname: "Al".into(),
            view: 8,
        },
        &answers,
    );
    send(&h, 1, Modal::new("Sure?", "Really"), &answers);
    let requests = all::<ModalFormRequest>(&h.tick(1));
    assert_eq!(
        requests.iter().map(|r| r.form_id).collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(requests[0].data.contains("custom_form") && requests[1].data.contains("modal"));
    h.receive(1, &submit(2, "true\n"));
    h.receive(1, &submit(1, "[\"Alexa\",12]\n"));
    h.tick(1);
    assert_eq!(
        taken(&answers),
        [
            "Alex: Submitted(true)",
            r#"Alex: Submitted(Settings { nickname: "Alexa", view: 12 })"#
        ]
    );
}

#[test]
fn answers_nobody_is_waiting_for_are_ignored() {
    let (mut h, answers) = (Harness::new(), Answers::default());
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // Never sent.
    h.receive(1, &submit(0, "0\n"));
    assert!(all::<Text>(&h.tick(1)).is_empty());

    send(&h, 1, menu(), &answers);
    h.tick_all();
    // Ids are per player: Steve has no form 0, and his answer does not use up Alex's.
    h.receive(2, &submit(0, "0\n"));
    h.receive(1, &submit(7, "0\n"));
    h.tick_all();
    assert!(taken(&answers).is_empty());
    h.receive(1, &submit(0, "0\n"));
    h.receive(1, &submit(0, "1\n"));
    h.tick_all();
    assert_eq!(taken(&answers), ["Alex: Submitted(0)"]);
    h.receive(1, &close(0));
    h.tick_all();
    assert!(taken(&answers).is_empty());

    send(&h, 2, menu(), &answers);
    assert_eq!(all::<ModalFormRequest>(h.tick_all().to(2))[0].form_id, 0);
}

#[test]
fn closed_and_impossible_answers_are_closed() {
    let (mut h, answers) = (Harness::new(), Answers::default());
    h.enter(1, "Alex");
    (0..5).for_each(|_| send(&h, 1, menu(), &answers));
    send(
        &h,
        1,
        Settings {
            nickname: String::new(),
            view: 2,
        },
        &answers,
    );
    h.tick(1);
    h.receive(1, &close(0));
    h.receive(1, &submit(1, "null\n"));
    h.receive(1, &submit(2, "2\n"));
    h.receive(1, &submit(3, "\"0\""));
    h.receive(1, &submit(4, &" ".repeat(40_000)));
    h.receive(1, &submit(5, "[\"x\",99]"));
    h.tick(1);
    assert_eq!(taken(&answers), ["Alex: Closed"; 6]);
}

#[test]
fn pending_forms_are_capped_and_dropped_when_the_player_leaves() {
    let (mut h, answers) = (Harness::new(), Answers::default());
    h.enter(1, "Alex");
    (0..20).for_each(|_| send(&h, 1, menu(), &answers));
    assert_eq!(all::<ModalFormRequest>(&h.tick(1)).len(), 16);
    // Each pending callback holds the answers; the four refused ones are gone already.
    assert_eq!(Arc::strong_count(&answers), 17);
    h.leave(1);
    h.tick_all();
    assert_eq!(Arc::strong_count(&answers), 1);
    assert!(taken(&answers).is_empty());

    // A form for someone who is gone or still joining is not sent and not kept.
    h.join(2, "Early");
    send(&h, 1, menu(), &answers);
    send(&h, 2, menu(), &answers);
    assert!(all::<ModalFormRequest>(h.tick_all().to(2)).is_empty());
    assert_eq!(Arc::strong_count(&answers), 1);
}
