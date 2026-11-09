//! Output › AI cleanup on Codex: whether the Codex CLI is installed and
//! signed in, which of its models cleans up and how hard it thinks first,
//! and how much of the ChatGPT plan's Codex usage is spent. Codex reports
//! all of it; Hex keeps no key ([`cleanup_server::codex`]).

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use rune::{El, Waker, row, text};

use super::super::controls::{
    ButtonKind, CardRow, Choice, DESCRIPTION, METRICS, Paint, button, described_row, segmented,
    select_sized, setting_row,
};
use super::super::{Menu, Ui};
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::dictation::cleanup_server::codex::{self, Account, Limit, Status};

/// The model select's width: GPT-6.1-Sol with its detail.
const MODEL_WIDTH: f32 = 220.0;
/// How often a sign-in in the browser is looked for.
const SIGN_IN_CHECK: Duration = Duration::from_secs(3);
/// How long a sign-in in the browser is looked for.
const SIGN_IN_WAIT: Duration = Duration::from_secs(5 * 60);
const MODEL_MENU: Menu = Menu::Pick("ai-cleanup-codex-model");
/// The reasoning levels Output offers; Codex has more, slower ones.
const EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// What Codex said about itself, loaded while Output shows Codex.
#[derive(Default)]
pub(in crate::rune_ui) struct CodexCleanup {
    status: Reported,
    /// A sign-in opened in the browser, looked for until it lands.
    signing_in: Option<Instant>,
    /// When the status was last asked for while signing in.
    asked: Option<Instant>,
}

#[derive(Default)]
enum Reported {
    #[default]
    Unknown,
    /// The status shown meanwhile, if any.
    Loading(Receiver<Result<Status, String>>, Option<Status>),
    Ready(Status),
    Failed(String),
}

impl Reported {
    fn status(&self) -> Option<&Status> {
        match self {
            Self::Loading(_, status) => status.as_ref(),
            Self::Ready(status) => Some(status),
            _ => None,
        }
    }
}

impl CodexCleanup {
    pub(super) fn busy(&self) -> bool {
        matches!(self.status, Reported::Loading(..)) || self.signing_in.is_some()
    }
}

/// The rows after Runs on while Codex is chosen.
pub(super) fn rows(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Vec<CardRow> {
    let mut rows: Vec<CardRow> = vec![status_row(ui, p).into()];
    let Some(status) = ui.output.cleanup.codex.status.status() else {
        return rows;
    };
    if status.account == Account::SignedOut {
        return rows;
    }
    rows.push(model_row(ui, p, status, layer).into());
    rows.push(effort_row(ui, p).into());
    rows.push(wait_row(ui, p).into());
    if !status.limits.is_empty() || status.limited {
        rows.push(usage_row(p, status).into());
    }
    rows
}

/// The note under the section: what Codex receives.
pub(super) fn note(p: Paint) -> El {
    text(tr(
        "Codex gets only the dictation and your instructions: no tools, no files, no commands. Nothing lands in its history.",
    ))
    .key("ai-cleanup-codex-note")
    .font(DESCRIPTION)
    .color(p.muted())
    .wrap()
    .mx(METRICS.row_px)
}

fn status_row(ui: &Ui, p: Paint) -> El {
    let cleanup = &ui.output.cleanup;
    let tested = super::ai_cleanup::test_result(ui);
    let (description, color, control): (String, _, Option<El>) = match &cleanup.codex.status {
        Reported::Unknown | Reported::Loading(_, None) => {
            (tr("Looking for Codex…").into(), p.muted(), None)
        }
        Reported::Failed(why) if why == codex::NOT_INSTALLED => (
            tr_fill(
                "Not found. Hex uses the Codex CLI on this computer: install it with {}, then sign in once",
                codex::INSTALL_COMMAND,
            ),
            p.warning(),
            Some(
                row()
                    .key("controls")
                    .gap(8.0)
                    .child(button(
                        p,
                        "ai-cleanup-codex-copy",
                        tr("Copy command"),
                        ButtonKind::Secondary,
                        |_, tx| tx.copy(codex::INSTALL_COMMAND.to_owned()),
                    ))
                    .child(check_again(p)),
            ),
        ),
        Reported::Failed(why) => (why.clone(), p.danger(), Some(check_again(p))),
        Reported::Ready(status) | Reported::Loading(_, Some(status)) => match &status.account {
            Account::SignedOut if cleanup.codex.signing_in.is_some() => (
                tr("Finish signing in in your browser").into(),
                p.muted(),
                Some(check_again(p)),
            ),
            Account::SignedOut => (
                tr("Not signed in. Signing in opens your browser; Hex never sees your password")
                    .into(),
                p.warning(),
                Some(button(
                    p,
                    "ai-cleanup-codex-sign-in",
                    tr("Sign in with ChatGPT"),
                    ButtonKind::Primary,
                    |ui, _| sign_in(ui),
                )),
            ),
            account => match tested {
                Some(Ok(took)) => (
                    tr_fill("Answered in {} s", &format!("{:.1}", took.as_secs_f32())),
                    p.success(),
                    Some(super::ai_cleanup::test_button(p)),
                ),
                Some(Err(why)) => (why, p.danger(), Some(super::ai_cleanup::test_button(p))),
                None if super::ai_cleanup::testing(ui) => (tr("Testing…").into(), p.muted(), None),
                None => (
                    signed_in(status, account),
                    p.muted(),
                    Some(super::ai_cleanup::test_button(p)),
                ),
            },
        },
    };
    described_row(
        p,
        "ai-cleanup-status",
        "Codex",
        Some((description, color)),
        control.unwrap_or_else(|| text("").key("ai-cleanup-test")),
    )
}

/// `Codex 0.164.0, signed in with ChatGPT Pro as …`.
fn signed_in(status: &Status, account: &Account) -> String {
    let who = match account {
        Account::ChatGpt { email, plan } if !plan.is_empty() => tr_named(
            "Signed in with ChatGPT {plan} as {email}",
            &[("plan", &capitalized(plan)), ("email", email)],
        ),
        Account::ChatGpt { email, .. } => tr_fill("Signed in with ChatGPT as {}", email),
        Account::ApiKey => tr("Uses an OpenAI API key").into(),
        _ => tr("Signed in").into(),
    };
    if status.version.is_empty() {
        who
    } else {
        format!("Codex {}. {who}", status.version)
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn check_again(p: Paint) -> El {
    button(
        p,
        "ai-cleanup-codex-check",
        tr("Check again"),
        ButtonKind::Secondary,
        |ui, _| ui.output.cleanup.codex.status = Reported::Unknown,
    )
}

fn model_row(ui: &Ui, p: Paint, status: &Status, layer: &mut Vec<El>) -> El {
    let saved = ui.settings.dictation_processing.server.codex_model.trim();
    let chosen = status
        .models
        .iter()
        .find(|model| model.id == saved)
        .or_else(|| status.models.iter().find(|model| model.default));
    let label = chosen.map_or(saved, |model| model.name.as_str());
    let label = if label.is_empty() {
        tr("Codex's default")
    } else {
        label
    };
    let description = chosen
        .map(|model| model.description.clone())
        .unwrap_or_else(|| {
            tr("The list comes from Codex: a new model shows up here by itself").into()
        });
    let choices = status
        .models
        .iter()
        .map(|model| {
            let id = model.id.clone();
            Choice {
                label: model.name.clone(),
                detail: model.default.then(|| tr("Codex's default").to_owned()),
                selected: chosen.is_some_and(|chosen| chosen.id == model.id),
                choose: Box::new(move |ui: &mut Ui| {
                    ui.open = None;
                    let id = id.clone();
                    if ui.settings.dictation_processing.server.codex_model != id {
                        ui.update(|settings| settings.dictation_processing.server.codex_model = id);
                    }
                }),
            }
        })
        .collect();
    let select = select_sized(
        p,
        "ai-cleanup-codex-model",
        tr("Model"),
        label,
        None,
        ui.open == Some(MODEL_MENU),
        choices,
        |ui: &mut Ui| super::super::settings::toggle_menu(ui, MODEL_MENU),
        layer,
        MODEL_WIDTH,
    );
    setting_row(
        p,
        "ai-cleanup-codex-model-row",
        tr("Model"),
        Some(&description),
        select,
    )
}

fn effort_row(ui: &Ui, p: Paint) -> El {
    let saved = ui.settings.dictation_processing.server.codex_effort.trim();
    let selected = EFFORTS
        .iter()
        .position(|effort| *effort == saved)
        .unwrap_or(0);
    setting_row(
        p,
        "ai-cleanup-codex-effort",
        tr("Reasoning"),
        Some(tr(
            "Low answers soonest. Higher levels think longer before they answer",
        )),
        segmented(
            p,
            "ai-cleanup-codex-effort-picker",
            &[tr("Low"), tr("Medium"), tr("High")],
            selected,
            |ui: &mut Ui, index| {
                let effort = EFFORTS[index.min(EFFORTS.len() - 1)];
                if ui.settings.dictation_processing.server.codex_effort != effort {
                    ui.update(|settings| {
                        settings.dictation_processing.server.codex_effort = effort.into()
                    });
                }
            },
        ),
    )
}

/// The waits Output offers, in seconds; settings may hold any other.
const WAITS: [u64; 3] = [30, 60, 120];

fn wait_row(ui: &Ui, p: Paint) -> El {
    let wait = ui
        .settings
        .dictation_processing
        .server
        .codex_wait()
        .as_secs();
    let selected = WAITS
        .iter()
        .position(|seconds| *seconds == wait)
        .unwrap_or(1);
    setting_row(
        p,
        "ai-cleanup-codex-wait",
        tr("Wait for Codex"),
        Some(tr(
            "How long a dictation waits for Codex before it pastes as heard",
        )),
        segmented(
            p,
            "ai-cleanup-codex-wait-picker",
            &[tr("30 s"), tr("1 min"), tr("2 min")],
            selected,
            |ui: &mut Ui, index| {
                let seconds = WAITS[index.min(WAITS.len() - 1)];
                if ui
                    .settings
                    .dictation_processing
                    .server
                    .codex_wait()
                    .as_secs()
                    != seconds
                {
                    ui.update(|settings| {
                        settings.dictation_processing.server.codex_wait_seconds = seconds
                    });
                }
            },
        ),
    )
}

fn usage_row(p: Paint, status: &Status) -> El {
    let windows = status.limits.iter().map(|limit| {
        let color = if limit.used_percent >= 90.0 {
            p.warning()
        } else {
            p.text()
        };
        row()
            .key(format!("window-{}", limit.window_minutes))
            .gap(6.0)
            .center_y()
            .child(
                text(window_name(limit))
                    .key("name")
                    .font(DESCRIPTION)
                    .color(p.muted()),
            )
            .child(
                text(format!("{:.0}%", limit.used_percent))
                    .key("used")
                    .font(p.k.ty.ui)
                    .color(color),
            )
    });
    let description = if status.limited {
        tr("The limit is reached. Until it resets, dictations paste as heard")
    } else {
        tr("Shared with Codex itself. A dictation spends very little of it")
    };
    setting_row(
        p,
        "ai-cleanup-codex-usage",
        tr("Plan usage"),
        Some(description),
        row().key("windows").gap(16.0).center_y().children(windows),
    )
}

fn window_name(limit: &Limit) -> String {
    match limit.window_minutes {
        10080 => tr("This week").into(),
        1440 => tr("Today").into(),
        minutes if minutes % 60 == 0 => tr_fill("{} hours", &(minutes / 60).to_string()),
        minutes => tr_fill("{} min", &minutes.to_string()),
    }
}

/// Opens the ChatGPT sign-in page Codex offers, then looks for the account.
fn sign_in(ui: &mut Ui) {
    if !ui.live() {
        return;
    }
    let waker: Option<Waker> = ui.modes.waker.clone();
    let _ = std::thread::Builder::new()
        .name("hex-codex-sign-in".into())
        .spawn(move || {
            match codex::sign_in(&AtomicBool::new(false)) {
                Ok(url) => {
                    if let Err(error) = crate::desktop::opencode_catalog::open_url(&url) {
                        tracing::warn!(%error, "couldn't open the Codex sign-in page");
                    }
                }
                Err(error) => tracing::warn!(%error, "Codex sign-in didn't start"),
            }
            if let Some(waker) = &waker {
                waker();
            }
        });
    ui.output.cleanup.codex.signing_in = Some(Instant::now());
    ui.output.cleanup.codex.asked = Some(Instant::now());
}

/// Loads the status while Output shows Codex, and again while a sign-in is
/// awaited; true when what shows changed.
pub(super) fn poll(ui: &mut Ui) -> bool {
    let state = &mut ui.output.cleanup.codex;
    if let Reported::Loading(receiver, _) = &state.status {
        let finished = match receiver.try_recv() {
            Ok(Ok(status)) => {
                if status.account != Account::SignedOut {
                    state.signing_in = None;
                }
                Reported::Ready(status)
            }
            Ok(Err(why)) => Reported::Failed(why),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Reported::Unknown,
        };
        state.status = finished;
        return true;
    }
    if state
        .signing_in
        .is_some_and(|since| since.elapsed() >= SIGN_IN_WAIT)
    {
        state.signing_in = None;
    }
    let awaiting_sign_in = state.signing_in.is_some()
        && state
            .asked
            .is_none_or(|asked| asked.elapsed() >= SIGN_IN_CHECK);
    let unknown = matches!(state.status, Reported::Unknown);
    if !(unknown || awaiting_sign_in)
        || !ui.settings.labs.ai_cleanup_server
        || ui.settings.dictation_processing.server.runs_on
            != crate::app_settings::CleanupRunsOn::Codex
        || !ui.live()
        || !ui.window.as_ref().is_some_and(rune::AppHandle::is_visible)
    {
        return false;
    }
    let shown = match std::mem::take(&mut ui.output.cleanup.codex.status) {
        Reported::Ready(status) => Some(status),
        _ => None,
    };
    let (sender, receiver) = channel();
    let waker: Option<Waker> = ui.modes.waker.clone();
    let started = std::thread::Builder::new()
        .name("hex-codex-status".into())
        .spawn(move || {
            let _ = sender
                .send(codex::status(&AtomicBool::new(false)).map_err(|error| error.to_string()));
            if let Some(waker) = &waker {
                waker();
            }
        });
    let state = &mut ui.output.cleanup.codex;
    state.asked = Some(Instant::now());
    state.status = match started {
        Ok(_) => Reported::Loading(receiver, shown),
        Err(error) => Reported::Failed(error.to_string()),
    };
    true
}

/// Codex signed in on a Pro plan, with two models and some usage.
pub(super) fn fixture(state: &mut CodexCleanup) {
    let model = |id: &str, name: &str, description: &str, default: bool| codex::Model {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        efforts: EFFORTS.iter().map(|effort| (*effort).into()).collect(),
        default_effort: "low".into(),
        default,
    };
    state.status = Reported::Ready(Status {
        executable: "/opt/homebrew/bin/codex".into(),
        version: "0.164.0".into(),
        account: Account::ChatGpt {
            email: "you@example.com".into(),
            plan: "pro".into(),
        },
        models: vec![
            model(
                "gpt-6.1-sol",
                "GPT-6.1-Sol",
                "Latest workhorse model for coding and everyday work",
                true,
            ),
            model(
                "gpt-6-luna",
                "GPT-6-Luna",
                "Fast and affordable model for easier tasks",
                false,
            ),
        ],
        limits: vec![
            Limit {
                window_minutes: 300,
                used_percent: 23.0,
                resets_at: None,
            },
            Limit {
                window_minutes: 10080,
                used_percent: 41.0,
                resets_at: None,
            },
        ],
        limited: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_plan_windows() {
        let limit = |window_minutes| Limit {
            window_minutes,
            used_percent: 0.0,
            resets_at: None,
        };
        assert_eq!(window_name(&limit(300)), "5 hours");
        assert_eq!(window_name(&limit(10080)), "This week");
        assert_eq!(capitalized("pro"), "Pro");
    }

    #[test]
    fn the_fixture_chooses_the_default_model() {
        let mut state = CodexCleanup::default();
        fixture(&mut state);
        let status = state.status.status().unwrap();
        assert!(status.models.iter().any(|model| model.default));
    }
}
