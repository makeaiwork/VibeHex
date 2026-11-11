use super::*;

fn note(version: &str) -> String {
    format!(
        "---\nversion: {version}\ndate: 2026-11-0{}\nlabs: [files]\nkeys: [ui_language]\n---\n\n# VibeHex {version}\n\n- New: A thing.\n",
        version.len() % 9
    )
}

/// Five releases and Next, as the app would carry them.
fn sample() -> &'static [Note] {
    let mut notes: Vec<Note> = [
        "3.3.0-vibe.1",
        "3.3.0-vibe.2",
        "3.3.0-vibe.3",
        "3.3.0-vibe.10",
        "next",
    ]
    .iter()
    .map(|version| parse(&note(version)))
    .collect();
    notes.sort_by_key(|note| order(&note.version).unwrap_or([u64::MAX; 4]));
    Box::leak(notes.into_boxed_slice())
}

fn running(version: &str, previous: Option<&str>) -> Running {
    Running {
        version: version.into(),
        dev: false,
        previous: previous.map(str::to_owned),
    }
}

fn versions(selection: &Selection) -> Vec<&str> {
    selection
        .notes
        .iter()
        .map(|note| note.version.as_str())
        .collect()
}

#[test]
fn versions_sort_by_their_numbers() {
    let all = sample();
    let names: Vec<&str> = all.iter().map(|note| note.version.as_str()).collect();
    assert_eq!(
        names,
        [
            "3.3.0-vibe.1",
            "3.3.0-vibe.2",
            "3.3.0-vibe.3",
            "3.3.0-vibe.10",
            "next"
        ]
    );
    assert_eq!(order("3.3.0-vibe.10"), Some([3, 3, 0, 10]));
    for odd in [
        "3.3.0",
        "0.0.0-dev-local-abc",
        "3.3.0-vibe.",
        "3.3-vibe.1",
        "next",
    ] {
        assert_eq!(order(odd), None, "{odd}");
    }
}

#[test]
fn an_update_reads_every_release_since_the_one_run_before() {
    let since = select_from(
        sample(),
        &Asked::default(),
        running("3.3.0-vibe.10", Some("3.3.0-vibe.1")),
    );
    assert_eq!(since.why, Why::Since);
    assert_eq!(
        versions(&since),
        ["3.3.0-vibe.10", "3.3.0-vibe.3", "3.3.0-vibe.2"]
    );
    assert_eq!(since.settings_keys(), ["labs.files", "ui_language"]);
}

#[test]
fn without_the_one_before_it_reads_the_one_running() {
    for previous in [
        None,
        Some("3.3.0-vibe.10"),
        Some("3.3.0-vibe.99"),
        Some("garbage"),
    ] {
        let this = select_from(
            sample(),
            &Asked::default(),
            running("3.3.0-vibe.10", previous),
        );
        assert_eq!(versions(&this), ["3.3.0-vibe.10"], "{previous:?}");
        assert_eq!(this.why, Why::Running);
    }
    let older = select_from(sample(), &Asked::default(), running("3.3.0-vibe.2", None));
    assert_eq!(versions(&older), ["3.3.0-vibe.2"]);
}

#[test]
fn a_development_build_reads_next_and_the_latest_release() {
    let dev = Running {
        version: "3.3.0".into(),
        dev: true,
        previous: None,
    };
    let selection = select_from(sample(), &Asked::default(), dev);
    assert_eq!(selection.why, Why::Dev);
    assert_eq!(versions(&selection), ["next", "3.3.0-vibe.10"]);
}

#[test]
fn a_version_without_notes_reads_the_latest() {
    let selection = select_from(sample(), &Asked::default(), running("3.2.0", None));
    assert_eq!(selection.why, Why::Latest);
    assert_eq!(versions(&selection), ["3.3.0-vibe.10"]);
}

#[test]
fn a_named_version_reads_only_it() {
    for named in ["3.3.0-vibe.2", "v3.3.0-vibe.2", "vibe.2", "2"] {
        let asked = Asked {
            version: Some(named.into()),
            history: false,
        };
        let selection = select_from(
            sample(),
            &asked,
            running("3.3.0-vibe.10", Some("3.3.0-vibe.1")),
        );
        assert_eq!(versions(&selection), ["3.3.0-vibe.2"], "{named}");
        assert_eq!(selection.why, Why::Named);
    }
    let asked = Asked {
        version: Some("3.3.0-vibe.42".into()),
        history: false,
    };
    let missing = select_from(sample(), &asked, running("3.3.0-vibe.10", None));
    assert_eq!(missing.why, Why::Latest);
    assert_eq!(missing.named.as_deref(), Some("3.3.0-vibe.42"));
}

#[test]
fn the_history_reads_every_release_newest_first() {
    let asked = Asked {
        version: None,
        history: true,
    };
    let selection = select_from(sample(), &asked, running("3.3.0-vibe.10", None));
    assert_eq!(
        versions(&selection),
        [
            "next",
            "3.3.0-vibe.10",
            "3.3.0-vibe.3",
            "3.3.0-vibe.2",
            "3.3.0-vibe.1"
        ]
    );
}

fn on_disk() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/releases");
    let mut files: Vec<(String, String)> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?.to_owned();
            (stem == NEXT || stem.contains("-vibe."))
                .then(|| (stem, std::fs::read_to_string(&path).unwrap()))
        })
        .collect();
    files.sort();
    files
}

/// Each VibeHex release's notes: named by its version, dated, its Labs
/// features and keys real, short plain bullets.
#[test]
fn every_release_note_is_well_formed() {
    let files = on_disk();
    assert_eq!(notes().len(), files.len(), "every file is carried");
    assert!(files.iter().any(|(name, _)| name == NEXT), "next.md exists");
    let known: Vec<&str> = crate::control::settings::kinds()
        .map(|(key, ..)| key)
        .collect();
    let mut problems = Vec::new();
    for (name, text) in &files {
        let note = parse(text);
        let mut problem = |what: String| problems.push(format!("{name}.md: {what}"));
        if note.version != *name {
            problem(format!("version {:?} isn't its file's name", note.version));
        }
        if name != NEXT && order(name).is_none() {
            problem("isn't named X.Y.Z-vibe.N".into());
        }
        let dated = note.date.len() == 10
            && note.date.bytes().enumerate().all(|(at, byte)| match at {
                4 | 7 => byte == b'-',
                _ => byte.is_ascii_digit(),
            });
        if !(dated || (name == NEXT && note.date == "unreleased")) {
            problem(format!("date {:?} isn't YYYY-MM-DD", note.date));
        }
        for lab in &note.labs {
            if crate::labs::Feature::from_name(lab).is_none() {
                problem(format!("{lab} is no Labs feature"));
            }
        }
        for key in &note.keys {
            if !known.contains(&key.as_str()) {
                problem(format!("{key} isn't a `hex settings` key"));
            }
        }
        if !note.body.starts_with("# VibeHex ") {
            problem("has no `# VibeHex <version>` title".into());
        }
        if text.contains('·') {
            problem("has a middle dot".into());
        }
        if note.body.len() / 4 >= 1_500 {
            problem(format!("about {} tokens", note.body.len() / 4));
        }
        let mut bullets = 0;
        let mut lines = 0;
        for line in note.body.lines().skip(1) {
            if line.starts_with("- ") {
                bullets += 1;
                lines = 1;
            } else if line.starts_with("  ") && bullets > 0 {
                lines += 1;
                if lines > 2 {
                    problem(format!("an item runs past two lines: {line}"));
                }
            }
            let hash = line
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|word| {
                    word.len() >= 7
                        && word.bytes().all(|b| b.is_ascii_hexdigit())
                        && word.bytes().any(|b| b.is_ascii_digit())
                        && word.bytes().any(|b| b.is_ascii_alphabetic())
                });
            if hash || line.contains('#') && line.contains("(#") {
                problem(format!("names a commit or a pull request: {line}"));
            }
        }
        if bullets == 0 {
            problem("lists no changes".into());
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// A tagged VibeHex release has its notes, so Assistant can tell what it
/// brought (`scripts/publish-vibehex.sh notes` writes them before the tag).
#[test]
fn every_release_tag_has_its_notes() {
    let root = env!("CARGO_MANIFEST_DIR");
    let Ok(output) = std::process::Command::new("git")
        .args(["-C", root, "tag", "--list", "v*-vibe.*"])
        .output()
    else {
        return;
    };
    let tags = String::from_utf8_lossy(&output.stdout);
    let missing: Vec<&str> = tags
        .lines()
        .filter_map(|tag| tag.trim().strip_prefix('v'))
        .filter(|version| {
            order(version).is_some() && find(version).is_none_or(|n| n.version != *version)
        })
        .collect();
    assert!(
        missing.is_empty(),
        "tagged without docs/releases/<version>.md: {}",
        missing.join(", ")
    );
}

#[test]
fn a_note_names_the_keys_of_its_labs_then_its_own() {
    let six = find("3.3.0-vibe.6").expect("3.3.0-vibe.6 has notes");
    let keys = six.settings_keys();
    assert!(
        keys.contains(&"labs.files") && keys.contains(&"stop_hotkey"),
        "{keys:?}"
    );
}

/// A question about what's new carries the notes picked, the rule for the
/// answer, and the keys those releases changed, so a small model can offer
/// them on a card.
#[test]
fn the_prompt_carries_the_notes_picked_and_their_keys() {
    use crate::assistant::changes::{self, catalog};
    use crate::assistant::prompt::{Reach, Room, build};
    use crate::assistant::setup::Stage;
    let context = changes::Context {
        settings: crate::app_settings::AppSettings::default(),
        send: true,
        state: catalog::State::default(),
    };
    let selection = select(
        &Asked::default(),
        running("3.3.0-vibe.6", Some("3.3.0-vibe.4")),
    );
    let prompt = build(
        "Что нового?",
        &[],
        Room::Small,
        Reach::Releases(&selection),
        Some(&context),
        &Stage::None,
    );
    assert!(prompt.instructions.contains("For you"));
    assert!(prompt.instructions.contains("hex-change"));
    for version in ["3.3.0-vibe.6", "3.3.0-vibe.5"] {
        assert!(
            prompt
                .input
                .contains(&format!("<release version=\"{version}\"")),
            "{version} missing"
        );
    }
    for version in ["3.3.0-vibe.4", "3.3.0-vibe.3", "Next"] {
        assert!(
            !prompt
                .input
                .contains(&format!("<release version=\"{version}")),
            "{version} isn't since"
        );
    }
    assert!(prompt.input.contains("ran 3.3.0-vibe.4 before it"));
    assert!(prompt.input.contains("Files (macOS)"), "the bullets go in");
    assert!(
        !prompt.input.contains("# VibeHex 3.3.0-vibe.6"),
        "titles stay out"
    );
    let catalog = &prompt.input[prompt.input.find("<changeable_settings").unwrap()..];
    assert!(
        catalog.contains("labs.collapsible_sidebar"),
        "a key the release changed is offered"
    );
    // The guide's question stays the guide's.
    let guide = build(
        "Что нового?",
        &[],
        Room::Small,
        Reach::Guide,
        Some(&context),
        &Stage::None,
    );
    assert!(!guide.input.contains("<releases"));
}

#[test]
fn a_development_build_is_told_so() {
    let dev = Running {
        version: "3.3.0".into(),
        dev: true,
        previous: None,
    };
    let block = prompt::block(&select(&Asked::default(), dev));
    assert!(block.contains("A development build (3.3.0)"));
    assert!(block.contains("<release version=\"Next (not released yet)\" date=\"unreleased\">"));
    let latest = notes().iter().rev().find(|note| !note.is_next()).unwrap();
    assert!(block.contains(&format!("<release version=\"{}\"", latest.version)));
}
