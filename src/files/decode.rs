//! Any audio or video file to the 16 kHz mono WAV transcription reads,
//! through AVFoundation (`scrawl-helper audio`, macOS).

use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::{Result, eyre};

/// Formats Choose file… and dropping offer; AVFoundation reads them all.
pub const EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "aac", "wav", "aif", "aiff", "caf", "flac", "ogg", "opus", "m4b", "mp4", "mov",
    "m4v", "3gp", "webm",
];

/// Whether `path` names a format Files takes.
pub fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
}

/// Decoding a long recording takes a while; past this it has stalled.
const DECODE_LIMIT: Duration = Duration::from_secs(30 * 60);

/// Writes `source`'s audio, mixed to 16 kHz mono, to `wav`, and returns its
/// length in milliseconds.
pub fn to_wav(source: &Path, wav: &Path) -> Result<u64> {
    if !source.is_file() {
        return Err(eyre!("The file is gone: {}", source.display()));
    }
    export(source, wav)?;
    let reader = hound::WavReader::open(wav)
        .map_err(|error| eyre!("The decoded audio couldn't be read: {error}"))?;
    let spec = reader.spec();
    let frames = u64::from(reader.duration());
    Ok(frames * 1_000 / u64::from(spec.sample_rate.max(1)))
}

#[cfg(all(target_os = "macos", hex_capture))]
fn export(source: &Path, wav: &Path) -> Result<()> {
    scrawl_capture::helper::export_audio_within(source, wav, DECODE_LIMIT).map_err(|error| {
        let message = format!("{error:#}");
        if message.contains("no audio") {
            eyre!("This file has no audio.")
        } else {
            eyre!("The file's audio couldn't be read: {message}")
        }
    })
}

#[cfg(not(all(target_os = "macos", hex_capture)))]
fn export(_source: &Path, _wav: &Path) -> Result<()> {
    let _ = DECODE_LIMIT;
    Err(eyre!("Files reads audio only on macOS for now."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_and_video_formats_are_taken_by_extension() {
        assert!(supported(Path::new("/a/Интервью.M4A")));
        assert!(supported(Path::new("talk.mov")));
        assert!(!supported(Path::new("notes.txt")));
        assert!(!supported(Path::new("noextension")));
    }
}
