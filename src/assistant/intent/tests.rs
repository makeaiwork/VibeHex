use super::*;

/// Tuesday 10 November 2026, noon local.
fn now() -> u64 {
    Day::new(2026, 11, 10).midnight(0).unwrap() + 12 * 3_600_000
}

fn day(year: i64, month: u32, day: u32) -> u64 {
    Day::new(year, month, day).midnight(0).unwrap()
}

fn range(question: &str) -> (Option<u64>, Option<u64>) {
    let intent = read(question, now()).unwrap_or_else(|| panic!("{question} isn't routed"));
    (intent.since_ms, intent.until_ms)
}

#[test]
fn calendar_days_count_both_ways() {
    let day = Day::new(2026, 11, 10);
    assert_eq!(Day::from_days(day.days()), day);
    assert_eq!(day.weekday(), 1, "a Tuesday");
    assert_eq!(day.plus(-10), Day::new(2026, 10, 31));
    assert_eq!(Day::new(2024, 2, 28).plus(1), Day::new(2024, 2, 29));
    assert_eq!(Day::new(2026, 12, 31).plus(1), Day::new(2027, 1, 1));
}

#[test]
fn times_are_read_against_now() {
    assert_eq!(
        range("Что мы решили по бюджету на встречах в ноябре?"),
        (Some(day(2026, 11, 1)), Some(day(2026, 12, 1)))
    );
    assert_eq!(
        range("Summarize my calls with Anna last week"),
        (Some(day(2026, 11, 2)), Some(day(2026, 11, 9)))
    );
    assert_eq!(
        range("What did I dictate about the release yesterday?"),
        (Some(day(2026, 11, 9)), Some(day(2026, 11, 10)))
    );
    assert_eq!(
        range("О чём была встреча в пятницу?"),
        (Some(day(2026, 11, 6)), Some(day(2026, 11, 7)))
    );
    assert_eq!(
        range("What did we decide in the meeting on November 4?"),
        (Some(day(2026, 11, 4)), Some(day(2026, 11, 5)))
    );
    assert_eq!(
        range("Что обсуждали на созвонах в декабре?"),
        (Some(day(2025, 12, 1)), Some(day(2026, 1, 1))),
        "a month still ahead is last year's"
    );
    assert_eq!(
        range("Сведи звонки за последние 3 дня"),
        (Some(day(2026, 11, 8)), Some(day(2026, 11, 11)))
    );
}

#[test]
fn the_last_few_meetings_are_counted() {
    let intent = read("Какие темы повторялись на последних пяти встречах?", now()).unwrap();
    assert_eq!(intent.latest, Some(5));
    assert!(intent.sure && intent.kinds.meetings && !intent.kinds.files);
    assert_eq!(
        read("What did we decide in my last call?", now())
            .unwrap()
            .latest,
        Some(1)
    );
}

#[test]
fn terms_keep_the_topic_and_drop_the_rest() {
    let intent = read(
        "Что мы решили по бюджету на встречах в ноябре? Сведи всё вместе.",
        now(),
    )
    .unwrap();
    assert_eq!(intent.terms, ["бюджет"]);
    let intent = read("Summarize my calls with Anna last week", now()).unwrap();
    assert_eq!(intent.terms, ["anna"]);
    let intent = read(
        "Что обсуждали с Анной на прошлой неделе на созвонах?",
        now(),
    )
    .unwrap();
    assert_eq!(intent.terms, ["анной", "анна"]);
}

#[test]
fn guide_questions_stay_with_the_guide() {
    for question in [
        "Can Zoom calls record by themselves, without asking?",
        "Who writes the notes? Are they good in Russian?",
        "Which speech model is best for Russian?",
        "Can I use my ChatGPT plan to clean up dictation?",
        "How do I record my meetings automatically?",
        "Where are my recordings stored?",
        "Можно ли записывать звонки в Zoom автоматически?",
        "Где хранятся мои записи?",
        "Как надиктовать текст в Telegram?",
        "How do I transcribe a file?",
        "Why does dictation stop after a minute?",
    ] {
        assert_eq!(read(question, now()), None, "{question}");
    }
}

#[test]
fn a_question_that_only_may_be_about_recordings_waits_for_the_search() {
    let intent = read("What did we decide about pricing?", now()).unwrap();
    assert!(!intent.sure);
    assert_eq!(intent.kinds, Kinds::ALL);
    assert_eq!(intent.terms, ["pricing"]);
}
