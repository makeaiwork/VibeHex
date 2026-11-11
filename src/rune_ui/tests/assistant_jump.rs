//! From an answer to the place it cites and back, driven headlessly: the
//! place lands a third of the way down the pane, and Back to Assistant (or
//! ⌘[) returns to the conversation scrolled where it was.

use super::*;

/// A window short enough that the answer and the transcript scroll.
const SHORT: (f32, f32) = (980.0, 400.0);

fn scrolled(rt: &Runtime<Ui>) -> f32 {
    rt.state.assistant.scroll_y.get()
}

/// Opens the answer's third place from its chip, scrolled part way up the
/// conversation, and comes back by `back`.
fn there_and_back(back: impl Fn(&mut Runtime<Ui>)) {
    let mut rt = runtime("assistant-recordings", true, SHORT);
    let id = rt.state.assistant.open;
    let view = element(&rt, "assistant-scroll").rect;
    rt.wheel(view[0] + view[2] / 2.0, view[1] + view[3] / 2.0, 30.0);
    settle(&mut rt);
    let before = scrolled(&rt);
    assert!(before > 0.0, "the conversation scrolls in a short window");
    assert_eq!(
        rt.scroll_at_bottom("assistant-scroll"),
        Some(false),
        "scrolled up from the end"
    );
    tap(&mut rt, "ref3");
    assert_eq!(rt.state.pane, Pane::Meetings);
    assert!(rt.find("back-to-assistant").is_some());
    back(&mut rt);
    settle(&mut rt);
    assert_eq!(rt.state.pane, Pane::Assistant);
    assert_eq!(rt.state.assistant.open, id);
    assert!(rt.find("back-to-assistant").is_none());
    let after = scrolled(&rt);
    assert!(
        (after - before).abs() < 1.0,
        "back at {after}, left at {before}"
    );
}

#[test]
fn back_to_assistant_returns_to_where_the_conversation_was_scrolled() {
    there_and_back(|rt| tap(rt, "back-to-assistant"));
}

#[test]
fn command_bracket_goes_back_too() {
    there_and_back(|rt| key(rt, Key::Command('['), Mods::default()));
}

/// The cited words land about a third of the way down the pane, whatever
/// its height, and the place at the very end stops where the text does.
#[test]
fn the_cited_place_lands_a_third_of_the_way_down() {
    for height in [560.0, 760.0] {
        let mut state = Ui::with(AppSettings::default(), Service::Fixture(Vec::new()));
        assistant::fixture::meeting_cited_on(&mut state, 0);
        let mut rt = runtime_for(state, true, (980.0, height));
        settle(&mut rt);
        let panel = element(&rt, "panel").rect;
        let (_, y) = rt.find("cited-now").expect("the cited words are lit");
        let down = (y - panel[1]) / panel[3];
        assert!(
            (0.25..0.5).contains(&down),
            "{height}: lit words {down:.2} of the way down"
        );
    }
}
