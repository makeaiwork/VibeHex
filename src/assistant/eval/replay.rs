//! The app's own heuristics over this machine's real data, printed for the
//! person at this machine to read: what Assistant would be told of the
//! install, what their use shows, the terms setup would offer and how
//! each was heard, the returning setup's opener, and what is not asked
//! again. Read-only: no model is asked, nothing is sent anywhere, nothing
//! is written or changed. Recordings are read only when the person has
//! allowed Assistant to (`assistant.read_recordings`).
//!
//! `scripts/assistant-replay` runs it; `HEX_REPLAY_SUPPORT` names another
//! data folder than this account's.

use std::path::PathBuf;

use crate::app_settings::AppSettings;
use crate::assistant::facts::{self, Fact, Facts, State};
use crate::assistant::recordings::{Allowed, Sources, usage};
use crate::assistant::setup::{self, engines, follow, past, plan, since};
use crate::assistant::{changes::vocab, language};

fn title(title: &str) {
    println!("\n== {title}");
}

/// The setups kept with the conversations, read as they are saved.
fn setups(root: &std::path::Path) -> Vec<past::Setup> {
    std::fs::read(root.join("assistant/conversations.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|store| serde_json::from_value(store["setups"].clone()).ok())
        .unwrap_or_default()
}

#[test]
#[ignore = "reads this machine's own data and prints it; run scripts/assistant-replay"]
fn replay_on_this_machine() {
    let root = match std::env::var_os("HEX_REPLAY_SUPPORT") {
        Some(root) => PathBuf::from(root),
        None => crate::app_paths::support_dir().expect("a data folder"),
    };
    let Ok((settings, _)) = AppSettings::read_from(&root.join("settings.json")) else {
        println!("No settings to read in {}", root.display());
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64);
    let setups = setups(&root);
    // As `facts::gather` reads it, but for the keychain, which a test
    // binary must not ask, and Codex and OpenCode, which only a running
    // VibeHex knows; the screens answer only on an app's main thread.
    let state = State {
        os: std::env::consts::OS,
        installed: Fact::Known(facts::on_disk(&settings)),
        servers: Fact::Known(engines::installed_servers()),
        answered: past::answered_before(&setups, &settings, since::changed_since),
        declined: past::declined(&setups),
        hardware: crate::desktop::hardware::detect(),
        ..State::default()
    };
    let facts = Facts::of(&settings, &state);
    language::within(&settings, || {
        title("What Assistant would be told of this install");
        print!(
            "{}",
            facts::text::knew(&facts, &facts::text::Sent::default())
        );
        println!(
            "(not told to a model: OpenRouter key saved: unknown here; local servers installed: {})",
            match state.servers() {
                [] => "none".into(),
                servers => servers.join(", "),
            }
        );

        title("Setups kept");
        println!("{} kept", setups.len());
        if let Some(last) = since::last(&setups) {
            println!(
                "the last one on {}: {} rows applied, {} declined, {} follow-ups answered",
                since::date(last.at_ms),
                last.applied.len(),
                last.declined.len(),
                last.asked.len()
            );
        }
        title("Answered before, not asked again");
        for ask in &state.answered {
            println!("{}: {:?}", ask.id, ask.chosen);
        }
        println!(
            "{} answered, {} declined lines held",
            state.answered.len(),
            state.declined.len()
        );

        let used = Allowed::from_settings(&settings.assistant).and_then(|allowed| {
            let sources = match std::env::var_os("HEX_REPLAY_SUPPORT") {
                Some(_) => Sources::at(&root),
                None => Sources::here()?,
            };
            let known: Vec<String> = settings
                .vocabulary
                .iter()
                .flat_map(|entry| std::iter::once(&entry.term).chain(&entry.sounds_like))
                .cloned()
                .collect();
            Some(usage::read(&allowed, &sources, &known, now))
        });
        title(
            "What your use shows (aggregates the app works out; a returning setup's model reads these)",
        );
        match &used {
            Some(used) => {
                for line in usage::lines(used) {
                    println!("{line}");
                }
            }
            None => {
                println!("Not read: Assistant may not read your recordings (Assistant › Settings).")
            }
        }

        let terms = used.as_ref().map(since::candidates).unwrap_or_default();
        title("Terms setup would offer for Vocabulary ([x] checked, [ ] unchecked)");
        for term in &terms {
            println!(
                "{} {}{}",
                if term.sure { "[x]" } else { "[ ]" },
                term.term,
                match term.sounds_like.is_empty() {
                    true => String::new(),
                    false => format!("  (heard as: {})", term.sounds_like.join(", ")),
                }
            );
        }
        println!(
            "{} terms, {} checked",
            terms.len(),
            terms.iter().filter(|term| term.sure).count()
        );

        let dictations = used.as_ref().map_or(0, |used| used.dictations);
        let meetings = used.as_ref().map_or(0, |used| used.meetings);
        title("The returning setup's opener");
        if since::returning(&setups, dictations, meetings) {
            let last = since::last(&setups).cloned();
            let since = since::Since {
                releases: since::releases_after(last.as_ref().map_or(0, |last| last.at_ms)),
                last,
                from: None,
                usage: used.clone(),
            };
            println!("{}", since::paragraph(&since));
        } else {
            println!("Setup would open as a first one: no setup kept and little use yet.");
        }

        let answers = since::last(&setups)
            .map(|last| last.answers.clone())
            .unwrap_or_default();
        title("Follow-ups setup would ask now, for the last setup's answers");
        for ask in follow::candidates(&answers, &settings, &state) {
            println!("{}", ask.id);
        }
        if state.hardware.screens.is_none() {
            println!(
                "(the screens are unknown here, so nothing that needs a notch is ruled out; a \
                 running VibeHex knows them)"
            );
        }
        title("The app's own plan for those answers ([x] checked)");
        match plan::with_terms(Vec::new(), &answers, &terms, &settings, &state) {
            Some(card) => {
                for item in &card.items {
                    let checked = if item.included { "[x]" } else { "[ ]" };
                    if item.key == vocab::KEY {
                        let names: Vec<String> = vocab::terms(&item.to)
                            .into_iter()
                            .map(|term| term.term)
                            .collect();
                        println!("{checked} term {}", names.join(", "));
                    } else {
                        println!(
                            "{checked} {} = {}{}",
                            item.key,
                            item.to,
                            if item.yours {
                                "  (you chose the value now)"
                            } else {
                                ""
                            }
                        );
                    }
                }
                println!("{} rows, {} already set", card.items.len(), card.kept.len());
            }
            None => println!("Nothing to change."),
        }
        title("Now, as setup's opening says it");
        println!("{}", setup::now::now(&facts));
    });
}
