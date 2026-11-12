//! Assistant as an agent: on a remote engine the model itself asks the app
//! for what to read, a step at a time, instead of the app guessing from
//! the question's words what to put in the prompt.
//!
//! - [`protocol`]: the fenced `hex-read` block a model asks with, and the
//!   `<read>` block its results come back in.

// The loop that speaks the protocol comes next; until then only its tests
// read it.
#![allow(dead_code)]

pub mod protocol;
