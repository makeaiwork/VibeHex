//! The ways Assistant invites a person to set VibeHex up, driven headlessly
//! on every OS: onboarding's "Set up with Assistant", and the sidebar's
//! mark after an update.

use rune::Runtime;

use super::super::sidebar::Pane;
use super::super::{SCENES, Ui, WINDOW_SIZE};
use super::layout::window;
use crate::assistant::setup::flow::{self, Step};

/// Scene `name`, or what `name` sets up after `then`, laid out.
fn scene(name: &str, then: impl FnOnce(&mut Ui)) -> Runtime<Ui> {
    let (_, apply) = SCENES
        .iter()
        .find(|(scene, _)| *scene == name)
        .unwrap_or_else(|| panic!("no scene {name}"));
    window(
        |ui| {
            apply(ui);
            then(ui);
        },
        WINDOW_SIZE,
    )
}

/// Presses `key` and lets the window settle.
fn tap(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
}

fn texts(rt: &Runtime<Ui>) -> Vec<String> {
    rt.elements().into_iter().filter_map(|e| e.text).collect()
}

/// VibeHex is ready offers Set up with Assistant beside Start using
/// VibeHex. A press does what Start does (the same readiness rule records
/// completion; onboarding closes), then opens Assistant on a new
/// conversation with setup's round one showing, no model asked.
#[test]
fn set_up_with_assistant_finishes_onboarding_and_opens_round_one() {
    // A new install: no dictations, meetings, or setups yet.
    let mut rt = scene("mac-onboarding-ready", |ui| {
        ui.history.entries.clear();
        ui.meetings.meetings.clear();
        // An upgrader shown onboarding again: opening Assistant here
        // counts as opening it after the update.
        ui.assistant.mark = Some("3.3.0-vibe.7".into());
    });
    assert!(rt.find("start-using-hex").is_some());
    assert!(
        super::super::onboarding_kit::live::completes(&rt.state),
        "Start's rule would record completion here"
    );
    tap(&mut rt, "set-up-with-assistant");
    let ui = &rt.state;
    assert!(
        ui.tour.is_none() && ui.first_run.is_none(),
        "onboarding closed"
    );
    assert_eq!(ui.pane, Pane::Assistant);
    assert_eq!(ui.assistant.view, super::View::Chat);
    let open = ui.assistant.conversation().expect("a new conversation");
    assert!(
        matches!(
            flow::step(&open.turns, ui.assistant.store.setups()),
            Step::Basics { folded: false, .. }
        ),
        "round one is open"
    );
    assert!(ui.assistant.answering.is_none(), "no model was asked");
    assert!(ui.assistant.mark.is_none(), "Assistant was opened");
    let shown = texts(&rt);
    assert!(
        shown
            .iter()
            .any(|text| text == "What do you use VibeHex for?"),
        "{shown:?}"
    );
}

/// Where Assistant is turned off, VibeHex is ready offers Start alone.
#[test]
fn with_assistant_off_onboarding_offers_start_alone() {
    let rt = scene("mac-onboarding-ready", |ui| {
        ui.settings.assistant.enabled = false;
    });
    assert!(rt.find("start-using-hex").is_some());
    assert!(rt.find("set-up-with-assistant").is_none());
}

/// Whether Assistant's sidebar row, or its rail button, shows the mark.
fn marked(rt: &Runtime<Ui>) -> bool {
    rt.elements()
        .into_iter()
        .any(|e| e.key.as_deref() == Some("mark") && e.path.contains("nav-assistant"))
}

/// After an update the sidebar's Assistant row carries a mark. Opening
/// Assistant lands on a new conversation's start page, Updated to and
/// Set VibeHex up for me together, not on the conversation left open; the
/// mark is gone, and leaving and coming back doesn't bring it back.
#[test]
fn the_mark_leads_once_to_the_start_page() {
    let mut rt = scene("assistant-marked", |ui| {
        let store = &mut ui.assistant.store;
        let id = store.start("Earlier", None, None);
        store.push(
            id,
            crate::assistant::store::Turn {
                question: "Earlier".into(),
                answer: "An answer.".into(),
                ..Default::default()
            },
        );
        ui.assistant.open = Some(id);
    });
    assert!(marked(&rt), "the row is marked");
    tap(&mut rt, "nav-assistant");
    assert_eq!(rt.state.pane, Pane::Assistant);
    assert_eq!(rt.state.assistant.open, None, "a new conversation");
    assert!(rt.find("whats-new").is_some(), "Updated to 3.3.0-vibe.7");
    assert!(rt.find("setup").is_some(), "Set VibeHex up for me");
    assert!(!marked(&rt) && rt.state.assistant.mark.is_none());
    // Away and back: no mark, and the conversation stays as left.
    let earlier = rt.state.assistant.store.conversations()[0].id;
    rt.state.assistant.open = Some(earlier);
    tap(&mut rt, "nav-general");
    assert!(!marked(&rt));
    tap(&mut rt, "nav-assistant");
    assert_eq!(rt.state.assistant.open, Some(earlier));
}

/// The folded sidebar's face carries the same mark; a first install, or
/// Assistant turned off, has none.
#[test]
fn the_rail_carries_the_mark_and_a_first_install_none() {
    assert!(marked(&scene("assistant-marked-rail", |_| {})));
    assert!(!marked(&scene("assistant-start", |ui| {
        ui.pane = Pane::General;
    })));
    assert!(!marked(&scene("assistant-marked", |ui| {
        ui.settings.assistant.enabled = false;
    })));
}
