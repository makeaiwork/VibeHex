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
