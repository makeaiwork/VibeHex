//! Labs › Listen to the recording: plays a file's or a meeting's audio so
//! its transcript can be checked against it. One thread per player decodes
//! the audio to 16 kHz mono (what transcription heard), then plays it on
//! the default output through rodio while the window reads the position.
//! Another speed plays at the voice's own pitch (`stretch`).
//!
//! A file plays its source, decoded again through AVFoundation; a meeting
//! plays both its tracks mixed, each placed where it started.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use color_eyre::eyre::{Result, WrapErr, eyre};

/// The rate everything plays at: the rate transcription heard.
pub const RATE: u32 = 16_000;

/// Speeds the player offers.
pub const SPEEDS: [f32; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

/// What a player plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Audio {
    /// A file Files took: its source, wherever it still is.
    File(PathBuf),
    /// A meeting's folder, with `manifest.json` and both tracks.
    Meeting(PathBuf),
}

/// Where a player is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    Failed(String),
}

const LOADING: u8 = 0;
const READY: u8 = 1;
const FAILED: u8 = 2;

/// What the window and the player thread share.
struct Shared {
    status: AtomicU8,
    error: Mutex<Option<String>>,
    /// The next sample to play.
    at: AtomicUsize,
    len: AtomicUsize,
    playing: AtomicBool,
    /// Speed × 100.
    speed: AtomicU32,
    /// The last sample was reached; the thread pauses.
    ended: AtomicBool,
}

enum Command {
    Play,
    Pause,
}

/// A recording, playing or paused. Dropping it stops it.
pub struct Player {
    shared: Arc<Shared>,
    commands: Sender<Command>,
}

impl Player {
    /// Starts decoding `audio`; `wake` is called while it plays and when
    /// it is ready, so the window redraws.
    pub fn open(audio: Audio, wake: impl Fn() + Send + 'static) -> Player {
        let shared = Arc::new(Shared {
            status: AtomicU8::new(LOADING),
            error: Mutex::new(None),
            at: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            playing: AtomicBool::new(false),
            speed: AtomicU32::new(100),
            ended: AtomicBool::new(false),
        });
        let (commands, receiver) = mpsc::channel();
        let thread_shared = shared.clone();
        let spawned = std::thread::Builder::new()
            .name("hex-playback".into())
            .spawn(move || run(&audio, &thread_shared, &receiver, &wake));
        if let Err(error) = spawned {
            fail(&shared, format!("couldn't start the player: {error}"));
        }
        Player { shared, commands }
    }

    pub fn status(&self) -> Status {
        match self.shared.status.load(Ordering::Acquire) {
            READY => Status::Ready,
            FAILED => Status::Failed(
                self.shared
                    .error
                    .lock()
                    .ok()
                    .and_then(|error| error.clone())
                    .unwrap_or_default(),
            ),
            _ => Status::Loading,
        }
    }

    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Acquire)
    }

    pub fn position_ms(&self) -> u64 {
        samples_ms(self.shared.at.load(Ordering::Acquire))
    }

    /// The length, once decoded.
    pub fn duration_ms(&self) -> u64 {
        samples_ms(self.shared.len.load(Ordering::Acquire))
    }

    /// Plays from where it is; from the start once it reached the end.
    pub fn play(&self) {
        if self.shared.ended.swap(false, Ordering::AcqRel)
            || self.shared.at.load(Ordering::Acquire) >= self.shared.len.load(Ordering::Acquire)
        {
            self.shared.at.store(0, Ordering::Release);
        }
        self.shared.playing.store(true, Ordering::Release);
        let _ = self.commands.send(Command::Play);
    }

    pub fn pause(&self) {
        self.shared.playing.store(false, Ordering::Release);
        let _ = self.commands.send(Command::Pause);
    }

    /// Moves to `ms`, playing or not; before it is decoded, playing starts
    /// there.
    pub fn seek(&self, ms: u64) {
        let len = self.shared.len.load(Ordering::Acquire);
        let mut at = ms_samples(ms);
        if len > 0 {
            at = at.min(len);
        }
        self.shared.ended.store(false, Ordering::Release);
        self.shared.at.store(at, Ordering::Release);
    }

    /// The track reads it as it plays.
    pub fn set_speed(&self, speed: f32) {
        let speed = speed.clamp(0.5, 3.0);
        self.shared
            .speed
            .store((speed * 100.0).round() as u32, Ordering::Release);
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.shared.playing.store(false, Ordering::Release);
        // The thread ends when the channel closes.
    }
}

fn samples_ms(samples: usize) -> u64 {
    samples as u64 * 1_000 / u64::from(RATE)
}

fn ms_samples(ms: u64) -> usize {
    (ms * u64::from(RATE) / 1_000) as usize
}

fn fail(shared: &Shared, error: String) {
    if let Ok(mut kept) = shared.error.lock() {
        *kept = Some(error);
    }
    shared.playing.store(false, Ordering::Release);
    shared.status.store(FAILED, Ordering::Release);
}

/// The player thread: decode, then follow the commands until the player is
/// dropped, waking the window while it plays.
fn run(audio: &Audio, shared: &Arc<Shared>, commands: &Receiver<Command>, wake: &dyn Fn()) {
    let samples = match decode(audio) {
        Ok(samples) => Arc::<[i16]>::from(samples),
        Err(error) => {
            tracing::warn!("couldn't read the recording to play it: {error:#}");
            fail(shared, format!("{error:#}"));
            wake();
            return;
        }
    };
    shared.len.store(samples.len(), Ordering::Release);
    let at = shared.at.load(Ordering::Acquire).min(samples.len());
    shared.at.store(at, Ordering::Release);
    shared.status.store(READY, Ordering::Release);
    wake();
    // The output opens on the first Play, so loading never wakes a device.
    let mut output: Option<(rodio::MixerDeviceSink, rodio::Player)> = None;
    loop {
        let command = if shared.playing.load(Ordering::Acquire) {
            match commands.recv_timeout(Duration::from_millis(80)) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match commands.recv() {
                Ok(command) => Some(command),
                Err(_) => return,
            }
        };
        match command {
            Some(Command::Play) => {
                if output.is_none() {
                    match open_output(samples.clone(), shared.clone()) {
                        Ok(opened) => output = Some(opened),
                        Err(error) => {
                            fail(shared, format!("{error:#}"));
                            wake();
                            return;
                        }
                    }
                }
                if let Some((_, player)) = &output {
                    player.play();
                }
            }
            Some(Command::Pause) => {
                if let Some((_, player)) = &output {
                    player.pause();
                }
            }
            None => {}
        }
        if shared.ended.load(Ordering::Acquire)
            && shared.playing.swap(false, Ordering::AcqRel)
            && let Some((_, player)) = &output
        {
            player.pause();
        }
        wake();
    }
}

fn open_output(
    samples: Arc<[i16]>,
    shared: Arc<Shared>,
) -> Result<(rodio::MixerDeviceSink, rodio::Player)> {
    let mut sink = rodio::DeviceSinkBuilder::open_default_sink()
        .map_err(|error| eyre!("The sound output couldn't be opened: {error}"))?;
    sink.log_on_drop(false);
    let player = rodio::Player::connect_new(sink.mixer());
    player.pause();
    player.append(Track {
        samples,
        shared,
        stretch: super::stretch::Stretch::new(),
        pending: std::collections::VecDeque::new(),
        expected: None,
    });
    Ok((sink, player))
}

/// The recording as a rodio source that reads and moves `Shared::at`, so a
/// seek is a store and the position is exact at any speed. At 1× it plays
/// sample by sample; at another speed a hop at a time, stretched. Past the
/// end it plays silence and marks the end for the thread to pause.
struct Track {
    samples: Arc<[i16]>,
    shared: Arc<Shared>,
    stretch: super::stretch::Stretch,
    /// The rest of the last stretched hop.
    pending: std::collections::VecDeque<f32>,
    /// Where the track left `at`; anything else is a seek.
    expected: Option<usize>,
}

impl Iterator for Track {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let at = self.shared.at.load(Ordering::Acquire);
        if self.expected != Some(at) {
            self.pending.clear();
            self.stretch.reset();
        }
        if let Some(sample) = self.pending.pop_front() {
            return Some(sample);
        }
        if at >= self.samples.len() {
            self.shared.ended.store(true, Ordering::Release);
            return Some(0.0);
        }
        let speed = self.shared.speed.load(Ordering::Acquire) as f32 / 100.0;
        let (next, sample) = if (speed - 1.0).abs() < 0.01 {
            self.stretch.reset();
            (at + 1, f32::from(self.samples[at]) / f32::from(i16::MAX))
        } else {
            let mut hop = Vec::with_capacity(super::stretch::HOP);
            let moved = self.stretch.hop(&self.samples, at, speed, &mut hop);
            self.pending.extend(hop);
            let first = self.pending.pop_front().unwrap_or_default();
            ((at + moved).min(self.samples.len()), first)
        };
        // A seek since `at` was read wins.
        let _ = self
            .shared
            .at
            .compare_exchange(at, next, Ordering::AcqRel, Ordering::Acquire);
        self.expected = Some(next);
        Some(sample)
    }
}

impl rodio::Source for Track {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        rodio::ChannelCount::MIN
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        rodio::SampleRate::new(RATE).unwrap_or(rodio::SampleRate::MIN)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn decode(audio: &Audio) -> Result<Vec<i16>> {
    match audio {
        Audio::File(source) => decode_file(source),
        Audio::Meeting(directory) => decode_meeting(directory),
    }
}

/// A file's audio, decoded again as transcription decoded it.
fn decode_file(source: &Path) -> Result<Vec<i16>> {
    let dir = super::store::dir()?.join("playing");
    std::fs::create_dir_all(&dir)?;
    let wav = dir.join(format!("{}.wav", std::process::id()));
    let decoded = super::decode::to_wav(source, &wav).and_then(|_| read_track(&wav));
    let _ = std::fs::remove_file(&wav);
    decoded.map(|samples| to_pcm(&samples))
}

#[derive(serde::Deserialize)]
struct Manifest {
    system: ManifestTrack,
    microphone: ManifestTrack,
}

#[derive(serde::Deserialize)]
struct ManifestTrack {
    file: String,
    first_arrival_us: Option<u64>,
}

/// Both tracks at 16 kHz, each from where it started on the meeting's
/// timeline (the transcript's times count from the earlier one), mixed.
fn decode_meeting(directory: &Path) -> Result<Vec<i16>> {
    let manifest: Manifest = serde_json::from_slice(
        &std::fs::read(directory.join("manifest.json"))
            .wrap_err("the meeting's manifest couldn't be read")?,
    )
    .wrap_err("the meeting's manifest couldn't be read")?;
    let origin = [
        manifest.system.first_arrival_us,
        manifest.microphone.first_arrival_us,
    ]
    .into_iter()
    .flatten()
    .min()
    .unwrap_or_default();
    let mut mixed: Vec<f32> = Vec::new();
    let mut found = 0;
    for track in [&manifest.system, &manifest.microphone] {
        let path = directory.join(&track.file);
        if !path.is_file() {
            continue;
        }
        let samples = read_track(&path)?;
        let offset_ms = track
            .first_arrival_us
            .unwrap_or(origin)
            .saturating_sub(origin)
            / 1_000;
        mix_into(&mut mixed, &samples, ms_samples(offset_ms));
        found += 1;
    }
    if found == 0 {
        return Err(eyre!("This meeting kept no audio."));
    }
    Ok(to_pcm(&mixed))
}

/// Adds `samples` to `mixed` from `offset`, growing it as needed.
fn mix_into(mixed: &mut Vec<f32>, samples: &[f32], offset: usize) {
    let end = offset + samples.len();
    if mixed.len() < end {
        mixed.resize(end, 0.0);
    }
    for (slot, sample) in mixed[offset..end].iter_mut().zip(samples) {
        *slot += sample;
    }
}

/// A WAV's samples, mixed to mono at 16 kHz, read a second at a time so a
/// long recording is never held at its own rate.
fn read_track(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .wrap_err_with(|| format!("{} couldn't be read", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    let mut resampler = crate::dictation::StreamingResampler::new(spec.sample_rate)
        .map_err(|error| eyre!(error))?;
    let scale = (1_i64 << (spec.bits_per_sample.clamp(1, 32) - 1)) as f32;
    let samples: Box<dyn Iterator<Item = Result<f32, hound::Error>>> = match spec.sample_format {
        hound::SampleFormat::Float => Box::new(reader.samples::<f32>()),
        hound::SampleFormat::Int => Box::new(
            reader
                .samples::<i32>()
                .map(move |sample| sample.map(|sample| sample as f32 / scale)),
        ),
    };
    let block = spec.sample_rate.max(1) as usize;
    let mut output = Vec::new();
    let mut mono = Vec::with_capacity(block);
    let (mut frame, mut channel) = (0.0, 0);
    for sample in samples {
        frame += sample?;
        channel += 1;
        if channel == channels {
            mono.push(frame / channels as f32);
            (frame, channel) = (0.0, 0);
            if mono.len() == block {
                output.extend(resampler.push(&mono));
                mono.clear();
            }
        }
    }
    output.extend(resampler.push(&mono));
    output.extend(resampler.finish());
    Ok(output)
}

fn to_pcm(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_mix_where_each_started() {
        let mut mixed = Vec::new();
        mix_into(&mut mixed, &[0.5, 0.5], 0);
        mix_into(&mut mixed, &[0.25, 0.25], 1);
        assert_eq!(mixed, vec![0.5, 0.75, 0.25]);
        assert_eq!(to_pcm(&[2.0, -2.0]), vec![i16::MAX, -i16::MAX]);
        assert_eq!(samples_ms(ms_samples(12_340)), 12_340);
    }
}
