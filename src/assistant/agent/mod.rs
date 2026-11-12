//! Assistant as an agent: on a remote engine the model itself asks the app
//! for what to read, a step at a time, instead of the app guessing from
//! the question's words what to put in the prompt.
//!
//! - [`protocol`]: the fenced `hex-read` block a model asks with, and the
//!   `<read>` block its results come back in.
//! - [`table`]: what it may ask for, all of it read-only; [`tools`]: the
//!   reading, within a turn's [`Limits`]; [`access`]: the person's leave
//!   to read their recordings, asked for mid-answer when they haven't said.
//! - [`trace`]: what a turn read, as the pane lists it.

// The loop that speaks the protocol comes next; until then only its tests
// read it.
#![allow(dead_code)]

pub mod access;
pub mod protocol;
pub mod table;
pub mod tools;
pub mod trace;

use std::time::Duration;

use crate::assistant::recordings::Budget;

/// How much one turn may ask and read.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Requests to the model; the last is told to answer.
    pub requests: usize,
    /// Calls the app runs of one reply's block.
    pub step_calls: usize,
    /// Calls in all.
    pub turn_calls: usize,
    /// Characters of one result.
    pub result_chars: usize,
    /// Characters of every result together.
    pub turn_chars: usize,
    /// One search of the recordings.
    pub search: Budget,
    /// The model's own time, after which the next request is the last.
    pub model_time: Duration,
}

impl Limits {
    /// A cloud model with a long window.
    pub const REMOTE: Self = Self {
        requests: 6,
        step_calls: 4,
        turn_calls: 12,
        result_chars: 9_000,
        turn_chars: 60_000,
        search: Budget::LARGE,
        model_time: Duration::from_secs(300),
    };
    /// The person's own server, whose window may be short.
    pub const OWN_SERVER: Self = Self {
        turn_chars: 16_000,
        search: Budget::SMALL,
        ..Self::REMOTE
    };
}
