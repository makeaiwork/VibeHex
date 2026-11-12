//! The gate's rules, one table per kind of proposal: each row names the
//! rule it holds the gate to.

use serde_json::{Value, json};

use super::*;
use crate::assistant::facts::Fact;
use crate::assistant::setup::Ask;
use crate::desktop::hardware::{Hardware, Pointer, Screens};

/// A verdict's kind, without the words an error carries.
fn kind(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Show => "show",
        Verdict::Unchecked(Held::Yours) => "unchecked: yours",
        Verdict::Unchecked(Held::Shorter) => "unchecked: shorter",
        Verdict::Unchecked(Held::SignedOut) => "unchecked: signed out",
        Verdict::Unchecked(Held::NotReady(_)) => "unchecked: not ready",
        Verdict::Drop(why) => match why {
            Why::Unknown => "drop: unknown",
            Why::Never => "drop: never",
            Why::Invalid(_) => "drop: invalid",
            Why::Elsewhere(_) => "drop: elsewhere",
            Why::Hardware => "drop: hardware",
            Why::Same => "drop: same",
            Why::On => "drop: on",
            Why::Declined => "drop: declined",
            Why::Answered => "drop: answered",
            Why::Lookup => "drop: lookup",
            Why::NotOffered => "drop: not offered",
            Why::Ordinary => "drop: ordinary",
            Why::Has => "drop: has",
        },
    }
}

/// Settings every OS has, as the tables expect them: fillers off, AI
/// cleanup on, history kept a month, notes on Codex, recognition Russian.
fn settings() -> AppSettings {
    let mut settings = AppSettings::default();
    settings.labs.filler_words = false;
    settings.labs.ai_cleanup_server = true;
    settings.labs.picker_downloaded = false;
    settings.meetings.enabled = true;
    settings.meetings.notes.writer = Some("codex:gpt-6.1-terra".into());
    settings.dictation_processing.server.codex_model = "gpt-6.1-terra".into();
    settings.transcription.language = "ru".into();
    let rules = crate::rune_ui::shortcut_rules();
    let month = Map::from_iter([("history_retention".to_owned(), json!("month"))]);
    settings::check(&settings, &month, &rules)
        .expect("a month of history")
        .settings
}

fn signed_in(signed_in: bool) -> State {
    State {
        os: std::env::consts::OS,
        codex_signed_in: Fact::Known(signed_in),
        ..State::default()
    }
}

/// A computer known to have no notch and no extra mouse buttons.
fn plain() -> State {
    State {
        hardware: Hardware {
            screens: Some(Screens {
                built_in: false,
                external: 1,
                notch: false,
            }),
            pointers: Some(vec![Pointer {
                name: "Magic Mouse".into(),
                extra_buttons: false,
            }]),
            model: None,
        },
        ..signed_in(true)
    }
}

fn declined(key: &str, from: Value, to: Value) -> State {
    State {
        declined: vec![past::Line {
            key: key.into(),
            to,
            from,
        }],
        ..signed_in(true)
    }
}

const ALL: [Origin; 3] = [Origin::Answer, Origin::Plan, Origin::Preset];
const FILLERS: &str = "labs.filler_words";
const HISTORY: &str = "history_retention";
const WRITER: &str = "meetings.notes.writer";
const DOWNLOADED: &str = "labs.picker_downloaded";

/// A Labs switch this OS doesn't have, if this build knows one.
fn elsewhere() -> Option<&'static str> {
    settings::kinds()
        .map(|(key, ..)| key)
        .find(|key| key.starts_with("labs.") && settings::unavailable(key).is_some())
}

struct Case {
    rule: &'static str,
    origins: &'static [Origin],
    state: State,
    question: &'static str,
    key: &'static str,
    to: Value,
    own: bool,
    note: Option<&'static str>,
    want: &'static str,
}

fn case(
    rule: &'static str,
    origins: &'static [Origin],
    state: &State,
    key: &'static str,
    to: Value,
    want: &'static str,
) -> Case {
    Case {
        rule,
        origins,
        state: state.clone(),
        question: "",
        key,
        to,
        own: false,
        note: None,
        want,
    }
}

#[test]
fn a_setting_passes_each_rule_in_order() {
    let settings = settings();
    let fresh = signed_in(true);
    let out = signed_in(false);
    let said_no = declined(FILLERS, json!(false), json!(true));
    let moved_since = declined(HISTORY, json!("week"), json!("forever"));
    const PLAN: [Origin; 1] = [Origin::Plan];
    const ANSWER: [Origin; 1] = [Origin::Answer];
    const LISTS: [Origin; 2] = [Origin::Plan, Origin::Preset];
    const ASKED: [Origin; 2] = [Origin::Answer, Origin::Preset];
    let on = json!(true);
    let mut cases = vec![
        case(
            "1: the key exists",
            &ALL,
            &fresh,
            "labs.teleport",
            on.clone(),
            "drop: unknown",
        ),
        case(
            "1: Assistant's own settings",
            &ALL,
            &fresh,
            "assistant.read_recordings",
            on.clone(),
            "drop: never",
        ),
        case(
            "1: a path",
            &ALL,
            &fresh,
            "meetings.output_folder",
            json!("/tmp"),
            "drop: never",
        ),
        case(
            "5: a value a save takes",
            &ALL,
            &fresh,
            "recording_duck_volume",
            json!(1.4),
            "drop: invalid",
        ),
        case(
            "6: the value now",
            &ALL,
            &fresh,
            FILLERS,
            json!(false),
            "drop: same",
        ),
        case(
            "6: the same engine written another way",
            &ALL,
            &fresh,
            WRITER,
            json!("codex"),
            "drop: same",
        ),
        case("a change", &ALL, &fresh, FILLERS, on.clone(), "show"),
        case(
            "7: declined in a setup, in a plan",
            &PLAN,
            &said_no,
            FILLERS,
            on.clone(),
            "drop: declined",
        ),
        case(
            "7: a typed question or the preset asks again",
            &ASKED,
            &said_no,
            FILLERS,
            on.clone(),
            "show",
        ),
        case(
            "7: declined, and the setting moved since",
            &PLAN,
            &moved_since,
            HISTORY,
            json!("forever"),
            "unchecked: yours",
        ),
        case(
            "9: a shorter retention, in a plan",
            &LISTS,
            &fresh,
            HISTORY,
            json!("day"),
            "unchecked: shorter",
        ),
        case(
            "9: a shorter retention in an answer is its card's own warning",
            &ANSWER,
            &fresh,
            HISTORY,
            json!("day"),
            "show",
        ),
        case(
            "9: Codex, signed out",
            &LISTS,
            &out,
            "dictation_processing.server.runs_on",
            json!("codex"),
            "unchecked: signed out",
        ),
        case(
            "9: Codex, signed in",
            &LISTS,
            &fresh,
            "dictation_processing.server.runs_on",
            json!("codex"),
            "show",
        ),
        case(
            "9: a value the person chose, in a plan",
            &PLAN,
            &fresh,
            HISTORY,
            json!("forever"),
            "unchecked: yours",
        ),
        case(
            "9: the preset and a typed question are asked for",
            &ASKED,
            &fresh,
            HISTORY,
            json!("forever"),
            "show",
        ),
        Case {
            own: true,
            ..case(
                "9: the person's own answer this round",
                &PLAN,
                &fresh,
                HISTORY,
                json!("forever"),
                "show",
            )
        },
        Case {
            note: Some("Start the OpenCode app first"),
            ..case(
                "9: an engine the rules know isn't ready",
                &LISTS,
                &fresh,
                FILLERS,
                on.clone(),
                "unchecked: not ready",
            )
        },
        Case {
            question: "Which models have I downloaded?",
            ..case(
                "8: a way to look, for a question that asks no change",
                &ANSWER,
                &fresh,
                DOWNLOADED,
                on.clone(),
                "drop: lookup",
            )
        },
        Case {
            question: "Show only downloaded models in the picker",
            ..case(
                "8: the same switch, asked for",
                &ANSWER,
                &fresh,
                DOWNLOADED,
                on.clone(),
                "show",
            )
        },
        Case {
            question: "Which models have I downloaded?",
            ..case(
                "8: a plan's line is no answer to a question",
                &PLAN,
                &fresh,
                DOWNLOADED,
                on.clone(),
                "show",
            )
        },
    ];
    if let Some(key) = elsewhere() {
        cases.push(case(
            "3: a Labs switch of another OS",
            &ALL,
            &fresh,
            key,
            on.clone(),
            "drop: elsewhere",
        ));
    }
    for key in ["labs.notch_indicator", "labs.mouse_shortcuts"] {
        cases.push(case(
            "4: no notch, no extra buttons",
            &ALL,
            &plain(),
            key,
            on.clone(),
            "drop: hardware",
        ));
        cases.push(case(
            "4: hardware not known",
            &ANSWER,
            &fresh,
            key,
            on.clone(),
            "show",
        ));
    }
    for case in cases {
        // A table row about a switch this OS lacks proves nothing here.
        if settings::unavailable(case.key).is_some() && case.want != "drop: elsewhere" {
            continue;
        }
        for origin in case.origins {
            let gate = Gate::new(&settings, &case.state).asked(case.question);
            let offer = Offer {
                key: case.key,
                to: &case.to,
                own: case.own,
                note: case.note,
            };
            let verdict = gate.setting(*origin, offer);
            assert_eq!(
                kind(&verdict),
                case.want,
                "{} ({origin:?}): {} = {}",
                case.rule,
                case.key,
                case.to
            );
        }
    }
}

/// Rule 2: the preset asked for where it isn't offered gets words alone,
/// whatever a model adds: a setting, a term, an edit.
#[test]
fn the_preset_where_it_is_not_offered_makes_no_card() {
    let state = signed_in(true);
    let russian = settings();
    let mut english = settings();
    english.transcription.language = "en".into();
    let term = Term {
        term: "Kubernetes".into(),
        sounds_like: vec!["кубернетис".into()],
    };
    for (settings, question, want) in [
        (&english, "Set it up like Denis", "drop: not offered"),
        (&english, "Turn on filler words", "show"),
        (&russian, "Set it up like Denis", "show"),
    ] {
        let gate = Gate::new(settings, &state).asked(question);
        let setting = gate.setting(Origin::Answer, Offer::of(FILLERS, &json!(true)));
        assert_eq!(kind(&setting), want, "{question}");
        assert_eq!(kind(&gate.term(Origin::Answer, &term, false)), want);
        assert_eq!(kind(&gate.edit(Origin::Answer)), want);
        // Setup's plan is not that question's answer.
        let plan = gate.setting(Origin::Plan, Offer::of(FILLERS, &json!(true)));
        assert_eq!(kind(&plan), "show");
    }
}

#[test]
fn a_term_passes_the_term_rule() {
    let mut settings = settings();
    settings.vocabulary = vec![VocabularyEntry {
        term: "Kubernetes".into(),
        sounds_like: Vec::new(),
    }];
    let fresh = signed_in(true);
    let said_no = declined(
        changes::vocab::KEY,
        Value::Null,
        json!([{"term": "Grafana"}]),
    );
    const MODEL: [Origin; 2] = [Origin::Answer, Origin::Plan];
    const PRESET: [Origin; 1] = [Origin::Preset];
    const PLAN: [Origin; 1] = [Origin::Plan];
    const ANSWER: [Origin; 1] = [Origin::Answer];
    const NAMED: &str = "Add Web to my vocabulary";
    // rule, origins, question, own, term, sounds like, verdict
    type Row = (
        &'static str,
        &'static [Origin],
        &'static str,
        bool,
        &'static str,
        &'static [&'static str],
        &'static str,
    );
    let cases: [Row; 15] = [
        (
            "one `hex vocabulary add` takes",
            &ALL,
            "",
            false,
            "  ",
            &[],
            "drop: invalid",
        ),
        (
            "Vocabulary has it, in any case",
            &ALL,
            "",
            false,
            "kubernetes",
            &["кубер"],
            "drop: has",
        ),
        ("a name", &ALL, "", false, "Supabase", &[], "show"),
        (
            "a name with its spellings",
            &ALL,
            "",
            false,
            "Grafana",
            &["графана"],
            "show",
        ),
        ("code", &ALL, "", false, "kubectl", &[], "show"),
        (
            "a mixed-case name of plain words",
            &ALL,
            "",
            false,
            "GitHub",
            &[],
            "show",
        ),
        (
            "a capital alone is no term",
            &MODEL,
            "",
            false,
            "Web",
            &[],
            "drop: ordinary",
        ),
        (
            "nor is Title Case",
            &MODEL,
            "",
            false,
            "New Session",
            &[],
            "drop: ordinary",
        ),
        (
            "nor a spelling that is the word",
            &MODEL,
            "",
            false,
            "Monkey",
            &["monkey"],
            "drop: ordinary",
        ),
        (
            "nor one heard right",
            &MODEL,
            "",
            false,
            "Monkey",
            &["манки"],
            "drop: ordinary",
        ),
        (
            "an ordinary word misheard",
            &MODEL,
            "",
            false,
            "Bolt",
            &["вольт"],
            "show",
        ),
        (
            "the preset's terms are asked for",
            &PRESET,
            "",
            false,
            "Web",
            &[],
            "show",
        ),
        (
            "a follow-up's Yes is the person's",
            &PLAN,
            "",
            true,
            "Web",
            &[],
            "show",
        ),
        (
            "a term the question names",
            &ANSWER,
            NAMED,
            false,
            "Web",
            &[],
            "show",
        ),
        (
            "a plan answers no typed question",
            &PLAN,
            NAMED,
            false,
            "Web",
            &[],
            "drop: ordinary",
        ),
    ];
    for (rule, origins, question, own, term, sounds_like, want) in cases {
        let term = Term {
            term: term.into(),
            sounds_like: sounds_like.iter().map(|&text| text.into()).collect(),
        };
        for origin in origins {
            let gate = Gate::new(&settings, &fresh).asked(question);
            let verdict = gate.term(*origin, &term, own);
            assert_eq!(kind(&verdict), want, "{rule} ({origin:?}): {}", term.term);
        }
    }
    // Declined in an earlier setup: a plan doesn't offer it again; an
    // answer to a typed question does, and so does the person's own Yes.
    let grafana = Term {
        term: "grafana".into(),
        sounds_like: Vec::new(),
    };
    let gate = Gate::new(&settings, &said_no);
    for (origin, own, want) in [
        (Origin::Plan, false, "drop: declined"),
        (Origin::Plan, true, "show"),
        (Origin::Answer, false, "show"),
        (Origin::Preset, false, "show"),
    ] {
        let verdict = gate.term(origin, &grafana, own);
        assert_eq!(kind(&verdict), want, "{origin:?} own {own}");
    }
}

#[test]
fn a_follow_up_question_passes_its_rules() {
    let settings = settings();
    let mut on = settings.clone();
    on.labs.filler_words = true;
    let fresh = signed_in(true);
    let answered = |id: &str| State {
        answered: vec![Ask {
            id: id.into(),
            ..Ask::default()
        }],
        ..signed_in(true)
    };
    let (fillers, terms) = (answered(FILLERS), answered(follow::OWN[0].id));
    let said_no = declined(FILLERS, json!(false), json!(true));
    let own = follow::OWN[0].id;
    let no_terms = declined(
        changes::vocab::KEY,
        Value::Null,
        json!([{"term": "Docker"}]),
    );
    // rule, state, the settings once round one is saved, id, picked, verdict
    let cases: [(&str, &State, &AppSettings, &str, bool, &str); 11] = [
        (
            "a switch that is off",
            &fresh,
            &settings,
            FILLERS,
            false,
            "show",
        ),
        (
            "no such setting",
            &fresh,
            &settings,
            "labs.teleport",
            false,
            "drop: unknown",
        ),
        (
            "never from a card",
            &fresh,
            &settings,
            "assistant.engine",
            false,
            "drop: never",
        ),
        (
            "on once round one is saved",
            &fresh,
            &on,
            FILLERS,
            false,
            "drop: on",
        ),
        (
            "answered before",
            &fillers,
            &settings,
            FILLERS,
            false,
            "drop: answered",
        ),
        (
            "the app's own pick",
            &fillers,
            &settings,
            FILLERS,
            true,
            "show",
        ),
        (
            "declined before",
            &said_no,
            &settings,
            FILLERS,
            false,
            "drop: declined",
        ),
        (
            "any other setting",
            &fresh,
            &settings,
            HISTORY,
            false,
            "show",
        ),
        (
            "the app's own question",
            &fresh,
            &settings,
            own,
            false,
            "show",
        ),
        (
            "its answer before",
            &terms,
            &settings,
            own,
            false,
            "drop: answered",
        ),
        (
            "its terms declined",
            &no_terms,
            &settings,
            own,
            false,
            "drop: answered",
        ),
    ];
    for (rule, state, after, id, picked, want) in cases {
        let verdict = Gate::new(&settings, state).ask(id, after, picked);
        assert_eq!(kind(&verdict), want, "{rule}: {id}");
    }
    let plain = plain();
    for key in ["labs.notch_indicator", "labs.mouse_shortcuts"] {
        let verdict = Gate::new(&settings, &plain).ask(key, &settings, false);
        let want = match settings::unavailable(key) {
            Some(_) => "drop: elsewhere",
            None => "drop: hardware",
        };
        assert_eq!(kind(&verdict), want, "{key}");
    }
}

/// Rule 8's words: what asks for a change, in English and Russian.
#[test]
fn a_question_asks_for_a_change_or_only_asks() {
    for (question, change) in [
        ("Which models have I downloaded?", false),
        ("How many models are there?", false),
        ("Какие модели у меня скачаны?", false),
        ("Где посмотреть статистику?", false),
        ("", false),
        ("Show only downloaded models", true),
        ("Turn on the search in settings", true),
        ("Can you enable history stats?", true),
        ("Покажи только скачанные модели", true),
        ("Включи поиск по настройкам", true),
        ("Хочу сортировку в списке моделей", true),
    ] {
        assert_eq!(lookup::wants_change(question), change, "{question}");
    }
    // Only a switch of what the window shows is a way to look.
    assert!(lookup::only(DOWNLOADED, "Which models are downloaded?"));
    assert!(!lookup::only(FILLERS, "Which models are downloaded?"));
    assert!(!lookup::only(DOWNLOADED, ""));
}

/// What the turn keeps: a line per proposal with its reason, nothing when
/// nothing was dropped; and which reasons are refusals a model is told.
#[test]
fn the_record_names_each_drop_with_its_reason() {
    assert_eq!(record(&[]), "");
    let drop = |what: &str, why| Dropped {
        what: what.into(),
        why,
    };
    let dropped = [
        drop("labs.picker_downloaded = true", Why::Lookup),
        drop("term Web", Why::Ordinary),
        drop("ask labs.notch_indicator", Why::Hardware),
    ];
    assert_eq!(
        record(&dropped),
        "Proposed and not offered:\n\
         labs.picker_downloaded = true: only a way to look something up\n\
         term Web: an ordinary word heard right, not a term\n\
         ask labs.notch_indicator: this computer can't use it\n"
    );
    let knew = format!("Language: Russian\n{}", record(&dropped));
    assert_eq!(recorded(&knew).len(), 3);
    assert_eq!(recorded(&knew)[1], dropped[1].line());
    assert!(recorded("Language: Russian\n").is_empty());
    assert!(dropped[1].term() && !dropped[0].term());
    for (why, refused) in [
        (Why::Unknown, true),
        (Why::Never, true),
        (Why::Invalid("too long".into()), true),
        (Why::Elsewhere("another OS".into()), false),
        (Why::Hardware, false),
        (Why::Same, false),
        (Why::On, false),
        (Why::Declined, false),
        (Why::Answered, false),
        (Why::Lookup, false),
        (Why::NotOffered, false),
        (Why::Ordinary, false),
        (Why::Has, false),
    ] {
        assert_eq!(why.refusal().is_some(), refused, "{why:?}");
        assert!(!why.words().is_empty());
    }
}
