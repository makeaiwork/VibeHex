use super::*;
use crate::assistant::store::Turn;

/// Assistant on a Mac with Meetings on, a conversation open and scrolled.
fn ui() -> Ui {
    let mut ui = super::super::super::scenes::mac_fixture();
    ui.assistant.fixture = true;
    ui.settings.meetings.enabled = true;
    ui.pane = Pane::Assistant;
    let id = ui.assistant.store.start("q", None, None);
    ui.assistant.store.push(
        id,
        Turn {
            question: "q".into(),
            answer: "a".into(),
            ..Turn::default()
        },
    );
    ui.assistant.open = Some(id);
    ui
}

fn resolve(ui: &Ui, name: &str) -> Option<(To, Vec<String>)> {
    Known::of(ui).resolve(name).map(|spot| (spot.to, spot.lit))
}

fn lit(titles: &[&str]) -> Vec<String> {
    titles.iter().map(|title| (*title).to_owned()).collect()
}

#[test]
fn every_pane_is_named_as_its_sidebar_names_it() {
    for (pane, name) in panes() {
        assert_eq!(pane.title(), name);
    }
}

#[test]
fn a_pane_and_a_path_into_it_resolve() {
    let ui = ui();
    assert_eq!(
        resolve(&ui, "General › Vocabulary"),
        Some((To::Pane(Pane::General, false), lit(&["Vocabulary"])))
    );
    assert_eq!(
        resolve(&ui, "Output"),
        Some((To::Pane(Pane::Output, false), lit(&[])))
    );
    // Any case, any spacing, a colon after it.
    assert_eq!(
        resolve(&ui, "general  ›  VOCABULARY:"),
        Some((To::Pane(Pane::General, false), lit(&["VOCABULARY"])))
    );
    assert_eq!(
        resolve(&ui, "Meetings › Settings › Detected calls"),
        Some((To::Pane(Pane::Meetings, true), lit(&["Detected calls"])))
    );
    assert_eq!(
        resolve(&ui, "Meetings › Settings"),
        Some((To::Pane(Pane::Meetings, true), lit(&[])))
    );
}

#[test]
fn a_lab_resolves_by_its_title_alone_or_under_labs() {
    let ui = ui();
    // Settings search is a lab on every OS; Smart spacing only on a Mac.
    let search = Some((To::Labs(Some("settings-search")), lit(&["Settings search"])));
    assert_eq!(resolve(&ui, "Labs › Settings search"), search);
    assert_eq!(resolve(&ui, "settings search"), search);
    if cfg!(target_os = "macos") {
        let smart = Some((To::Labs(Some("smart-spacing")), lit(&["Smart spacing"])));
        assert_eq!(resolve(&ui, "smart spacing"), smart);
        let quick = resolve(&ui, "Add to vocabulary from a selection");
        assert!(matches!(quick, Some((To::Labs(Some(_)), _))), "{quick:?}");
    }
    // A lab this Hex doesn't have: Labs itself, its deepest known part.
    assert_eq!(
        resolve(&ui, "Labs › No such lab"),
        Some((To::Labs(None), lit(&[])))
    );
}

#[test]
fn a_rows_long_name_finds_its_section_and_row() {
    let ui = ui();
    assert_eq!(
        resolve(&ui, "Turn off the mic when not dictating"),
        Some((
            To::Pane(Pane::General, false),
            lit(&["Microphone", "Turn off the mic when not dictating"])
        ))
    );
    assert_eq!(
        resolve(&ui, "AI cleanup › Runs on"),
        Some((
            To::Pane(Pane::Output, false),
            lit(&["AI cleanup", "Runs on"])
        ))
    );
}

#[test]
fn what_names_no_place_stays_bold() {
    let ui = ui();
    for name in [
        "Copy",
        "Allow",
        "Reasoning",
        "Always record",
        "Requests › Runs on",
        "General ›",
        "",
    ] {
        assert_eq!(resolve(&ui, name), None, "{name}");
    }
}

#[test]
fn a_pane_this_mac_hides_is_no_place() {
    let mut ui = ui();
    ui.settings.meetings.enabled = false;
    ui.platform.developer = false;
    assert_eq!(resolve(&ui, "Meetings › Settings"), None);
}

#[test]
fn opening_a_place_selects_its_pane_lights_it_and_leads_back() {
    let mut ui = ui();
    open_named(&mut ui, "General › Vocabulary", &mut Tx::default());
    assert_eq!(ui.pane, Pane::General);
    assert_eq!(
        ui.place.as_ref().map(|lit| lit.titles.clone()),
        Some(vec![vec!["vocabulary".to_owned()]])
    );
    assert_eq!(probe(&ui), Some(0), "the Vocabulary row is lit");
    assert!(ui.came_from.is_some(), "Back to Assistant shows");
    // Another pane ends the light.
    ui.select_pane(Pane::History, &mut Tx::default());
    assert!(ui.place.is_none());
}

#[test]
fn a_title_under_advanced_opens_the_fold() {
    let mut folded = ui();
    open_named(&mut folded, "General › Application", &mut Tx::default());
    assert!(general::advanced_open(&folded, Pane::General));
    assert_eq!(probe(&folded), Some(0));
    // One that shows without it leaves the fold closed.
    let mut ui = ui();
    open_named(&mut ui, "General › Vocabulary", &mut Tx::default());
    assert!(!general::advanced_open(&ui, Pane::General));
}

#[test]
fn a_settings_path_opens_that_view() {
    let mut ui = ui();
    open_named(&mut ui, "Meetings › Settings", &mut Tx::default());
    assert_eq!(ui.pane, Pane::Meetings);
    assert_eq!(
        ui.meetings.view,
        super::super::super::meetings::View::Settings
    );
    assert!(ui.place.is_none(), "nothing in it to light");
}

#[test]
fn a_lab_opens_labs_on_its_card_lit() {
    let mut ui = ui();
    open_named(&mut ui, "Labs › Settings search", &mut Tx::default());
    assert!(ui.labs_open);
    assert_eq!(probe(&ui), Some(0), "its card's title is lit");
    assert!(names(&ui, "Settings search"), "its card is lit whole");
    assert!(!names(&ui, "Smart spacing"), "its neighbours aren't");
    assert!(
        ui.came_from.is_some(),
        "Back to Assistant shows on Labs too"
    );
}

#[test]
fn an_answer_links_its_places_only() {
    let ui = ui();
    let known = Known::of(&ui);
    let (out, names) = super::super::strong::linked(
        "Open **General › Vocabulary**, press **Add**, then **Labs › Smart spacing**.",
        |name| known.shown(name),
    );
    assert_eq!(
        out,
        "Open [**General › Vocabulary**](hex-place:0), press **Add**, then [**Labs › Smart spacing**](hex-place:1)."
    );
    assert_eq!(names, ["General › Vocabulary", "Labs › Smart spacing"]);
}

/// A place the model names in English reads in the window's words, and
/// one already in them, or with a full stop, stays as it reads.
#[test]
fn a_place_shows_in_the_windows_words() {
    let ui = ui();
    crate::desktop::i18n::apply(Some("ru"));
    let known = Known::of(&ui);
    let english = known.shown("General › Vocabulary");
    let own = known.shown("Основные › Словарь");
    let stop = known.shown("History.");
    let nowhere = known.shown("Nowhere");
    crate::desktop::i18n::apply(Some("en"));
    assert_eq!(english.as_deref(), Some("Основные › Словарь"));
    assert_eq!(own.as_deref(), Some("Основные › Словарь"));
    assert_eq!(stop.as_deref(), Some("История."));
    assert_eq!(nowhere, None);
}
