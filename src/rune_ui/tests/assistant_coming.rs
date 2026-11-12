//! What an answer shows before its words: "Looking in the guide" and
//! "Preparing a change" are steps going on, with the same three dots as a
//! search, and the dots' clock counts them, so the wave runs.

use super::*;

/// A question being answered: nothing drafted yet, or a draft with a
/// change block coming after it.
fn asking(change_coming: bool) -> Ui {
    let mut state = Ui::with(AppSettings::default(), Service::Fixture(Vec::new()));
    assistant::fixture::answering(&mut state);
    let answering = state.assistant.answering.as_mut().expect("answering");
    if change_coming {
        answering.cards_coming = true;
    } else {
        answering.draft.clear();
    }
    state
}

fn has_dots(rt: &Runtime<Ui>, line: &str) {
    for dot in ["a", "b", "c"] {
        assert!(
            rt.find(&format!("{line}/mark/{dot}")).is_some(),
            "{line} has no dot {dot}"
        );
    }
    assert!(rt.find(&format!("{line}/words")).is_some(), "{line}");
}

#[test]
fn looking_in_the_guide_is_a_step_with_its_three_dots() {
    let mut rt = runtime_for(asking(false), true, PANE_SIZE);
    settle(&mut rt);
    has_dots(&rt, "looking");
    assert!(assistant::steps_live(&rt.state), "the dots' clock runs");
}

#[test]
fn preparing_a_change_is_a_step_with_its_three_dots() {
    let mut rt = runtime_for(asking(true), true, PANE_SIZE);
    settle(&mut rt);
    has_dots(&rt, "preparing");
    assert!(rt.find("looking").is_none(), "the draft shows instead");
    assert!(assistant::steps_live(&rt.state), "the dots' clock runs");
    // Words alone, nothing going on: the clock rests.
    rt.state
        .assistant
        .answering
        .as_mut()
        .expect("answering")
        .cards_coming = false;
    assert!(!assistant::steps_live(&rt.state));
}
