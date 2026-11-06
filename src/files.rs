//! Labs › Files: audio and video files dropped on the Files pane are
//! transcribed with the dictation engine's model (or one chosen for files),
//! cleaned up, and reworked with the person's own requests, each result a
//! version of the file's text. See `docs/features/files.md`.

pub mod decode;
pub mod playback;
pub mod requests;
pub mod rewrite;
pub mod store;
pub mod text;
pub mod tidy;
pub mod worker;
