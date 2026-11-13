//! Layout tests of the Assistant pane, on every OS and with no golden
//! image: the generic check ([`check`]) over every Assistant scene, and
//! over a matrix of its surfaces by language, content, and width
//! ([`matrix`]); and the feature doc's list of those surfaces held to the
//! code ([`inventory`]).

mod check;
mod inventory;
mod matrix;

use rune::text::TextSystem;
use rune::{Env, Runtime};

use super::super::{SCENES, Service, Ui, offscreen, on_key, poll, view};
use super::long_fixture as long;
use crate::app_settings::AppSettings;
pub(super) use check::{Laid, Problem, Rules, numberless, problems};

/// What the pane cuts or overlaps on purpose.
pub(super) const RULES: Rules = Rules {
    scrolls: &["scroll", "list", "listbox", "assistant-scroll"],
    floats: &[
        ("#link-", "a link's press area lies over its words"),
        ("#mark-", "a mark's tint lies over its words"),
        ("eyes", "the face's eyes look aside"),
        (
            "#choices-menu",
            "a companion's open choices are the kit's menu, over the window",
        ),
        (
            "assistant-engine-control",
            "a quiet select hangs 8 pt past its row's edge, so its words end where the row's do",
        ),
        (
            "shortcut-keys",
            "the shortcuts card's rows hold General's recorder, 3 pt wider than its control",
        ),
        (
            "shortcut-mode-trigger",
            "the shortcuts card's Shortcut mode is a quiet select, 8 pt past its row's edge",
        ),
        (
            "dictation-hotkey",
            "General's shortcut recorder, 3 pt wider than its control: a place Assistant opens, not its surface",
        ),
    ],
    cuts: &[
        (
            "assistant-picker/name",
            "a narrow window cuts the open conversation's name, not the pane's title",
        ),
        (
            "/copy/description",
            "a Labs card's description stops at two lines: a place Assistant opens, not its surface",
        ),
    ], // A long value's three lines, with "Show all" under them (`value`).
    unfolds: &["all"],
};

const SCALE: f32 = 2.0;

/// A window of `size` showing what `apply` sets up, laid out and settled.
pub(super) fn window(apply: impl FnOnce(&mut Ui), size: (f32, f32)) -> Runtime<Ui> {
    let mut state = Ui::with(AppSettings::default(), Service::Fixture(Vec::new()));
    apply(&mut state);
    let env = Env {
        system_dark: false,
        scale: SCALE,
    };
    let text = TextSystem::new(rune_theme_opencode::FONTS, rune_theme_opencode::FAMILIES);
    let mut rt = offscreen::runtime(state, view, size, env, text);
    rt.poll = Some(poll);
    rt.on_key = Some(on_key);
    rt.settle_all();
    rt.frame(0.0);
    for _ in 0..30 {
        rt.frame(1.0 / 30.0);
    }
    rt.settle_all();
    rt.frame(0.0);
    rt
}

/// The Assistant's scenes, by name.
pub(super) fn scenes() -> impl Iterator<Item = (&'static str, fn(&mut Ui))> {
    SCENES
        .iter()
        .filter(|(name, _)| name.starts_with("assistant"))
        .map(|(name, apply)| (*name, *apply))
}

/// What is wrong in the pane, the sidebar and the window's chrome left
/// to their own tests.
pub(super) fn faults(rt: &Runtime<Ui>) -> Vec<Problem> {
    problems(&Laid::of(rt, SCALE), RULES)
}
