//! A plan's Vocabulary terms as scenes: the one row folded and unfolded in
//! an English plan (a term too long for its column and a long list of how
//! one sounds among them), and Denis's preset with its terms unfolded. The
//! preset folded is `setup_fixture::setup_preset`.

use serde_json::json;

use super::super::Ui;
use super::changes::At;
use super::fixture::{open, turn};
use super::fixture_cards::{conversation, item, term, with_cards};
use crate::assistant::changes::{Card, CardKind, Item};

/// Eight terms, one of them unchecked: enough to start folded.
fn terms() -> Vec<Item> {
    let mut terms = vec![
        term("Kubernetes", &["кубернетис", "кубернетес"]),
        term("kubectl", &["кубектл", "куб контрол"]),
        term(
            "Model Context Protocol server",
            &["модел контекст протокол сервер"],
        ),
        term(
            "PostgreSQL",
            &[
                "постгрес",
                "постгрес кью эл",
                "постгре эс кью эл",
                "пост грес",
                "постгря",
                "постгресс",
                "пэ гэ",
                "постгрескуэль",
            ],
        ),
        term("Grafana", &["графана"]),
        term("Donaudampfschifffahrtsgesellschaft", &[]),
        term("Supabase", &["супабейс", "супабэйз"]),
        term("Tailscale", &["тейлскейл"]),
    ];
    terms[6].included = false;
    terms
}

/// A plan of two settings and eight terms; its place in the conversation.
fn plan(ui: &mut Ui) -> At {
    open(ui);
    // Two switches every OS has.
    ui.settings.labs.filler_words = false;
    ui.settings.labs.vocabulary_biasing = false;
    let question = "Set up Vocabulary for the tools I talk about";
    let answer = turn(
        question,
        "Here is what fits how you work. Untick what you don't need.",
        &["topics/vocabulary"],
        1,
    );
    let mut items = vec![
        item(
            "labs.vocabulary_biasing",
            json!(false),
            json!(true),
            "The speech model hears your terms as you spell them",
        ),
        item(
            "labs.filler_words",
            json!(false),
            json!(true),
            "Takes the ums and ahs out of what you dictate",
        ),
    ];
    items.extend(terms());
    let card = Card {
        kind: CardKind::Plan,
        items,
        ..Card::default()
    };
    let id = conversation(ui, question, vec![with_cards(answer, vec![card])], &[]);
    At {
        conversation: id,
        turn: 0,
        card: 0,
    }
}

/// The terms folded: "Add 7 of 8 Vocabulary terms" and the first few.
pub(in super::super) fn plan_terms(ui: &mut Ui) {
    plan(ui);
}

/// The same terms unfolded: a row each, the long ones wrapped in theirs.
pub(in super::super) fn plan_terms_open(ui: &mut Ui) {
    let at = plan(ui);
    ui.assistant.terms_flipped = vec![at];
}

/// Denis's preset with its terms unfolded: every one a row.
pub(in super::super) fn preset_terms_open(ui: &mut Ui) {
    super::setup_fixture::setup_preset(ui);
    if let Some(conversation) = ui.assistant.open {
        ui.assistant.terms_flipped = vec![At {
            conversation,
            turn: 0,
            card: 0,
        }];
    }
}
