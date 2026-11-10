//! Assistant: answers questions about VibeHex from the user guide
//! (docs/guide), which the binary carries. It is on by default and not in
//! Labs, an exception the owner chose (see AGENTS.md).
//!
//! `guide` holds the articles, `retrieve` finds the ones a question is
//! about, `prompt` puts together what a model reads for one question. What reaches a model is the question, the conversation so far,
//! and guide text: never transcripts, history, meetings, or keys.

pub mod guide;
pub mod intent;
pub mod prompt;
pub mod retrieve;
pub mod store;
pub mod worker;
