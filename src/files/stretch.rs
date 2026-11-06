//! Speech played faster or slower at its own pitch, for the player: WSOLA.
//! Each step takes a 40 ms frame of the recording, windowed, and lays its
//! first half over the last frame's second half. The frame starts where
//! the speed has moved the recording on, give or take a few milliseconds:
//! wherever its first half best matches what followed the last frame, so
//! the waves line up and no echo or click comes from the seam.

/// A frame: 40 ms at 16 kHz.
const FRAME: usize = 640;
/// What each step plays: half a frame.
pub(super) const HOP: usize = FRAME / 2;
/// How far a frame may move from where the speed puts it, either way.
const SEEK: usize = 120;

pub(super) struct Stretch {
    /// A periodic Hann window: halves a hop apart sum to one.
    window: Vec<f32>,
    /// The last frame's second half, windowed, waiting for the next frame.
    tail: Vec<f32>,
    /// Where the last frame started in the recording.
    last: Option<usize>,
}

impl Stretch {
    pub(super) fn new() -> Self {
        let window = (0..FRAME)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FRAME as f32).cos())
            .collect();
        Self {
            window,
            tail: vec![0.0; HOP],
            last: None,
        }
    }

    /// Starts over, as after a seek: the next frame fades in.
    pub(super) fn reset(&mut self) {
        self.tail.fill(0.0);
        self.last = None;
    }

    /// Plays the next hop of `samples` from `at` at `speed` into `out`, and
    /// says how far the recording moved on.
    pub(super) fn hop(
        &mut self,
        samples: &[i16],
        at: usize,
        speed: f32,
        out: &mut Vec<f32>,
    ) -> usize {
        let sample = |index: usize| {
            samples
                .get(index)
                .map_or(0.0, |sample| f32::from(*sample) / f32::from(i16::MAX))
        };
        let start = match self.last {
            None => at,
            Some(last) => {
                let natural = last + HOP;
                let fit = |start: usize| {
                    (0..HOP)
                        .map(|i| sample(natural + i) * sample(start + i))
                        .sum::<f32>()
                };
                (at.saturating_sub(SEEK)..=at + SEEK)
                    .step_by(2)
                    .map(|start| (start, fit(start)))
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(at, |(start, _)| start)
            }
        };
        for i in 0..HOP {
            out.push(self.tail[i] + sample(start + i) * self.window[i]);
        }
        for i in 0..HOP {
            self.tail[i] = sample(start + HOP + i) * self.window[HOP + i];
        }
        self.last = Some(start);
        (speed * HOP as f32).round() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, seconds: f32) -> Vec<i16> {
        (0..(16_000.0 * seconds) as usize)
            .map(|i| ((std::f32::consts::TAU * hz * i as f32 / 16_000.0).sin() * 16_000.0) as i16)
            .collect()
    }

    /// Plays all of `samples` at `speed`: what came out.
    fn played(samples: &[i16], speed: f32) -> Vec<f32> {
        let mut stretch = Stretch::new();
        let (mut at, mut out) = (0, Vec::new());
        while at < samples.len() {
            at += stretch.hop(samples, at, speed, &mut out);
        }
        out
    }

    fn crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
            .count()
    }

    #[test]
    fn twice_the_speed_takes_half_the_time() {
        let samples = tone(220.0, 2.0);
        let fast = played(&samples, 2.0);
        assert!(
            (fast.len() as i64 - 16_000).abs() <= HOP as i64,
            "{}",
            fast.len()
        );
        let slow = played(&samples, 0.75);
        let expected = (32_000.0 / 0.75) as i64;
        assert!(
            (slow.len() as i64 - expected).abs() <= 2 * HOP as i64,
            "{}",
            slow.len()
        );
    }

    #[test]
    fn a_voice_keeps_its_pitch_and_level() {
        let samples = tone(220.0, 2.0);
        let fast = played(&samples, 1.5);
        // Past the first frame's fade-in: as many crossings per second as
        // the tone has, and the tone's level, with no seam dips.
        let steady = &fast[FRAME..fast.len() - FRAME];
        let per_second = crossings(steady) as f32 * 16_000.0 / steady.len() as f32;
        assert!((per_second - 440.0).abs() < 8.0, "{per_second}");
        let peak = steady
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let quietest = steady
            .chunks(80)
            .map(|chunk| {
                chunk
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
            })
            .fold(f32::MAX, f32::min);
        assert!(quietest > peak * 0.9, "{quietest} of {peak}");
    }
}
