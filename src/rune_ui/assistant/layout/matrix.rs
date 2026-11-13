//! Every Assistant scene through the layout check, and the matrix: each
//! of the pane's surfaces by language, by content, by window width. A
//! surface is a scene's fixture; a cell sets its language, lengthens what
//! a model wrote in it (`long`), and lays it out at a width.
//!
//! `docs/features/assistant.md` lists each surface with its scenes and its
//! test here ("Surfaces, scenes, and tests"), and `layout::inventory`
//! fails when a scene or a surface is missing from it.

use super::super::super::{MIN_WINDOW_SIZE, WINDOW_SIZE};
use super::*;

/// Tall enough that nothing in a conversation is scrolled away.
const TALL: f32 = 9000.0;

/// English, Russian, and German for its long words.
const LANGUAGES: [&str; 3] = ["en", "ru", "de"];

/// The default width and the narrowest the window goes.
const WIDTHS: [f32; 2] = [WINDOW_SIZE.0, MIN_WINDOW_SIZE.0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Content {
    /// The fixture as it is.
    Ordinary,
    /// Long titles, long model names, 95 terms, long sounds-like lists.
    Long,
}

/// One of the pane's surfaces: its name in the inventory, its test here,
/// and the scenes that show it.
pub(super) struct Surface {
    pub(super) name: &'static str,
    pub(super) test: &'static str,
    pub(super) scenes: &'static [&'static str],
}

/// The matrix's rows. A new surface gets a row, a test, and a line in
/// the feature doc's inventory.
pub(super) const SURFACES: &[Surface] = &[
    Surface {
        name: "Start page",
        test: "start_page",
        scenes: &["assistant-start"],
    },
    Surface {
        name: "Composer",
        test: "composer",
        scenes: &["assistant"],
    },
    Surface {
        name: "Answer with steps and what it knew",
        test: "answer_with_steps_and_what_it_knew",
        scenes: &["assistant-steps-open", "assistant-knew"],
    },
    Surface {
        name: "Setting card",
        test: "setting_card",
        scenes: &["assistant-cards"],
    },
    Surface {
        name: "Apply N changes",
        test: "apply_n_changes",
        scenes: &["assistant-cards-more", "assistant-change-both"],
    },
    Surface {
        name: "Plan with its terms folded",
        test: "plan_with_its_terms_folded",
        scenes: &["assistant-plan-terms"],
    },
    Surface {
        name: "Plan with its terms unfolded",
        test: "plan_with_its_terms_unfolded",
        scenes: &["assistant-plan-terms-open"],
    },
    Surface {
        name: "Plan with long values",
        test: "plan_with_long_values",
        scenes: &["assistant-long-values", "assistant-long-values-open"],
    },
    Surface {
        name: "Shortcuts card",
        test: "shortcuts_card",
        scenes: &[
            "assistant-shortcuts-card",
            "assistant-setup-preset-shortcuts-recording",
            "assistant-setup-preset-shortcuts-recorded",
        ],
    },
    Surface {
        name: "Vocabulary edit card",
        test: "vocabulary_edit_card",
        scenes: &["assistant-vocab-edit"],
    },
    Surface {
        name: "Follow-up round",
        test: "follow_up_round",
        scenes: &["assistant-setup-follow", "assistant-setup-follow-ru"],
    },
    Surface {
        name: "Returning opener",
        test: "returning_opener",
        scenes: &["assistant-setup-returning"],
    },
    Surface {
        name: "Consent card",
        test: "consent_card",
        scenes: &["assistant-consent"],
    },
    Surface {
        name: "Consent card mid-answer",
        test: "consent_card_mid_answer",
        scenes: &["assistant-agent-consent"],
    },
    Surface {
        name: "No model page",
        test: "no_model_page",
        scenes: &["assistant-connect"],
    },
];

fn scene(name: &str) -> fn(&mut Ui) {
    scenes()
        .find(|(scene, _)| *scene == name)
        .unwrap_or_else(|| panic!("no scene {name}"))
        .1
}

/// A problem without the window's chrome and a list's row numbers, so
/// 95 rows with one fault read as one.
fn short(problem: &Problem) -> String {
    let path = numberless(&problem.path.replace("kit-root/window/panel/", ""));
    let kind = problem.to_string();
    let kind = kind.split(':').next().unwrap_or_default();
    format!("{kind}: {path}")
}

/// What is wrong in `rt`, each fault once with its first detail.
fn distinct(rt: &Runtime<Ui>) -> Vec<String> {
    let mut found: Vec<(String, String)> = Vec::new();
    for problem in faults(rt) {
        let key = short(&problem);
        if !found.iter().any(|(known, _)| *known == key) {
            found.push((key, problem.detail));
        }
    }
    found
        .into_iter()
        .map(|(key, detail)| format!("{key} ({detail})"))
        .collect()
}

/// What is wrong in one cell.
fn cell(
    surface: &Surface,
    name: &str,
    language: &str,
    content: Content,
    width: f32,
) -> Vec<String> {
    let apply = scene(name);
    let rt = window(
        |ui| {
            apply(ui);
            long::speak(ui, language);
            if content == Content::Long {
                long::stretch(ui, language);
                if surface.name == "Composer" {
                    long::draft(ui, language);
                }
            }
        },
        (width, TALL),
    );
    distinct(&rt)
        .into_iter()
        .map(|fault| format!("{name} {language} {content:?} {width}: {fault}"))
        .collect()
}

/// Every cell of one surface: its scenes by language, content, and width.
fn surface(test: &str) {
    let surface = SURFACES
        .iter()
        .find(|surface| surface.test == test)
        .unwrap_or_else(|| panic!("no surface for {test}"));
    let mut found = Vec::new();
    for name in surface.scenes {
        for language in LANGUAGES {
            for content in [Content::Ordinary, Content::Long] {
                for width in WIDTHS {
                    found.extend(cell(surface, name, language, content, width));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "{}: {} layout faults\n{}",
        surface.name,
        found.len(),
        found.join("\n")
    );
}

macro_rules! surfaces {
    ($($test:ident),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                surface(stringify!($test));
            }
        )*

        /// Every row of the matrix has its test.
        #[test]
        fn every_surface_has_its_test() {
            let tests = [$(stringify!($test)),*];
            let rows: Vec<&str> = SURFACES.iter().map(|surface| surface.test).collect();
            assert_eq!(rows, tests);
        }
    };
}

surfaces!(
    start_page,
    composer,
    answer_with_steps_and_what_it_knew,
    setting_card,
    apply_n_changes,
    plan_with_its_terms_folded,
    plan_with_its_terms_unfolded,
    plan_with_long_values,
    shortcuts_card,
    vocabulary_edit_card,
    follow_up_round,
    returning_opener,
    consent_card,
    consent_card_mid_answer,
    no_model_page,
);

/// Every Assistant scene as its fixture is, in the window as it opens
/// and in one tall enough to show the whole conversation.
#[test]
fn every_scene_is_laid_out_whole() {
    let mut found = Vec::new();
    for (name, apply) in scenes() {
        for size in [WINDOW_SIZE, (WINDOW_SIZE.0, TALL)] {
            let rt = window(apply, size);
            found.extend(
                distinct(&rt)
                    .into_iter()
                    .map(|fault| format!("{name} {size:?}: {fault}")),
            );
        }
    }
    assert!(
        found.is_empty(),
        "{} layout faults\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The long content is what it says: 95 terms in a plan, and the long
/// words of the language asked.
#[test]
fn the_long_content_is_long() {
    let apply = scene("assistant-plan-terms-open");
    let rt = window(
        |ui| {
            apply(ui);
            long::stretch(ui, "de");
        },
        (WINDOW_SIZE.0, TALL),
    );
    let texts: Vec<String> = rt
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    let has = |needle: &str| texts.iter().any(|text| text.contains(needle));
    assert!(has(&format!("Kubernetes{}", long::TERMS - 5)), "{texts:?}");
    assert!(has(
        "Rindfleischetikettierungsüberwachungsaufgabenübertragungsgesetz"
    ));
    assert!(has(long::MODEL));
}
