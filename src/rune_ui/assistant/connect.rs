//! No model to ask: the panel that stands in for the field when the
//! engine Assistant would ask can't answer (OpenCode isn't running, or
//! Codex isn't installed). It offers Codex first, signing in with ChatGPT
//! right here, then OpenCode, then the engines Output › AI cleanup sets up.
//! Choosing one here sets only Assistant's model, never dictation's.
//!
//! Whether Codex is installed and OpenCode runs is looked up off the UI
//! thread while the pane shows: again every few seconds while the panel
//! shows, else once a minute. OpenCode is asked only when Assistant would
//! use it or the panel offers it, and Codex starts only when the person
//! presses Sign in with ChatGPT (or asks a question on it).

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use rune::{El, Tx, col, text};

use super::super::controls::{
    ButtonKind, DESCRIPTION, Paint, button, card, icon_row, icon_row_tagged,
};
use super::super::icons;
use super::super::lists::{Tone, small_button, tone_tag};
use super::super::sidebar::Pane;
use super::super::{Ui, labs};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::codex::{self, Account};
use crate::files::rewrite::Engine;

/// How often the panel looks again while it shows, and otherwise.
const WHILE_SHOWN: Duration = Duration::from_secs(5);
const OTHERWISE: Duration = Duration::from_secs(60);

/// What the pane knows of the engines that need a look.
#[derive(Default)]
pub(in super::super) struct Reach {
    /// Whether Codex is installed; `None` until looked.
    pub(super) codex: Option<bool>,
    /// Whether an OpenCode service answers; `None` until asked.
    pub(super) opencode: Option<bool>,
    looked_at: Option<Instant>,
    looking: Option<Receiver<(bool, Option<bool>)>>,
}

/// Whether the engine the field would ask can't answer: the panel shows
/// in place of the field.
pub(super) fn needed(ui: &Ui) -> bool {
    let (engine, effort) = super::chosen(ui);
    let engine = super::engine(&ui.settings, engine.as_deref(), effort.as_deref());
    unreachable(ui, &engine)
}

/// Whether `engine` can't answer, as last looked: OpenCode isn't running,
/// or Codex isn't installed. Unknown until looked.
pub(super) fn unreachable(ui: &Ui, engine: &Engine) -> bool {
    let reach = &ui.assistant.reach;
    match engine {
        Engine::OpenCode(_) => reach.opencode == Some(false),
        _ if engine.codex_model().is_some() => reach.codex == Some(false),
        _ => false,
    }
}

/// Takes a finished look and starts the next one when due. True when what
/// shows changed.
pub(super) fn poll(ui: &mut Ui) -> bool {
    let mut changed = false;
    let reach = &mut ui.assistant.reach;
    if let Some(looking) = &reach.looking {
        match looking.try_recv() {
            Ok((codex, opencode)) => {
                reach.codex = Some(codex);
                if opencode.is_some() {
                    reach.opencode = opencode;
                }
                reach.looking = None;
                changed = true;
            }
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => reach.looking = None,
        }
    }
    let shown = ui.pane == Pane::Assistant
        && !ui.labs_open
        && ui.assistant.view == super::View::Chat
        && ui.live()
        && ui.window.as_ref().is_some_and(rune::AppHandle::is_visible);
    if !shown {
        return changed;
    }
    let panel = needed(ui);
    let every = if panel { WHILE_SHOWN } else { OTHERWISE };
    if ui
        .assistant
        .reach
        .looked_at
        .is_some_and(|at| at.elapsed() < every)
    {
        return changed;
    }
    let (engine, effort) = super::chosen(ui);
    let engine = super::engine(&ui.settings, engine.as_deref(), effort.as_deref());
    let ask_opencode = panel || matches!(engine, Engine::OpenCode(_));
    let (sender, receiver) = channel();
    let waker = ui.modes.waker.clone();
    let started = std::thread::Builder::new()
        .name("hex-assistant-reach".into())
        .spawn(move || {
            let codex = codex::find().is_some();
            let opencode = ask_opencode.then(crate::desktop::opencode_service::running);
            let _ = sender.send((codex, opencode));
            if let Some(waker) = &waker {
                waker();
            }
        });
    let reach = &mut ui.assistant.reach;
    reach.looked_at = Some(Instant::now());
    if started.is_ok() {
        reach.looking = Some(receiver);
    }
    changed
}

/// The panel: what to connect, Codex first.
pub(super) fn panel(ui: &Ui, p: Paint) -> El {
    let reach = &ui.assistant.reach;
    let macos = ui.platform.caps.os == super::super::platform::Os::Macos;
    let codex = if reach.codex == Some(false) {
        icon_row_tagged(
            p,
            "assistant-codex",
            &rune_theme_opencode::icons::CODE_LINES,
            "Codex",
            Some(tone_tag(p, "tag", tr("Recommended"), Tone::Accent)),
            Some(
                tr_fill(
                    "Not found. VibeHex uses the Codex CLI on this computer: install it with {}, then sign in once",
                    codex::INSTALL_COMMAND,
                )
                .as_str(),
            ),
            small_button(p, "copy", tr("Copy command"), |_, tx| {
                tx.copy(codex::INSTALL_COMMAND.to_owned())
            }),
        )
    } else {
        icon_row_tagged(
            p,
            "assistant-codex",
            &rune_theme_opencode::icons::CODE_LINES,
            "Codex",
            Some(tone_tag(p, "tag", tr("Recommended"), Tone::Accent)),
            Some(tr(
                "Your ChatGPT plan, no API key. Signing in opens your browser",
            )),
            button(
                p,
                "sign-in",
                tr("Sign in with ChatGPT"),
                // Under a conversation, its own card holds the screen's
                // one main action.
                if ui
                    .assistant
                    .conversation()
                    .is_some_and(|open| !open.turns.is_empty())
                {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Primary
                },
                |ui, _| sign_in(ui),
            ),
        )
    };
    let opencode_control = if reach.opencode == Some(true) {
        small_button(p, "use", tr("Use OpenCode"), |ui, _| {
            ui.update(|settings| settings.assistant.engine = Some("opencode".into()));
        })
    } else {
        tone_tag(p, "state", tr("Not running"), Tone::Neutral)
    };
    let mut rows = vec![
        codex,
        icon_row(
            p,
            "assistant-opencode",
            &rune_theme_opencode::icons::CONSOLE,
            "OpenCode",
            Some(tr(
                "Any model you set up in OpenCode, while it runs on this computer",
            )),
            opencode_control,
        ),
        icon_row(
            p,
            "assistant-server",
            &icons::GLOBE,
            tr("OpenRouter or your own server"),
            Some(tr("An API key, or the address of LM Studio or Ollama")),
            small_button(p, "open", tr("Open AI cleanup"), |ui, tx| {
                open_cleanup(ui, tx)
            }),
        ),
    ];
    if macos {
        rows.push(icon_row_tagged(
            p,
            "assistant-built-in",
            &icons::MONITOR,
            tr("Built in"),
            Some(tone_tag(p, "tag", tr("Offline"), Tone::Neutral)),
            Some(tr("A model on this Mac. Slower, and nothing leaves it")),
            small_button(p, "open", tr("Open AI cleanup"), |ui, tx| {
                open_cleanup(ui, tx)
            }),
        ));
    }
    let hint = if macos {
        tr(
            "Not sure? Codex if you have ChatGPT Plus or Pro. Built in if nothing may leave this Mac",
        )
    } else {
        tr("Not sure? Codex if you have ChatGPT Plus or Pro")
    };
    col()
        .key("assistant-connect")
        .pt(28.0)
        .gap(6.0)
        .child(
            text(tr("Connect a model to ask about VibeHex"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(
            text(tr(
                "Assistant answers from the guide built into VibeHex and needs a model to think with",
            ))
            .key("lead")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap(),
        )
        .child(card(p, "engines", rows).mt(12.0))
        .child(
            text(hint)
                .key("hint")
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap()
                .mt(6.0)
                .mx(16.0),
        )
}

/// Assistant on Codex, and Codex signing in when it isn't: the page opens
/// in the browser and Codex finishes the sign-in itself. Hex sees no
/// password.
fn sign_in(ui: &mut Ui) {
    let on_codex = ui
        .settings
        .assistant
        .engine
        .as_deref()
        .is_some_and(|engine| engine.starts_with("codex"));
    if !on_codex {
        ui.update(|settings| settings.assistant.engine = Some("codex".into()));
    }
    start_sign_in(ui);
}

/// Codex signs in, when it isn't: the page opens in the browser and Codex
/// finishes it. Hex sees no password. A card's Sign in with ChatGPT uses
/// it as is, so Assistant's own model stays as it was.
pub(super) fn start_sign_in(ui: &mut Ui) {
    if !ui.live() {
        return;
    }
    let waker = ui.modes.waker.clone();
    let _ = std::thread::Builder::new()
        .name("hex-assistant-codex-sign-in".into())
        .spawn(move || {
            let cancelled = AtomicBool::new(false);
            let signed_in =
                codex::status(&cancelled).is_ok_and(|status| status.account != Account::SignedOut);
            if !signed_in {
                match codex::sign_in(&cancelled) {
                    Ok(url) => {
                        if let Err(error) = crate::desktop::opencode_catalog::open_url(&url) {
                            tracing::warn!(%error, "couldn't open the Codex sign-in page");
                        }
                    }
                    Err(error) => tracing::warn!(%error, "Codex sign-in didn't start"),
                }
            }
            if let Some(waker) = &waker {
                waker();
            }
        });
}

/// Where an API key, a server, or the built-in model is set up: Output ›
/// AI cleanup, or its Labs row while that is off. Dictation's Runs on
/// stays as it is.
pub(super) fn open_cleanup(ui: &mut Ui, tx: &mut Tx) {
    if ui.settings.labs.ai_cleanup_server {
        ui.select_pane(Pane::Output, tx);
    } else {
        let link = labs::LabsLink {
            lab: Some(crate::labs::Feature::AiCleanupServer.name().into()),
            ..Default::default()
        };
        labs::reveal(ui, &link, tx);
    }
}

/// Setup run to its plan with nothing connected (a scene): round one as
/// preselected but dictation cleaned up on Codex, Yes to each round's
/// first follow-up, the app's own plan (Codex signed out: unticked,
/// with its sign-in) ending on where to connect a model, and this panel under it.
pub(in super::super) fn setup_fixture(ui: &mut Ui) {
    use crate::assistant::setup::flow::{self, Step};
    super::fixture::connect(ui);
    super::super::output::codex_signed_out(ui);
    super::ask::begin(ui);
    for _ in 0..8 {
        let Some(open) = ui.assistant.conversation() else {
            return;
        };
        let conversation = open.id;
        match flow::step(&open.turns, ui.assistant.store.setups()) {
            Step::Basics { turn, .. } => {
                let at = super::ask::At { conversation, turn };
                let asks = &open.turns[turn].asks;
                let empty = asks
                    .iter()
                    .position(|ask| ask.id == "use" && ask.chosen.is_empty());
                let cleanup = asks.iter().position(|ask| ask.id == "cleanup");
                if let Some(number) = empty {
                    super::ask::pick(ui, at, number, "dictation_any");
                }
                if let Some(number) = cleanup {
                    super::ask::pick(ui, at, number, "codex");
                }
                super::ask::answered(ui, at, false);
            }
            Step::Follow { turn, .. } => {
                let at = super::ask::At { conversation, turn };
                super::ask::pick(ui, at, 0, "yes");
                super::ask::answered(ui, at, false);
            }
            _ => return,
        }
    }
}
