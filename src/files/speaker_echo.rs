//! Takes the call's own sound out of a meeting's microphone track, so the
//! player plays each remote word once.
//!
//! On speakers the microphone hears the call too: in a call on external
//! speakers (2026-11-11) about 250 ms after the system track and only about
//! 4 dB quieter, so mixing the two tracks played every remote word twice.
//! WebRTC's echo canceller (AEC3, `sonora`) takes the system track as the
//! sound the speakers played and removes it from the microphone, after the
//! call, on the recorded tracks; capture and transcription are untouched.
//! The result is kept beside the recording (`FILE`), so a meeting is
//! cleaned once, the first time it plays.

use std::path::Path;

use color_eyre::eyre::{Result, WrapErr};

use super::playback::RATE;

/// The cleaned microphone, beside `microphone.wav` in the meeting's folder
/// (16 kHz, 16-bit mono). It goes wherever the folder goes: Delete and
/// retention move the whole folder to the Trash.
pub const FILE: &str = "microphone.echo-cancelled.wav";

/// AEC3 works in 10 ms frames.
const FRAME: usize = RATE as usize / 100;

/// `microphone` with `system`'s sound cancelled out of it, both 16 kHz and
/// placed on the meeting's timeline from `microphone_at` and `system_at`
/// samples. Read from `directory`'s copy when one of the same length is
/// there, else cancelled and saved there.
pub fn cleaned(
    directory: &Path,
    microphone: &[f32],
    microphone_at: usize,
    system: &[f32],
    system_at: usize,
) -> Vec<f32> {
    let path = directory.join(FILE);
    if let Ok(saved) = read(&path)
        && saved.len() == microphone.len()
    {
        return saved;
    }
    let started = std::time::Instant::now();
    let samples = cancel(microphone, microphone_at, system, system_at);
    tracing::info!(
        seconds = microphone.len() / RATE as usize,
        took_ms = started.elapsed().as_millis() as u64,
        "cancelled the speakers' echo in a meeting's microphone"
    );
    if let Err(error) = save(&path, &samples) {
        tracing::warn!("couldn't keep the meeting's cleaned microphone: {error:#}");
    }
    samples
}

/// Runs AEC3 over the whole track: for each 10 ms of microphone, first the
/// system audio of the same moment, then the microphone.
pub fn cancel(
    microphone: &[f32],
    microphone_at: usize,
    system: &[f32],
    system_at: usize,
) -> Vec<f32> {
    let stream = sonora::StreamConfig::new(RATE, 1);
    let mut processor = sonora::AudioProcessing::builder()
        .config(sonora::Config {
            echo_canceller: Some(sonora::config::EchoCanceller::default()),
            ..Default::default()
        })
        .capture_config(stream)
        .render_config(stream)
        .build();
    let system_sample = |at: usize| {
        at.checked_sub(system_at)
            .and_then(|index| system.get(index))
            .copied()
            .unwrap_or(0.0)
    };
    let mut cleaned = Vec::with_capacity(microphone.len() + FRAME);
    let mut render = [0.0; FRAME];
    let mut capture = [0.0; FRAME];
    let mut played = [0.0; FRAME];
    let mut output = [0.0; FRAME];
    for start in (0..microphone.len()).step_by(FRAME) {
        for (offset, slot) in render.iter_mut().enumerate() {
            *slot = system_sample(microphone_at + start + offset);
        }
        capture.fill(0.0);
        let end = (start + FRAME).min(microphone.len());
        capture[..end - start].copy_from_slice(&microphone[start..end]);
        let rendered = processor.process_render_f32(&[&render], &mut [&mut played]);
        let captured =
            rendered.and_then(|()| processor.process_capture_f32(&[&capture], &mut [&mut output]));
        match captured {
            Ok(()) => cleaned.extend_from_slice(&output),
            // A frame it can't take plays as recorded.
            Err(error) => {
                tracing::debug!(?error, "echo canceller refused a frame");
                cleaned.extend_from_slice(&capture);
            }
        }
    }
    cleaned.truncate(microphone.len());
    cleaned
}

fn read(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let scale = f32::from(i16::MAX);
    reader
        .samples::<i16>()
        .map(|sample| Ok(f32::from(sample?) / scale))
        .collect()
}

/// Written whole beside the final name, then moved over it, so a copy cut
/// short by quitting is never read back; owner-only, like the recording.
fn save(path: &Path, samples: &[f32]) -> Result<()> {
    let partial = path.with_extension("wav.partial");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options
        .open(&partial)
        .wrap_err_with(|| format!("{} couldn't be written", partial.display()))?;
    let mut writer = hound::WavWriter::new(std::io::BufWriter::new(file), spec)?;
    for sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
    }
    writer.finalize()?;
    std::fs::rename(&partial, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Noise in syllable-long bursts, a stand-in for speech.
    fn speech(seconds: usize, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..seconds * RATE as usize)
            .map(|index| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = (state >> 8) as f32 / (1 << 24) as f32 - 0.5;
                let syllable = (index / 3_200) % 3 != 2;
                if syllable { noise * 0.4 } else { 0.0 }
            })
            .collect()
    }

    fn level_db(samples: &[f32]) -> f32 {
        let power =
            samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len().max(1) as f32;
        10.0 * (power + 1e-12).log10()
    }

    #[test]
    fn the_speakers_echo_goes_and_the_users_voice_stays() {
        let rate = RATE as usize;
        // The remote side talks for 20 s, then the user for 5 s.
        let mut system = speech(20, 1);
        system.extend(std::iter::repeat_n(0.0, 5 * rate));
        let delay = rate / 4;
        let mut microphone = vec![0.0; system.len()];
        for (index, sample) in system.iter().enumerate() {
            if let Some(slot) = microphone.get_mut(index + delay) {
                *slot += sample * 0.6;
            }
        }
        let user = speech(5, 2);
        let user_at = 20 * rate;
        for (slot, sample) in microphone[user_at..].iter_mut().zip(&user) {
            *slot += sample;
        }
        // The microphone track started 100 ms after the system track.
        let microphone_at = rate / 10;
        let microphone = microphone.split_off(microphone_at);

        let cleaned = cancel(&microphone, microphone_at, &system, 0);
        assert_eq!(cleaned.len(), microphone.len());
        // Past the first seconds, while the canceller learns the room.
        let echo = 10 * rate..19 * rate;
        let echo_before = level_db(&microphone[echo.clone()]);
        let echo_after = level_db(&cleaned[echo]);
        assert!(
            echo_before - echo_after > 15.0,
            "{echo_before} -> {echo_after}"
        );
        let voice = user_at - microphone_at + rate / 2..user_at - microphone_at + 5 * rate - rate;
        let voice_before = level_db(&microphone[voice.clone()]);
        let voice_after = level_db(&cleaned[voice]);
        assert!(
            (voice_before - voice_after).abs() < 2.0,
            "{voice_before} -> {voice_after}"
        );
    }

    #[test]
    fn a_meeting_is_cleaned_once_and_read_back_after() {
        let directory = std::env::temp_dir().join(format!(
            "hex-speaker-echo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let system = speech(2, 3);
        let microphone = speech(2, 4);
        let first = cleaned(&directory, &microphone, 0, &system, 0);
        assert!(directory.join(FILE).is_file());
        assert!(!directory.join(FILE).with_extension("wav.partial").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(directory.join(FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        // The saved copy is read back, not cancelled again.
        let marked = vec![0.25; microphone.len()];
        save(&directory.join(FILE), &marked).unwrap();
        let again = cleaned(&directory, &microphone, 0, &system, 0);
        assert!((again[0] - 0.25).abs() < 1e-3);
        assert_eq!(first.len(), again.len());
        // A copy of another length belongs to other audio: cancelled anew.
        let longer = speech(3, 4);
        assert_eq!(
            cleaned(&directory, &longer, 0, &system, 0).len(),
            longer.len()
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
