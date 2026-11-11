//! A question in a meeting's or a file's Requests that reaches past its own
//! text (`assistant::intent::beyond`) also reads the person's other
//! recordings: the Assistant's search (`assistant::recordings`), leaving
//! out the item asked about, within the engine's budget, once the person
//! allowed it. The excerpts go into the request's prompt as numbered other
//! recordings; the answer cites them `[n]` / `[n.m]`, and the turn keeps
//! the places cited and the search's steps (`store::Linked`). Each step
//! reaches the task as it lands, so the chat lists them while it works.

use super::rewrite::Engine;
use super::store::Linked;
use super::worker::update_task;
use crate::assistant::prompt;
use crate::assistant::recordings::{self, Budget, Found, cite};
use crate::assistant::worker::Recall;

/// What a question read past its own text.
#[derive(Debug, Default)]
pub(super) struct Reading {
    /// The other recordings found; none when it read none.
    found: Option<Found>,
    /// What the prompt adds: the excerpts and how to cite them, or that
    /// the person said not now.
    pub(super) prompt: String,
}

impl Reading {
    /// Whether the answer may cite other recordings.
    pub(super) fn cites(&self) -> bool {
        self.found.is_some()
    }

    /// `answer` as the session keeps it, its marks as their excerpt's `[n]`,
    /// with the places it cites and the search's steps.
    pub(super) fn settle(&self, answer: &str) -> (String, Linked) {
        match &self.found {
            Some(found) => (
                cite::plain(answer),
                Linked {
                    refs: found.cited(answer),
                    steps: found.steps.clone(),
                },
            ),
            None => (answer.to_owned(), Linked::default()),
        }
    }
}

/// Reads what `recall` allows for task `task`, on `engine`'s budget (fewer
/// excerpts for a built-in model), each step reaching the task as it lands.
/// A question that only may reach past its text, and found nothing, reads
/// nothing and shows no steps.
pub(super) fn read(task: u64, recall: &Recall, engine: &Engine) -> Reading {
    let search = match recall {
        Recall::Guide => return Reading::default(),
        Recall::Declined => {
            return Reading {
                found: None,
                prompt: prompt::LINKED_DECLINED.to_owned(),
            };
        }
        Recall::Search(search) => search,
    };
    let budget = if engine.is_local() {
        Budget::SMALL
    } else {
        Budget::LARGE
    };
    let found = recordings::search(
        &search.allowed,
        &search.sources,
        &search.intent,
        &search.vocabulary,
        budget,
        search.now_ms,
        &mut |step| update_task(task, |task| task.steps.push(step.clone())),
    );
    let kept = search.intent.sure || !found.excerpts.is_empty();
    update_task(task, |task| {
        task.searching = false;
        if !kept {
            task.steps.clear();
        }
    });
    if !kept {
        return Reading::default();
    }
    Reading {
        prompt: prompt::linked(&found, search.now_ms),
        found: Some(found),
    }
}

/// A draft as it shows while an answer that cites comes: never a half
/// written `[2.3-`, and lines as their excerpt's `[n]`.
pub(super) fn draft(text: &str) -> String {
    prompt::draft(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::recordings::{Excerpt, Kind, Line, Step};

    fn found() -> Found {
        Found {
            excerpts: vec![Excerpt {
                n: 1,
                kind: Kind::Meeting,
                id: "1700-a".into(),
                title: "Budget".into(),
                at_ms: 10,
                offset_ms: Some(0),
                label: "Them".into(),
                text: "The limit holds.".into(),
                lines: vec![Line {
                    text: "The limit holds.".into(),
                    start_ms: 0,
                    end_ms: 900,
                    at: 0..16,
                }],
            }],
            steps: vec![Step::Meetings {
                searched: 4,
                found: 1,
            }],
        }
    }

    #[test]
    fn an_answer_keeps_the_places_it_cites_and_shows_their_numbers() {
        let reading = Reading {
            found: Some(found()),
            prompt: String::new(),
        };
        let (answer, linked) = reading.settle("Last time the limit held [1.1]. [7]");
        assert_eq!(answer, "Last time the limit held [1]. [7]");
        assert_eq!(linked.refs.len(), 1);
        assert_eq!(linked.refs[0].lines[0].at, 0..16);
        assert_eq!(linked.steps.len(), 1);
        // Nothing read: the answer stays as written, brackets and all.
        let (answer, linked) = Reading::default().settle("Item [1.1]");
        assert_eq!(answer, "Item [1.1]");
        assert_eq!(linked, Linked::default());
        assert_eq!(draft("So [1.1] and [1.").trim_end(), "So [1] and");
    }

    #[test]
    fn not_now_tells_the_model_and_reads_nothing() {
        let engine = Engine::resolve(None, &Default::default(), false, &Default::default());
        let reading = read(0, &Recall::Declined, &engine);
        assert!(!reading.cites());
        assert_eq!(reading.prompt, prompt::LINKED_DECLINED);
        assert!(read(0, &Recall::Guide, &engine).prompt.is_empty());
    }
}
