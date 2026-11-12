//! A plan's Vocabulary terms as one row: a checkbox for them all, "Add 95
//! Vocabulary terms" (or "Add 90 of 95 …" once some are unchecked) with a
//! chevron, and under it, while folded, the first few by name. Unfolded,
//! each term is the row it always was (checkbox, term, how it sounds,
//! Edit), under why it is offered. It starts unfolded with a handful of
//! terms (`Terms::FEW`) and folded above that: the owner's preset brings
//! 95, and a list that long buries the settings it comes with. The group
//! is one change in the plan's counts (`state::plan_counts`), whatever it
//! holds; the save and Undo still work term by term.

use rune::{El, col, icon, row, text};
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint};
use super::changes::{self as act, At};
use crate::assistant::changes::state::{Live, Row, Terms};
use crate::assistant::changes::{Card, vocab};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// How many terms the folded row names before "and N more".
const NAMED: usize = 4;

/// Whether the terms of the plan at `at` show unfolded.
pub(super) fn open(ui: &Ui, at: At, card: &Card, terms: &Terms) -> bool {
    // A term being edited is never folded away.
    let editing = ui
        .assistant
        .vocab_edit
        .is_some_and(|(edited, index)| edited == at && terms.rows.contains(&index));
    editing || (terms.starts_open(card) != ui.assistant.terms_flipped.contains(&at))
}

/// "Add 95 Vocabulary terms", or how many of them are checked.
pub(super) fn title(terms: &Terms) -> String {
    match (terms.checked, terms.rows.len()) {
        (1, 1) => tr("Add 1 Vocabulary term").to_owned(),
        (checked, total) if checked == total => {
            tr_fill("Add {} Vocabulary terms", &total.to_string())
        }
        (checked, total) => tr_named(
            "Add {checked} of {total} Vocabulary terms",
            &[
                ("checked", &checked.to_string()),
                ("total", &total.to_string()),
            ],
        ),
    }
}

/// The first few terms by name, "and 91 more" after them.
fn named(card: &Card, terms: &Terms) -> String {
    let names: Vec<String> = terms
        .rows
        .iter()
        .take(NAMED)
        .filter_map(|index| vocab::terms(&card.items[*index].to).into_iter().next())
        .map(|term| term.term)
        .collect();
    match terms.rows.len().saturating_sub(names.len()) {
        0 => names.join(", "),
        more => format!(
            "{} {}",
            names.join(", "),
            tr_fill("and {} more", &more.to_string())
        ),
    }
}

/// Checks every term, or unchecks them all when all are checked.
fn toggle_all(ui: &mut Ui, at: At) {
    act::set(ui, at, |card| {
        let Some(terms) = Terms::of(card) else {
            return;
        };
        let check = terms.checked < terms.rows.len();
        for index in terms.rows {
            card.items[index].included = check;
        }
    });
}

/// The one row: the checkbox (or how the terms went), the title with its
/// chevron, and the first few names while folded.
pub(super) fn head(ui: &Ui, p: Paint, at: At, card: &Card, terms: &Terms, live: &Live) -> El {
    let c = &p.k.c;
    let open = open(ui, at, card, terms);
    let title = title(terms);
    let shown = terms.row(card, live);
    let lead = match &shown {
        Row::Open { checked, .. } => {
            super::tick::tick(p, *checked, &title, move |ui: &mut Ui| toggle_all(ui, at))
        }
        Row::Applied => super::vocab::mark(&theme::CHECK, p.success()),
        Row::Failed(_) => super::vocab::mark(&theme::CIRCLE_EXCLAMATION, p.danger()),
        Row::Left | Row::Downloading { .. } => super::vocab::mark(&theme::DASH, c.icon_faint),
    };
    let off = matches!(shown, Row::Left);
    let fold = row()
        .key("fold")
        .role("button")
        .label(title.clone())
        .checked(open)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .gap(6.0)
        .center_y()
        .shrink()
        .min_w(0.0)
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .child(
            text(title.clone())
                .key("name")
                .font(rune::TextStyle {
                    weight: 520.0,
                    ..p.k.ty.ui
                })
                .color(if off { p.muted() } else { p.text() })
                .wrap_anywhere()
                .shrink(),
        )
        .child(
            icon(
                if open {
                    &theme::CHEVRON_DOWN
                } else {
                    &theme::CHEVRON_RIGHT
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_faint),
        )
        .on_press(move |ui: &mut Ui| {
            let flipped = &mut ui.assistant.terms_flipped;
            match flipped.iter().position(|known| *known == at) {
                Some(index) => {
                    flipped.remove(index);
                }
                None => flipped.push(at),
            }
        });
    let mut words = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(row().key("names").child(fold));
    if !open {
        words = words.child(
            text(named(card, terms))
                .key("named")
                .font(DESCRIPTION)
                .color(p.muted())
                .wrap_anywhere(),
        );
    }
    row()
        .key("terms")
        .role("group")
        .label(title)
        .px(14.0)
        .py(9.0)
        .gap(10.0)
        .items_start()
        .child(row().key("lead").pt(2.0).child(lead))
        .child(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::Decision;
    use crate::assistant::changes::state;

    /// The scene's plan: two settings and eight terms, one unchecked.
    fn plan() -> (Ui, At) {
        crate::desktop::i18n::apply(Some("en"));
        let mut ui = super::super::super::scenes::mac_fixture();
        super::super::terms_fixture::plan_terms(&mut ui);
        let at = At {
            conversation: ui.assistant.open.expect("open"),
            turn: 0,
            card: 0,
        };
        (ui, at)
    }

    fn terms(ui: &Ui, at: At) -> Terms {
        Terms::of(act::card(ui, at).expect("card")).expect("terms")
    }

    #[test]
    fn eight_terms_are_one_folded_row_and_one_change() {
        let (mut ui, at) = plan();
        let card = act::card(&ui, at).unwrap().clone();
        let group = terms(&ui, at);
        assert!(!open(&ui, at, &card, &group), "eight start folded");
        assert_eq!(title(&group), "Add 7 of 8 Vocabulary terms");
        assert_eq!(
            named(&card, &group),
            "Kubernetes, kubectl, Model Context Protocol server, PostgreSQL and 4 more"
        );
        // Two settings and the terms: three changes, whatever the terms.
        assert_eq!(state::plan_counts(&card), (3, 3));

        // The row's checkbox checks them all, then none.
        toggle_all(&mut ui, at);
        assert_eq!(title(&terms(&ui, at)), "Add 8 Vocabulary terms");
        toggle_all(&mut ui, at);
        assert_eq!(terms(&ui, at).checked, 0);
        assert_eq!(
            state::plan_counts(act::card(&ui, at).unwrap()),
            (3, 2),
            "no term checked: the group isn't selected"
        );

        // The chevron unfolds it; a term being edited never folds away.
        ui.assistant.terms_flipped.push(at);
        assert!(open(&ui, at, &card, &group));
        ui.assistant.terms_flipped.clear();
        ui.assistant.vocab_edit = Some((at, group.rows[3]));
        assert!(open(&ui, at, &card, &group));
    }

    #[test]
    fn a_handful_starts_unfolded_and_one_term_reads_as_one() {
        let (mut ui, at) = plan();
        act::set(&mut ui, at, |card| card.items.truncate(3));
        let card = act::card(&ui, at).unwrap().clone();
        let group = terms(&ui, at);
        assert!(open(&ui, at, &card, &group));
        assert_eq!(title(&group), "Add 1 Vocabulary term");
        assert_eq!(named(&card, &group), "Kubernetes");
        // Folded by hand, it stays so.
        ui.assistant.terms_flipped.push(at);
        assert!(!open(&ui, at, &card, &group));
    }

    /// The fold changes what shows, never what saves: the checked terms go
    /// in term by term, the unchecked one stays out, and Undo takes out
    /// exactly what the plan added.
    #[test]
    fn folded_terms_save_and_undo_term_by_term() {
        let (mut ui, at) = plan();
        let before = ui.settings.vocabulary.len();
        act::apply(&mut ui, at);
        let card = act::card(&ui, at).unwrap().clone();
        assert!(
            matches!(card.decision, Decision::Applied { .. }),
            "{:?}",
            card.decision
        );
        let has = |ui: &Ui, term: &str| ui.settings.vocabulary.iter().any(|e| e.term == term);
        assert_eq!(ui.settings.vocabulary.len(), before + 7);
        assert!(has(&ui, "Kubernetes") && has(&ui, "Tailscale"));
        assert!(!has(&ui, "Supabase"), "the unchecked term stays out");
        assert_eq!(state::plan_applied(&card), (3, 3));
        act::undo(&mut ui, at);
        assert_eq!(ui.settings.vocabulary.len(), before);
        assert!(!has(&ui, "Kubernetes"));
    }

    #[test]
    fn the_rows_title_speaks_every_language() {
        for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
            crate::desktop::i18n::speaking(Some(code), || {
                for english in [
                    "Add 1 Vocabulary term",
                    "Add {} Vocabulary terms",
                    "Add {checked} of {total} Vocabulary terms",
                ] {
                    assert_ne!(tr(english), english, "{code}: {english}");
                }
                let some = Terms {
                    rows: (0..95).collect(),
                    checked: 90,
                };
                let title = title(&some);
                assert!(title.contains("90") && title.contains("95"), "{title}");
                assert!(!title.contains('{'), "{code}: {title}");
            });
        }
    }
}
