//! "Mute" and "Lower audio" through the players themselves, for an output
//! whose volume macOS cannot set (a DisplayPort or HDMI display, many USB
//! interfaces): Music's and Spotify's own volume, over AppleScript, ramped
//! by the same `output_ducking` ramp as an output's channels.
use std::cell::RefCell;
use std::process::Command;

use super::{MediaPlayer, running_media_players};
use crate::audio::output_ducking::Volume;

/// The players that were playing when lowering began, with their volumes
/// as fractions (AppleScript's `sound volume` is 0 to 100).
pub(super) struct Players {
    players: Vec<MediaPlayer>,
    /// What the last write read back, for the next read only: one ramp step
    /// runs one script, while a read before restoring asks the players (and
    /// notices a volume changed meanwhile).
    echoed: RefCell<Option<Vec<f32>>>,
}

impl Players {
    /// The running players with a volume that are playing now, if any.
    pub(super) fn playing() -> Option<Self> {
        let running = running_media_players();
        let candidates: Vec<_> = MediaPlayer::ALL
            .into_iter()
            .filter(|player| player.has_volume() && running.contains(player))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let players: Vec<_> = osascript(&playing_script(&candidates))?
            .split(',')
            .map(str::trim)
            .filter_map(MediaPlayer::from_name)
            .collect();
        (!players.is_empty()).then_some(Self {
            players,
            echoed: RefCell::new(None),
        })
    }
}

impl Volume for Players {
    fn read(&self) -> Option<Vec<f32>> {
        if let Some(echoed) = self.echoed.take() {
            return Some(echoed);
        }
        parse_volumes(
            &osascript(&volumes_script(&self.players, None))?,
            self.players.len(),
        )
    }

    fn write(&mut self, values: &[f32]) -> bool {
        let echoed = osascript(&volumes_script(&self.players, Some(values)))
            .and_then(|reply| parse_volumes(&reply, self.players.len()));
        let written = echoed.is_some();
        self.echoed.replace(echoed);
        written
    }
}

/// The names of `players` that are playing, comma-separated. Only running
/// players are named: a `tell` to one that is not installed asks where it is.
fn playing_script(players: &[MediaPlayer]) -> String {
    let mut script = String::from("set playingPlayers to {}\n");
    for player in players {
        let name = player.name();
        let playing = player.playing_clause();
        script.push_str(&format!(
            "try\n  if application \"{name}\" is running then\n    tell application \"{name}\"\n      if {playing} then set end of playingPlayers to \"{name}\"\n    end tell\n  end if\nend try\n"
        ));
    }
    script.push_str("return playingPlayers\n");
    script
}

/// Sets each player's volume to `values` when given, then returns every
/// volume, -1 for a player that quit (never relaunching it).
fn volumes_script(players: &[MediaPlayer], values: Option<&[f32]>) -> String {
    let mut script = String::from("set playerVolumes to {}\n");
    for (index, player) in players.iter().enumerate() {
        let name = player.name();
        let set = values
            .and_then(|values| values.get(index))
            .map(|value| {
                let percent = (value.clamp(0.0, 1.0) * 100.0).round() as u8;
                format!("      set sound volume to {percent}\n")
            })
            .unwrap_or_default();
        script.push_str(&format!(
            "try\n  if application \"{name}\" is running then\n    tell application \"{name}\"\n{set}      set end of playerVolumes to sound volume\n    end tell\n  else\n    set end of playerVolumes to -1\n  end if\non error\n  set end of playerVolumes to -1\nend try\n"
        ));
    }
    script.push_str("return playerVolumes\n");
    script
}

/// `count` volumes from AppleScript's "100, 40", as fractions; none when a
/// player quit or the reply is not that.
fn parse_volumes(reply: &str, count: usize) -> Option<Vec<f32>> {
    let volumes = reply
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<f32>()
                .ok()
                .filter(|volume| (0.0..=100.0).contains(volume))
                .map(|volume| volume / 100.0)
        })
        .collect::<Option<Vec<_>>>()?;
    (volumes.len() == count).then_some(volumes)
}

fn osascript(script: &str) -> Option<String> {
    match Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Ok(output) => {
            tracing::warn!(
                error = %String::from_utf8_lossy(&output.stderr).trim(),
                "could not set the players' volume while recording"
            );
            None
        }
        Err(error) => {
            tracing::warn!(%error, "could not set the players' volume while recording");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volumes_parse_as_fractions_and_a_quit_player_fails_the_read() {
        assert_eq!(parse_volumes("100, 40", 2), Some(vec![1.0, 0.4]));
        assert_eq!(parse_volumes("5", 1), Some(vec![0.05]));
        assert_eq!(parse_volumes("100, -1", 2), None);
        assert_eq!(parse_volumes("100", 2), None);
        assert_eq!(parse_volumes("", 1), None);
    }

    #[test]
    fn scripts_tell_only_the_named_players_and_never_launch_one() {
        let players = [MediaPlayer::Music];
        let read = volumes_script(&players, None);
        assert!(read.contains("if application \"Music\" is running"));
        assert!(!read.contains("Spotify"));
        assert!(!read.contains("set sound volume"));
        let write = volumes_script(&players, Some(&[0.054]));
        assert!(write.contains("set sound volume to 5\n"));
        let playing = playing_script(&[MediaPlayer::Spotify]);
        assert!(playing.contains("tell application \"Spotify\""));
        assert!(!playing.contains("Music"));
    }

    #[test]
    fn only_players_with_a_volume_are_lowered() {
        assert!(MediaPlayer::Music.has_volume());
        assert!(MediaPlayer::Spotify.has_volume());
        assert!(!MediaPlayer::Vlc.has_volume());
    }
}
