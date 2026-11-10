//! Labs › Lower every app: "Mute" and "Lower audio" on an output macOS has no
//! volume for (a DisplayPort or HDMI display) turn every other app down in
//! Hex itself. A global process tap, muted while tapped, takes every other
//! app's sound (Hex's own sounds stay out of it), and a private aggregate
//! device plays it to the same output at the ramp's gain. The gain eases a
//! little every sample, so the ramp's 16 ms steps never click.
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use block2::RcBlock;
use color_eyre::eyre::{Result, eyre};
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_audio::{
    AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
    AudioDeviceStart, AudioDeviceStop, AudioHardwareCreateAggregateDevice,
    AudioHardwareCreateProcessTap, AudioHardwareDestroyAggregateDevice,
    AudioHardwareDestroyProcessTap, AudioObjectID, CATapDescription, CATapMuteBehavior,
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceMainSubDeviceKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceSubDeviceListKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioDevicePropertyDeviceUID, kAudioSubDeviceUIDKey,
    kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey,
};
use objc2_core_audio_types::{AudioBuffer, AudioBufferList, AudioTimeStamp};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};

use crate::audio::output_ducking::Volume;

/// The gain moves this fraction of the way to its target every sample:
/// about 10 ms at 48 kHz, quick enough to follow the ramp, slow enough that
/// its steps never click.
const EASE: f32 = 1.0 / 480.0;
/// How long tearing down waits for the gain to come back to full, so the
/// apps' own sound takes over at the same loudness.
const SETTLE: Duration = Duration::from_millis(300);
/// Output channels beyond these are left silent (a stereo tap feeds them).
const MAX_CHANNELS: usize = 8;

/// Every other app's sound, tapped and played back at a gain.
pub(super) struct EveryApp {
    tap: AudioObjectID,
    aggregate: AudioObjectID,
    io_proc: AudioDeviceIOProcID,
    /// The gain the ramp asks for, as f32 bits.
    target: Arc<AtomicU32>,
    /// The gain the last IO cycle ended on, as f32 bits.
    current: Arc<AtomicU32>,
}

impl EveryApp {
    /// Taps every other app onto `output`; none without system audio access
    /// (a tap without it would open the macOS dialog mid-dictation).
    pub(super) fn start(output: AudioObjectID) -> Option<Self> {
        if !matches!(
            crate::meeting::system_audio_authorization(),
            crate::meeting::SystemAudioAuthorization::Granted
        ) {
            tracing::info!("Lower every app needs system audio access; lowering the players");
            return None;
        }
        Self::open(output)
            .inspect_err(|error| tracing::warn!(%error, "could not turn every app down"))
            .ok()
    }

    fn open(output: AudioObjectID) -> Result<Self> {
        let output_uid =
            crate::microphone_activity::property_cf_string(output, kAudioDevicePropertyDeviceUID)
                .ok_or_else(|| eyre!("the output has no UID"))?;
        let excluded = crate::meeting::own_process_object()
            .map(|object| vec![NSNumber::new_u32(object)])
            .unwrap_or_default();
        let excluded = NSArray::from_retained_slice(&excluded);
        // SAFETY: Standard CATapDescription initialization and setters.
        let description = unsafe {
            let description = CATapDescription::initStereoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &excluded,
            );
            description.setName(&NSString::from_str("Hex lowered apps"));
            description.setPrivate(true);
            description.setMuteBehavior(CATapMuteBehavior::MutedWhenTapped);
            description
        };
        let mut tap: AudioObjectID = 0;
        // SAFETY: `description` is a valid tap description; `tap` is writable.
        check(
            unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut tap) },
            "could not tap the other apps",
        )?;
        let mut lowered = Self {
            tap,
            aggregate: 0,
            io_proc: None,
            target: Arc::new(AtomicU32::new(1f32.to_bits())),
            current: Arc::new(AtomicU32::new(1f32.to_bits())),
        };
        // SAFETY: Reading the UUID of a live description.
        let tap_uid = unsafe { description.UUID() }.UUIDString().to_string();
        lowered.aggregate = aggregate(&tap_uid, &output_uid)?;
        let target = lowered.target.clone();
        let current = lowered.current.clone();
        let gain = Cell::new(1.0f32);
        let block = RcBlock::new(
            move |_now: NonNull<AudioTimeStamp>,
                  input: NonNull<AudioBufferList>,
                  _input_time: NonNull<AudioTimeStamp>,
                  output: NonNull<AudioBufferList>,
                  _output_time: NonNull<AudioTimeStamp>| {
                let mut value = gain.get();
                let goal = f32::from_bits(target.load(Ordering::Relaxed));
                // SAFETY: Core Audio passes valid float buffer lists for this cycle.
                unsafe { render(input, output, &mut value, goal) };
                gain.set(value);
                current.store(value.to_bits(), Ordering::Relaxed);
            },
        );
        let mut io_proc: AudioDeviceIOProcID = None;
        // SAFETY: The aggregate exists; Core Audio copies the block.
        check(
            unsafe {
                AudioDeviceCreateIOProcIDWithBlock(
                    NonNull::from(&mut io_proc),
                    lowered.aggregate,
                    None,
                    RcBlock::as_ptr(&block),
                )
            },
            "could not attach to the other apps' tap",
        )?;
        lowered.io_proc = io_proc;
        // SAFETY: Starting the IOProc created above.
        check(
            unsafe { AudioDeviceStart(lowered.aggregate, lowered.io_proc) },
            "could not play the other apps back",
        )?;
        Ok(lowered)
    }
}

impl Volume for EveryApp {
    fn read(&self) -> Option<Vec<f32>> {
        Some(vec![f32::from_bits(self.target.load(Ordering::Relaxed))])
    }

    fn write(&mut self, values: &[f32]) -> bool {
        let Some(&gain) = values.first() else {
            return false;
        };
        self.target
            .store(gain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        true
    }
}

impl Drop for EveryApp {
    fn drop(&mut self) {
        if self.io_proc.is_some() {
            // The apps' own sound takes over at full gain only once the
            // playback reached it, so restoring never jumps.
            self.target.store(1f32.to_bits(), Ordering::Relaxed);
            let started = Instant::now();
            while f32::from_bits(self.current.load(Ordering::Relaxed)) < 0.995
                && started.elapsed() < SETTLE
            {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        // SAFETY: Each identifier was created here and is torn down once, in
        // reverse order of creation.
        unsafe {
            if self.aggregate != 0 {
                if self.io_proc.is_some() {
                    AudioDeviceStop(self.aggregate, self.io_proc);
                    AudioDeviceDestroyIOProcID(self.aggregate, self.io_proc);
                }
                AudioHardwareDestroyAggregateDevice(self.aggregate);
            }
            if self.tap != 0 {
                AudioHardwareDestroyProcessTap(self.tap);
            }
        }
    }
}

/// A private aggregate device that reads the tap and plays to `output_uid`.
fn aggregate(tap_uid: &str, output_uid: &str) -> Result<AudioObjectID> {
    fn key(name: &std::ffi::CStr) -> Retained<NSString> {
        NSString::from_str(name.to_str().unwrap_or_default())
    }
    let sub_tap: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::from_retained_objects(
        &[
            &*key(kAudioSubTapUIDKey),
            &*key(kAudioSubTapDriftCompensationKey),
        ],
        &[
            object(NSString::from_str(tap_uid)),
            object(NSNumber::new_bool(true)),
        ],
    );
    let sub_device: Retained<NSDictionary<NSString, AnyObject>> =
        NSDictionary::from_retained_objects(
            &[&*key(kAudioSubDeviceUIDKey)],
            &[object(NSString::from_str(output_uid))],
        );
    let description: Retained<NSDictionary<NSString, AnyObject>> =
        NSDictionary::from_retained_objects(
            &[
                &*key(kAudioAggregateDeviceNameKey),
                &*key(kAudioAggregateDeviceUIDKey),
                &*key(kAudioAggregateDeviceIsPrivateKey),
                &*key(kAudioAggregateDeviceTapAutoStartKey),
                &*key(kAudioAggregateDeviceMainSubDeviceKey),
                &*key(kAudioAggregateDeviceSubDeviceListKey),
                &*key(kAudioAggregateDeviceTapListKey),
            ],
            &[
                object(NSString::from_str("Hex Lowered Apps")),
                object(NSString::from_str(&format!(
                    "anomaly.hex.lowered-apps.{}",
                    std::process::id()
                ))),
                object(NSNumber::new_bool(true)),
                object(NSNumber::new_bool(true)),
                object(NSString::from_str(output_uid)),
                object(NSArray::from_retained_slice(&[sub_device])),
                object(NSArray::from_retained_slice(&[sub_tap])),
            ],
        );
    let mut aggregate: AudioObjectID = 0;
    // SAFETY: NSDictionary is toll-free bridged to CFDictionary.
    let dictionary: &CFDictionary =
        unsafe { &*(Retained::as_ptr(&description) as *const CFDictionary) };
    check(
        unsafe { AudioHardwareCreateAggregateDevice(dictionary, NonNull::from(&mut aggregate)) },
        "could not create the playback device",
    )?;
    Ok(aggregate)
}

fn object<T: objc2::Message>(value: Retained<T>) -> Retained<AnyObject> {
    // SAFETY: Every Objective-C object is an `AnyObject`.
    unsafe { Retained::cast_unchecked(value) }
}

fn check(status: i32, context: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(eyre!("{context} (OSStatus {status})"))
    }
}

/// One channel inside a buffer list: its first sample, the samples between
/// frames, and its frames.
#[derive(Clone, Copy)]
struct Lane {
    first: *mut f32,
    stride: usize,
    frames: usize,
}

/// The list's channels in order, interleaved or not, up to `MAX_CHANNELS`.
///
/// # Safety
/// `list` must point at a valid `AudioBufferList` of 32-bit float samples.
unsafe fn lanes(list: NonNull<AudioBufferList>) -> ([Option<Lane>; MAX_CHANNELS], usize) {
    let mut lanes = [None; MAX_CHANNELS];
    let mut count = 0;
    let list = unsafe { list.as_ref() };
    let buffers: &[AudioBuffer] =
        unsafe { std::slice::from_raw_parts(list.mBuffers.as_ptr(), list.mNumberBuffers as usize) };
    for buffer in buffers {
        let channels = (buffer.mNumberChannels as usize).max(1);
        if buffer.mData.is_null() {
            continue;
        }
        let frames = buffer.mDataByteSize as usize / size_of::<f32>() / channels;
        for channel in 0..channels {
            if count == MAX_CHANNELS {
                return (lanes, count);
            }
            lanes[count] = Some(Lane {
                // SAFETY: `channel` is inside the buffer's first frame.
                first: unsafe { buffer.mData.cast::<f32>().add(channel) },
                stride: channels,
                frames,
            });
            count += 1;
        }
    }
    (lanes, count)
}

/// Plays `input` into `output` at `gain`, easing it towards `goal` every
/// frame. Output channel `n` plays input channel `n` (mod the input's
/// channels); with no input, the output is silent.
///
/// # Safety
/// Both lists must point at valid `AudioBufferList`s of 32-bit float samples.
unsafe fn render(
    input: NonNull<AudioBufferList>,
    output: NonNull<AudioBufferList>,
    gain: &mut f32,
    goal: f32,
) {
    let (sources, source_count) = unsafe { lanes(input) };
    let (sinks, sink_count) = unsafe { lanes(output) };
    let frames = sinks[..sink_count]
        .iter()
        .flatten()
        .map(|lane| lane.frames)
        .min()
        .unwrap_or(0);
    for frame in 0..frames {
        *gain += (goal - *gain) * EASE;
        for (index, sink) in sinks[..sink_count].iter().flatten().enumerate() {
            let sample = (source_count > 0)
                .then(|| sources[index % source_count])
                .flatten()
                .filter(|source| frame < source.frames)
                // SAFETY: `frame` is inside the source lane.
                .map_or(0.0, |source| unsafe {
                    *source.first.add(frame * source.stride)
                });
            // SAFETY: `frame` is inside every sink lane.
            unsafe { *sink.first.add(frame * sink.stride) = sample * *gain };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of two buffers, which `AudioBufferList` (one buffer) can't hold.
    #[repr(C)]
    struct TwoBuffers {
        count: u32,
        buffers: [AudioBuffer; 2],
    }

    fn buffer(samples: &mut [f32], channels: u32) -> AudioBuffer {
        AudioBuffer {
            mNumberChannels: channels,
            mDataByteSize: std::mem::size_of_val(samples) as u32,
            mData: samples.as_mut_ptr().cast(),
        }
    }

    #[test]
    fn interleaved_stereo_plays_into_two_mono_buffers_at_the_eased_gain() {
        let mut input = vec![0.5, -0.5, 0.5, -0.5, 0.5, -0.5];
        let mut left = vec![9.0; 3];
        let mut right = vec![9.0; 3];
        let mut source = AudioBufferList {
            mNumberBuffers: 1,
            mBuffers: [buffer(&mut input, 2)],
        };
        let mut sink = TwoBuffers {
            count: 2,
            buffers: [buffer(&mut left, 1), buffer(&mut right, 1)],
        };
        let mut gain = 1.0;
        // SAFETY: Both lists describe the live vectors above.
        unsafe {
            render(
                NonNull::from(&mut source),
                NonNull::from(&mut sink).cast(),
                &mut gain,
                1.0,
            );
        }
        assert_eq!(left, vec![0.5; 3]);
        assert_eq!(right, vec![-0.5; 3]);

        let mut gain = 1.0;
        // SAFETY: As above.
        unsafe {
            render(
                NonNull::from(&mut source),
                NonNull::from(&mut sink).cast(),
                &mut gain,
                0.0,
            );
        }
        // One ease a frame: each frame a little quieter than the last, none silent.
        assert!(left[0] < 0.5 && left[1] < left[0] && left[2] < left[1] && left[2] > 0.49);
        assert!((gain - (1.0 - EASE).powi(3)).abs() < 1e-6);
    }

    #[test]
    fn no_input_leaves_the_output_silent() {
        let mut out = vec![9.0; 4];
        let mut source = AudioBufferList {
            mNumberBuffers: 0,
            mBuffers: [buffer(&mut [], 1)],
        };
        let mut sink = AudioBufferList {
            mNumberBuffers: 1,
            mBuffers: [buffer(&mut out, 2)],
        };
        let mut gain = 1.0;
        // SAFETY: Both lists describe live buffers.
        unsafe {
            render(
                NonNull::from(&mut source),
                NonNull::from(&mut sink),
                &mut gain,
                1.0,
            );
        }
        assert_eq!(out, vec![0.0; 4]);
    }
}
