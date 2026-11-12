//! A returning setup in the pane (`setup::since`): someone who set
//! VibeHex up before, or has used it a while, gets what came since above
//! round one, which stays folded to one row, "Has anything changed in how
//! you use VibeHex?", its answers as they were. Continue goes straight on
//! to the follow-ups and the plan; "Change answers" opens the questions.
//! Their use is read only while Assistant may read recordings.

use rune::{El, col, el, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::ask::At;
use crate::assistant::recordings::{self, usage};
use crate::assistant::setup::since::{self, Since};
use crate::assistant::setup::{self, Ask};
use crate::desktop::i18n::tr;

/// What a returning setup opens with, when setup returns: the last setup,
/// the releases since, and the use, only while Assistant may read
/// recordings.
pub(super) fn since(ui: &Ui) -> Option<Since> {
    let setups = ui.assistant.store.setups();
    let dictations = ui.history.kept(since::USED_DICTATIONS);
    if !since::returning(setups, dictations, ui.meetings.meetings.len()) {
        return None;
    }
    let last = since::last(setups).cloned();
    // Without a setup, the releases since the first version run here
    // (none in a scene).
    let first = (last.is_none() && !ui.is_fixture() && !ui.assistant.fixture)
        .then(|| {
            crate::assistant::releases::history::read()
                .into_iter()
                .next()
        })
        .flatten();
    let after = match (&last, &first) {
        (Some(last), _) => last.at_ms,
        (None, Some(first)) => first.first_ms,
        (None, None) => 0,
    };
    let fixture = ui.assistant.fixture || ui.is_fixture();
    let usage = recordings::Allowed::from_settings(&ui.settings.assistant)
        .filter(|_| !fixture)
        .and_then(|allowed| {
            let sources = recordings::Sources::here()?;
            let known: Vec<String> = ui
                .settings
                .vocabulary
                .iter()
                .flat_map(|entry| std::iter::once(&entry.term).chain(&entry.sounds_like))
                .cloned()
                .collect();
            Some(usage::read(
                &allowed,
                &sources,
                &known,
                ui.assistant.now_ms(),
            ))
        });
    Some(Since {
        last,
        releases: since::releases_after(after),
        from: first.map(|first| first.version),
        usage,
    })
}

/// What the open setup's opening knew came since the last one, for the
/// model.
pub(super) fn facts(ui: &Ui, conversation: u32) -> String {
    ui.assistant
        .store
        .get(conversation)
        .and_then(|open| {
            super::ask::setup_turns(&open.turns)
                .first()
                .map(|turn| turn.facts.clone())
        })
        .unwrap_or_default()
}

/// A returning setup's round one as one row: whether anything changed,
/// the answers as they were, "Change answers" to open them, Continue.
pub(super) fn folded(p: Paint, at: At, asks: &[Ask]) -> El {
    let c = &p.k.c;
    let mut chips = row().key("chips").gap(6.0).flex_wrap();
    for (number, ask) in asks.iter().enumerate() {
        if !setup::shows(asks, ask) || ask.before {
            continue;
        }
        let chosen = ask
            .options
            .iter()
            .enumerate()
            .filter(|(_, option)| ask.chosen.contains(&option.id));
        for (index, option) in chosen {
            let key = format!("q{number}option{index}");
            chips = chips.child(
                super::ask::chip(p, &key, ask, option, true, true)
                    .role("note")
                    .checked(true),
            );
        }
    }
    let go = rune_kit::Button::new(tr("Continue"))
        .variant(rune_kit::ButtonVariant::Neutral)
        .on_press(move |ui: &mut Ui| super::ask::answered(ui, at, false));
    col()
        .key(format!("round{}", at.turn))
        .role("group")
        .label(tr("Setup questions"))
        .gap(10.0)
        .px(14.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .border(p.hair, c.border_base)
        .child(
            text(tr("Has anything changed in how you use VibeHex?"))
                .key("q")
                .font(p.k.ty.body_medium)
                .color(p.text())
                .wrap(),
        )
        .child(chips)
        .child(
            row()
                .key("continue")
                .gap(8.0)
                .center_y()
                .child(super::card::push(
                    p,
                    "change",
                    tr("Change answers"),
                    super::card::Look::Ghost,
                    move |ui, _| ui.assistant.store.unfold(at.conversation, at.turn),
                ))
                .child(el().key("space").grow(1.0))
                .child(super::super::controls::PushSize::build_small_push(go, p.k).key("go")),
        )
}

#[cfg(test)]
mod tests {
    use super::super::ask;
    use crate::assistant::setup::Answer;
    use crate::assistant::setup::past::Setup;
    use crate::assistant::store::Turn;

    fn turns(ui: &super::Ui) -> Vec<Turn> {
        ui.assistant.conversation().unwrap().turns.clone()
    }

    /// A setup finished before: the opening says since when, round one
    /// is folded with the answers as they were, and Continue goes on
    /// without asking it.
    #[test]
    fn a_returning_setup_skips_round_one() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.store.finish(Setup {
            at_ms: 1_795_714_800_000,
            answers: vec![
                Answer {
                    id: "use".into(),
                    chosen: vec!["dictation_any".into(), "meetings".into()],
                    ..Answer::default()
                },
                Answer {
                    id: "cleanup".into(),
                    chosen: vec!["none".into()],
                    ..Answer::default()
                },
            ],
            ..Setup::default()
        });
        ask::begin(&mut ui);
        let opening = turns(&ui)[0].clone();
        assert!(opening.folded);
        assert!(opening.since.contains("2026-11-26"), "{}", opening.since);
        assert!(
            opening
                .facts
                .contains("answers then: use: dictation_any, meetings")
        );
        let chosen = |id: &str| {
            opening
                .asks
                .iter()
                .find(|ask| ask.id == id)
                .unwrap()
                .chosen
                .clone()
        };
        assert_eq!(chosen("use"), ["dictation_any", "meetings"]);
        assert_eq!(chosen("cleanup"), ["none"]);
        let at = ask::At {
            conversation: ui.assistant.open.unwrap(),
            turn: 0,
        };
        ask::answered(&mut ui, at, false);
        let after = turns(&ui);
        assert!(after[0].asks.iter().all(|ask| ask.done));
        assert!(after.len() > 1, "nothing came after round one");
        assert!(
            after[1..]
                .iter()
                .all(|turn| turn.asks.iter().all(|ask| ask.id != "use"))
        );
    }

    /// A plan put off still finishes the setup: it is kept, and the next
    /// setup returns.
    #[test]
    fn a_decided_plan_is_kept_and_the_next_setup_returns() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.meetings.meetings.clear();
        ui.history.entries.clear();
        ask::begin(&mut ui);
        assert!(!turns(&ui)[0].folded);
        let conversation = ui.assistant.open.unwrap();
        for _ in 0..6 {
            let open = turns(&ui);
            let last = open.len() - 1;
            if !open[last].cards.is_empty() {
                break;
            }
            let at = ask::At {
                conversation,
                turn: last,
            };
            ask::answered(&mut ui, at, last > 0);
        }
        let open = turns(&ui);
        let turn = open.len() - 1;
        assert!(!open[turn].answers.is_empty(), "no plan");
        let at = super::super::changes::At {
            conversation,
            turn,
            card: 0,
        };
        super::super::changes::decline(&mut ui, at);
        assert_eq!(ui.assistant.store.setups().len(), 1);
        // Decided once, kept once.
        super::super::changes::decline(&mut ui, at);
        assert_eq!(ui.assistant.store.setups().len(), 1);
        ui.assistant.open = None;
        ask::begin(&mut ui);
        assert!(ui.assistant.conversation().unwrap().turns[0].folded);
    }

    /// The terms the use shows wait for the plan: no card at the opening,
    /// then in the plan the sure one checked and the other unchecked.
    #[test]
    fn the_uses_terms_come_with_the_plan() {
        use crate::assistant::changes::vocab;
        use crate::assistant::recordings::usage::{Term, Usage};
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        let term = |word: &str, fixed: usize| Term {
            word: word.into(),
            count: 4,
            heard: vec!["гранола".into()],
            fixed,
        };
        let usage = Usage {
            dictations: 80,
            last_30_days: 80,
            terms: vec![term("Granola", 3), term("Outkit", 1)],
            ..Usage::default()
        };
        let since = super::Since {
            usage: Some(usage),
            ..super::Since::default()
        };
        ask::start(&mut ui, Some(since));
        let opening = turns(&ui)[0].clone();
        assert!(opening.cards.is_empty(), "no terms card at the opening");
        assert!(opening.folded && opening.terms.len() == 2);
        assert!(!opening.since.contains("Vocabulary"), "{}", opening.since);
        let conversation = ui.assistant.open.unwrap();
        for _ in 0..6 {
            let open = turns(&ui);
            let last = open.len() - 1;
            if !open[last].cards.is_empty() {
                break;
            }
            let at = ask::At {
                conversation,
                turn: last,
            };
            ask::answered(&mut ui, at, last > 0);
        }
        let plan = turns(&ui).last().unwrap().cards[0].clone();
        let rows: Vec<(String, bool)> = plan
            .items
            .iter()
            .filter(|item| item.key == vocab::KEY)
            .map(|item| (vocab::label(&item.to), item.included))
            .collect();
        assert_eq!(
            rows,
            [("Granola".to_owned(), true), ("Orukeet".to_owned(), false)]
        );
    }

    /// A fresh install asks round one open, with nothing about since; 50
    /// dictations, or a meeting, make it a returning one.
    #[test]
    fn a_first_setup_asks_round_one() {
        let mut ui = super::super::super::scenes::mac_fixture();
        let meetings = std::mem::take(&mut ui.meetings.meetings);
        ui.history.entries.clear();
        assert!(super::since(&ui).is_none());
        ui.meetings.meetings = meetings;
        assert!(super::since(&ui).is_some() || ui.meetings.meetings.is_empty());
        ui.meetings.meetings.clear();
        ask::begin(&mut ui);
        let opening = turns(&ui)[0].clone();
        assert!(!opening.folded && opening.since.is_empty() && opening.facts.is_empty());
    }
}
