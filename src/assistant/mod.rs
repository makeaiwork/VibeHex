//! Assistant: answers questions about VibeHex from the user guide
//! (docs/guide), which the binary carries. It is on by default and not in
//! Labs, an exception the owner chose (see AGENTS.md).
//!
//! `guide` holds the articles, `retrieve` finds the ones a question is
//! about, `prompt` puts together what a model reads for one question. What
//! reaches a model is the question, the conversation so far, and guide
//! text; never history, meetings, Files texts, or keys, except for one path:
//! `intent` reads from a question's words alone whether it asks about the
//! person's own recordings, and only once they have allowed it
//! (`recordings::Allowed`) does `recordings` search them and hand the
//! passages found to the prompt.

pub mod guide;
pub mod intent;
pub mod prompt;
pub mod recordings;
pub mod retrieve;
pub mod store;
pub mod worker;
