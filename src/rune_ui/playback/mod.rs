//! Labs › Listen to the recording, in the window (variant D of the
//! playback board): Play leads the quick requests over a file's or a
//! meeting's field; pressed, it unfolds into an oval player in the chips'
//! place (×, Play or Pause with the time, back 5 s, the scrub, the length,
//! the speed), and × folds it back with the chips. The transcript follows
//! the sound: the line playing lights up, and a click on a line plays from
//! there, or, while paused, marks where Play starts. Space plays or pauses
//! and ← → move 5 s while the player shows.
//!
//! Transcripts keep times per line (a file's segment, a meeting's turn), not
//! per word, so a line is what lights up and what a click picks.

mod bar;
mod lines;

pub(super) use bar::{PLAY_ICON, lead, unfolded};
pub(super) use lines::{file_original, heard, meeting_turn, playing_fill, playing_line};

use super::Ui;
use crate::files::playback::{Audio, Player, Status};

/// How far Back and ← → move.
const SKIP_MS: u64 = 5_000;

/// What plays: a Files record or a meeting, by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Of {
    File(String),
    Meeting(String),
}

/// A scene's player: a position and a length, no sound.
#[derive(Clone, Copy, Debug, Default)]
struct Still {
    at_ms: u64,
    duration_ms: u64,
    playing: bool,
}

#[derive(Default)]
pub(in super::super) struct Playback {
    /// The recording the player belongs to.
    of: Option<Of>,
    player: Option<Player>,
    /// The player shows in the chips' place.
    open: bool,
    /// Where Play starts, picked from the text before anything loaded.
    mark_ms: Option<u64>,
    /// A line was picked while paused: it shows marked until Play.
    marked: bool,
    still: Option<Still>,
    speed: f32,
    /// What the window last drew: playing, the tenth of a second, loaded.
    drawn: (bool, u64, bool),
}

impl Playback {
    /// A scene's player over `of`, unfolded or not.
    pub(super) fn still(of: Of, at_ms: u64, duration_ms: u64, playing: bool, open: bool) -> Self {
        Self {
            of: Some(of),
            open,
            marked: !playing && at_ms > 0,
            still: Some(Still {
                at_ms,
                duration_ms,
                playing,
            }),
            speed: 1.0,
            ..Self::default()
        }
    }

    fn belongs(&self, of: &Of) -> bool {
        self.of.as_ref() == Some(of)
    }

    fn playing(&self) -> bool {
        match (&self.still, &self.player) {
            (Some(still), _) => still.playing,
            (None, Some(player)) => player.playing(),
            _ => false,
        }
    }

    fn at_ms(&self) -> u64 {
        match (&self.still, &self.player) {
            (Some(still), _) => still.at_ms,
            (None, Some(player)) if player.status() == Status::Ready => player.position_ms(),
            _ => self.mark_ms.unwrap_or_default(),
        }
    }

    /// The length, once known.
    fn duration_ms(&self) -> Option<u64> {
        match (&self.still, &self.player) {
            (Some(still), _) => Some(still.duration_ms),
            (None, Some(player)) if player.status() == Status::Ready => Some(player.duration_ms()),
            _ => None,
        }
    }

    fn speed(&self) -> f32 {
        if self.speed > 0.0 { self.speed } else { 1.0 }
    }

    fn status(&self) -> Option<Status> {
        self.player.as_ref().map(Player::status)
    }
}

/// Labs › Listen to the recording is on.
pub(super) fn enabled(ui: &Ui) -> bool {
    crate::labs::Feature::Playback.on(&ui.settings)
}

/// Where `of`'s audio is.
fn audio(ui: &Ui, of: &Of) -> Option<Audio> {
    match of {
        Of::File(id) => ui
            .files
            .record(id)
            .map(|record| Audio::File(record.path.clone())),
        Of::Meeting(id) => crate::meeting::root()
            .ok()
            .map(|root| Audio::Meeting(root.join(id))),
    }
}

/// Moves the player to `of`, dropping another recording's.
fn take(ui: &mut Ui, of: &Of) {
    let pane = &mut ui.playback;
    if !pane.belongs(of) {
        *pane = Playback {
            of: Some(of.clone()),
            speed: pane.speed,
            ..Playback::default()
        };
    }
}

/// Plays `of`, loading it first; from a picked line when there is one.
fn play(ui: &mut Ui, of: &Of) {
    take(ui, of);
    if ui.playback.still.is_some() {
        if let Some(still) = &mut ui.playback.still {
            still.playing = true;
        }
        ui.playback.open = true;
        ui.playback.marked = false;
        return;
    }
    if ui.playback.player.is_none() || matches!(ui.playback.status(), Some(Status::Failed(_))) {
        let Some(audio) = audio(ui, of) else {
            return;
        };
        let waker = ui.modes.waker.clone();
        let player = Player::open(audio, move || {
            if let Some(waker) = &waker {
                waker();
            }
        });
        player.set_speed(ui.playback.speed());
        if let Some(mark) = ui.playback.mark_ms {
            player.seek(mark);
        }
        ui.playback.player = Some(player);
    }
    let pane = &mut ui.playback;
    pane.open = true;
    pane.marked = false;
    if let Some(player) = &pane.player {
        player.play();
    }
}

fn pause(ui: &mut Ui) {
    let pane = &mut ui.playback;
    if let Some(still) = &mut pane.still {
        still.playing = false;
    }
    if let Some(player) = &pane.player {
        player.pause();
    }
}

/// Play or Pause.
fn toggle(ui: &mut Ui, of: &Of) {
    if ui.playback.belongs(of) && ui.playback.playing() {
        pause(ui);
    } else {
        play(ui, of);
    }
}

/// Folds the player back into Play, pausing it where it is.
fn close(ui: &mut Ui) {
    pause(ui);
    ui.playback.open = false;
}

/// Moves to `ms`; with nothing loaded, Play will start there.
fn seek(ui: &mut Ui, ms: u64) {
    let pane = &mut ui.playback;
    let ms = pane.duration_ms().map_or(ms, |length| ms.min(length));
    pane.mark_ms = Some(ms);
    if let Some(still) = &mut pane.still {
        still.at_ms = ms;
    }
    if let Some(player) = &pane.player {
        player.seek(ms);
    }
}

/// A line of `of`'s transcript was clicked: playing goes on from it, and
/// paused, it is where Play starts.
pub(super) fn pick(ui: &mut Ui, of: &Of, ms: u64) {
    take(ui, of);
    seek(ui, ms);
    ui.playback.marked = !ui.playback.playing();
}

fn skip(ui: &mut Ui, forward: bool) {
    let at = ui.playback.at_ms();
    let to = if forward {
        at + SKIP_MS
    } else {
        at.saturating_sub(SKIP_MS)
    };
    seek(ui, to);
}

fn set_speed(ui: &mut Ui, speed: f32) {
    ui.playback.speed = speed;
    if let Some(player) = &ui.playback.player {
        player.set_speed(speed);
    }
}

/// Space plays or pauses and ← → move 5 s while the player shows; a field
/// being typed in keeps its keys.
pub(super) fn on_key(ui: &mut Ui, event: &rune::KeyEvent) -> bool {
    let Some(of) = ui.playback.of.clone() else {
        return false;
    };
    if !ui.playback.open || !enabled(ui) || ui.open.is_some() || !showing(ui, &of) {
        return false;
    }
    let plain = !event.mods.cmd && !event.mods.ctrl && !event.mods.alt;
    match &event.key {
        rune::Key::Text(text) if text == " " && plain => toggle(ui, &of),
        rune::Key::Left if plain => skip(ui, false),
        rune::Key::Right if plain => skip(ui, true),
        _ => return false,
    }
    true
}

/// `of` is the file or meeting on screen.
fn showing(ui: &Ui, of: &Of) -> bool {
    match of {
        Of::File(id) => {
            ui.pane == super::Pane::Files && ui.files.selected.as_deref() == Some(id.as_str())
        }
        Of::Meeting(id) => {
            ui.pane == super::Pane::Meetings && ui.meetings.selected.as_deref() == Some(id.as_str())
        }
    }
}

/// Redraws while it plays; a player whose recording left the screen stops.
pub(super) fn poll(ui: &mut Ui) -> bool {
    let Some(of) = ui.playback.of.clone() else {
        return false;
    };
    if ui.playback.still.is_some() {
        return false;
    }
    if !showing(ui, &of) || !enabled(ui) {
        if ui.playback.player.is_some() {
            ui.playback = Playback {
                speed: ui.playback.speed,
                ..Playback::default()
            };
            return true;
        }
        return false;
    }
    let pane = &mut ui.playback;
    let now = (
        pane.playing(),
        pane.at_ms() / 100,
        pane.status() == Some(Status::Ready),
    );
    let changed = now != pane.drawn;
    pane.drawn = now;
    changed
}
