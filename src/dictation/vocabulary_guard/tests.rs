use super::*;

fn entry(term: &str, sounds_like: &[&str]) -> VocabularyEntry {
    VocabularyEntry {
        term: term.into(),
        sounds_like: sounds_like.iter().map(|&text| text.into()).collect(),
    }
}

/// A list in the shapes people keep: a compound name with its split
/// hearings, a short name inside it, a term that is the tail of a longer
/// one, and names heard in Russian.
fn guard() -> Guard {
    Guard::new(&[
        entry("NightOwl", &["Night owl", "Knight Owl"]),
        entry("Owl", &["оул", "оула"]),
        entry("Claude Code", &["клод код", "клот код"]),
        entry("Claude", &["клод"]),
        entry("Codex", &["кодекс"]),
        entry("Auto sync", &["авто синку"]),
        entry("sync", &["синк"]),
        entry("Browser Use", &["браузер юз"]),
        entry("Docker", &["докер"]),
        entry("Playwright", &["плейрайт"]),
        entry("Markdown", &["маркдаун"]),
        entry("GitHub", &["гитхаб"]),
        entry("AGENTS.md", &["агентс мд"]),
        entry("X", &["и"]),
    ])
}

/// What Hex does around cleanup: spell the exact forms, then check the
/// answer against that transcript.
fn checked(heard: &str, cleaned: &str) -> Checked {
    let guard = guard();
    guard.check(&guard.spell(heard), cleaned)
}

#[test]
fn unsupported_terms_are_undone_and_the_rest_of_cleanup_stays() {
    // (heard, cleanup wrote, what stands after the guard)
    let cases = [
        // Another phrase that shares one word with a spelling.
        (
            "ну это найт шифт был тяжёлый",
            "Это NightOwl был тяжёлый.",
            "Это найт шифт был тяжёлый.",
        ),
        (
            "найт шифт, потом отдых, потом найт шифт",
            "NightOwl, NightOwl, NightOwl.",
            "Найт шифт, потом отдых, потом найт шифт.",
        ),
        // A neighbour swallowed: "feed owl" is not the product.
        (
            "скажи feed owl и всё",
            "Скажи NightOwl и всё.",
            "Скажи feed owl и всё.",
        ),
        (
            "покорми оула вечером",
            "Покорми NightOwl вечером.",
            "Покорми Owl вечером.",
        ),
        // A longer term than was said; the shorter term's spelling stands.
        (
            "включи синк на ночь",
            "Включи Auto sync на ночь.",
            "Включи sync на ночь.",
        ),
        (
            "спроси у клода",
            "Спроси у Claude Code.",
            "Спроси у Claude.",
        ),
        (
            "почисти синком папку",
            "Почисти Auto sync папку.",
            "Почисти sync папку.",
        ),
        // An unrelated word.
        (
            "это был клинок, а не нож",
            "Это был Claude, а не нож.",
            "Это был клинок, а не нож.",
        ),
        (
            "юз кейс простой",
            "Browser Use кейс простой.",
            "Юз кейс простой.",
        ),
        ("купи докторскую", "Купи Docker.", "Купи докторскую."),
        (
            "там код и кодекс рядом",
            "Там Codex и Codex рядом.",
            "Там код и Codex рядом.",
        ),
        // A term from nothing, with and without words around it.
        ("сохрани файл", "Сохрани файл. Markdown.", "Сохрани файл."),
        (
            "открой заметки",
            "Открой заметки Markdown.",
            "Открой заметки.",
        ),
        ("", "Markdown.", ""),
    ];
    for (heard, cleaned, expected) in cases {
        let checked = checked(heard, cleaned);
        assert_eq!(
            checked.text, expected,
            "heard {heard:?}, cleaned {cleaned:?}"
        );
        assert!(checked.undone >= 1, "{heard:?}: {checked:?}");
        assert_eq!(checked.kept, 0, "{heard:?}: {checked:?}");
    }
}

#[test]
fn supported_terms_stay_as_cleanup_wrote_them() {
    // (heard, cleanup wrote, terms cleanup wrote that the transcript lacked)
    let cases = [
        // The term said in the other alphabet, unlisted.
        ("запусти плейврайт тесты", "Запусти Playwright тесты.", 1),
        ("открой докир сейчас", "Открой Docker сейчас.", 1),
        ("open night owl now", "Open NightOwl now.", 0),
        // A heard form with a case ending, joined or apart.
        ("в найтоуле всё есть", "В NightOwl всё есть.", 1),
        ("без найт оула никак", "Без NightOwl никак.", 1),
        (
            "работаю с докером и гитхабом",
            "Работаю с Docker и GitHub.",
            2,
        ),
        ("спроси у клод кода", "Спроси у Claude Code.", 1),
        // A listed form with punctuation inside it.
        (
            "запусти клод, код и подожди",
            "Запусти Claude Code и подожди.",
            1,
        ),
        ("это найт-оул, наверное", "Это NightOwl, наверное.", 1),
        // Two terms side by side, each from its own words.
        ("клод код и кодекс", "Claude Code и Codex", 0),
        (
            "сравни клот кода и кодэкс",
            "Сравни Claude Code и Codex.",
            2,
        ),
        ("плейврайт докир рядом", "Playwright Docker рядом.", 2),
        // The model hangs a Russian ending on the name.
        ("в гитхапе лежит", "В GitHub'е лежит.", 1),
        // The term as half of a hyphenated word keeps both halves.
        ("подними докир контейнер", "Подними Docker-контейнер.", 1),
        // Said aloud with its dot.
        ("прочитай агентс точка мд", "Прочитай AGENTS.md.", 1),
        // Fillers and a stutter dropped beside the term are cleanup's work.
        ("ну вот этот докир, короче", "Этот Docker.", 1),
        ("плей плейврайт запусти", "Запусти Playwright.", 1),
        // Cleanup moved the term within the sentence.
        ("в докире открой проект", "Открой проект в Docker.", 1),
    ];
    for (heard, cleaned, written) in cases {
        let checked = checked(heard, cleaned);
        assert_eq!(checked.text, cleaned, "heard {heard:?}: {checked:?}");
        assert_eq!(
            (checked.kept, checked.undone, checked.restored),
            (written, 0, 0),
            "heard {heard:?}: {checked:?}"
        );
    }
}

#[test]
fn a_word_swallowed_beside_a_supported_term_is_put_back() {
    let cases = [
        // The exact form is spelled in code; cleanup then dropped "feed".
        (
            "скажи feed night owl и всё",
            "Скажи NightOwl и всё.",
            "Скажи feed NightOwl и всё.",
        ),
        (
            "не feed owl, а feed knight owl",
            "Не NightOwl, а NightOwl.",
            "Не feed owl, а feed NightOwl.",
        ),
        (
            "открой найтоул панель",
            "Открой NightOwl.",
            "Открой NightOwl панель.",
        ),
        (
            "запусти клод код ревью",
            "Запусти Claude.",
            "Запусти Claude Code ревью.",
        ),
    ];
    for (heard, cleaned, expected) in cases {
        let checked = checked(heard, cleaned);
        assert_eq!(
            checked.text, expected,
            "heard {heard:?}, cleaned {cleaned:?}"
        );
        assert!(checked.restored >= 1, "{heard:?}: {checked:?}");
    }
}

#[test]
fn a_freely_rewritten_answer_is_left_alone() {
    // A mode that translates or rewrites isn't cleanup the guard can line
    // up: the words around the term are the model's own.
    let cases = [
        (
            "исправь ошибку в сборке и перезапусти контейнер",
            "Fix the build error and restart the Docker container.",
        ),
        (
            "напиши письмо про отпуск",
            "Subject: Vacation\n\nI will be away; notes are in Markdown on GitHub, as agreed.",
        ),
    ];
    for (heard, cleaned) in cases {
        let checked = checked(heard, cleaned);
        assert_eq!(checked.text, cleaned, "{checked:?}");
        assert_eq!(checked.undone, 0);
    }
    // A long stretch with no kept word is not judged at all.
    let heard = (0..40)
        .map(|i| format!("слово{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = (0..40)
        .map(|i| format!("word{i}"))
        .collect::<Vec<_>>()
        .join(" ")
        + " Docker";
    let checked = checked(&heard, &cleaned);
    assert_eq!(
        (checked.text.as_str(), checked.unjudged),
        (cleaned.as_str(), 1)
    );
}

#[test]
fn nothing_to_judge_changes_nothing() {
    let guard = guard();
    for (heard, cleaned) in [
        ("ну привет как дела", "Привет, как дела?"),
        ("открой Docker сейчас", "Открой Docker сейчас."),
        ("", ""),
    ] {
        let checked = guard.check(heard, cleaned);
        assert_eq!(checked.text, cleaned);
        assert_eq!(
            (
                checked.kept,
                checked.undone,
                checked.restored,
                checked.unjudged
            ),
            (0, 0, 0, 0)
        );
    }
    let empty = Guard::new(&[]);
    assert_eq!(empty.spell("клод код"), "клод код");
    assert_eq!(empty.check("найт шифт", "NightOwl").text, "NightOwl");
}

#[test]
fn the_cleaned_texts_own_layout_survives_a_repair() {
    let checked = checked(
        "первое найт шифт второе докир",
        "- Первое: NightOwl.\n- Второе: Docker.\n",
    );
    assert_eq!(checked.text, "- Первое: найт шифт.\n- Второе: Docker.\n");
    assert_eq!((checked.kept, checked.undone), (1, 1));
}

#[test]
fn exact_spellings_become_their_term_before_cleanup() {
    let guard = guard();
    let cases = [
        // Whole words, any case, any alphabet; the punctuation around stays.
        (
            "Запусти Клод Код, потом кодекс.",
            "Запусти Claude Code, потом Codex.",
        ),
        ("(докер)", "(Docker)"),
        // The longest spelling wins over a shorter one inside it.
        ("включи авто синку", "включи Auto sync"),
        ("включи синк", "включи sync"),
        // Joined or split, a spelling is its letters; one word that differs
        // from its term only in case or a hyphen is not a mishearing.
        (
            "найтовл и Knightowl и knight owl",
            "найтовл и NightOwl и NightOwl",
        ),
        ("Night-owl", "Night-owl"),
        // Not inside a longer word, and not an inflection the list lacks.
        ("кодексом и докеры", "кодексом и докеры"),
        // Punctuation inside ends a match; only the last word may carry it.
        ("клод, код", "Claude, код"),
        // An address is typed, not misheard.
        ("docs/докер и докер.ру", "docs/докер и докер.ру"),
        // One word that differs from its term only in case is left alone.
        ("nightowl и docker", "nightowl и docker"),
        // A spelling under two characters is ignored.
        ("и так и сяк", "и так и сяк"),
        // Already the term: nothing to do, and a second pass changes nothing.
        ("Claude Code и NightOwl", "Claude Code и NightOwl"),
    ];
    for (heard, expected) in cases {
        let spelled = guard.spell(heard);
        assert_eq!(spelled, expected, "{heard:?}");
        assert_eq!(guard.spell(&spelled), spelled, "second pass over {heard:?}");
    }
}

#[test]
fn the_first_term_listed_owns_a_shared_spelling() {
    let guard = Guard::new(&[
        entry("Barn door", &["барндор"]),
        entry("BarnDoor", &["барндор"]),
    ]);
    assert_eq!(guard.spell("закрой барндор"), "закрой Barn door");
}

/// The guard runs on the dictation path after every cleanup: an ordinary
/// dictation must cost well under a millisecond. Prints what it measured;
/// the bound is loose enough for a loaded debug build.
#[test]
fn an_ordinary_dictation_is_checked_in_well_under_a_millisecond() {
    let vocabulary: Vec<VocabularyEntry> = (0..120)
        .map(|index| {
            entry(
                &format!("Term{index}"),
                &[&format!("терм {index}"), &format!("тёрм{index}")],
            )
        })
        .chain([
            entry("Claude Code", &["клод код", "клот код"]),
            entry("Docker", &["докер"]),
            entry("NightOwl", &["Night owl"]),
        ])
        .collect();
    let started = std::time::Instant::now();
    let guard = Guard::new(&vocabulary);
    let built = started.elapsed();
    let heard = "ну короче я вчера запустил клод код в докире и он сам поправил тесты, \
                 потом я посмотрел найт шифт и увидел что там всё упало из-за сети, \
                 поэтому давай сегодня ещё раз прогоним терм 7 и посмотрим на результат"
        .to_string();
    let cleaned = "Короче, я вчера запустил Claude Code в Docker, и он сам поправил тесты. \
                   Потом я посмотрел NightOwl и увидел, что там всё упало из-за сети, \
                   поэтому давай сегодня ещё раз прогоним Term7 и посмотрим на результат.";
    const ROUNDS: u32 = 200;
    let started = std::time::Instant::now();
    let mut undone = 0;
    for _ in 0..ROUNDS {
        let spelled = guard.spell(&heard);
        undone += guard.check(&spelled, cleaned).undone;
    }
    let each = started.elapsed() / ROUNDS;
    eprintln!("guard built in {built:?}; spell + check of 39 words: {each:?}");
    assert_eq!(undone, ROUNDS as usize);
    assert!(each < std::time::Duration::from_millis(5), "{each:?}");
}
