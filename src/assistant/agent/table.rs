//! The tools a model may call, as its instructions list them: each one's
//! name, its call written out, and what it gives. All of them only read.

/// One tool, as the model reads it.
pub struct Tool {
    pub name: &'static str,
    /// Its call, written out.
    pub call: &'static str,
    pub what: &'static str,
    /// It reads the person's recordings: only with their leave.
    pub recordings: bool,
}

const fn tool(name: &'static str, call: &'static str, what: &'static str) -> Tool {
    Tool {
        name,
        call,
        what,
        recordings: false,
    }
}

const fn recordings(name: &'static str, call: &'static str, what: &'static str) -> Tool {
    Tool {
        name,
        call,
        what,
        recordings: true,
    }
}

pub const TOOLS: &[Tool] = &[
    tool(
        "guide_search",
        r#"{"tool":"guide_search","query":"words","limit":4}"#,
        "the guide's articles that match some words, as ids and titles (at most 6)",
    ),
    tool(
        "guide_read",
        r#"{"tool":"guide_read","ids":["labs/meetings"]}"#,
        "guide articles whole, by id from <guide_contents> (at most 3), with their settings' lines",
    ),
    tool(
        "facts",
        r#"{"tool":"facts"}"#,
        "the <state> of this install again",
    ),
    tool(
        "settings_get",
        r#"{"tool":"settings_get","keys":["labs.smart_spacing"]}"#,
        "settings' lines (key | values | where | now) by key, or by \"prefix\":\"meetings.\"",
    ),
    tool(
        "cleanup_instructions",
        r#"{"tool":"cleanup_instructions"}"#,
        "the person's cleanup instructions and replacements, every mode's",
    ),
    tool(
        "vocabulary_lookup",
        r#"{"tool":"vocabulary_lookup","words":["Kubernetes"]}"#,
        "the Vocabulary entries that name some words, with their sounds-like spellings",
    ),
    tool(
        "vocabulary_list",
        r#"{"tool":"vocabulary_list"}"#,
        "the Vocabulary's terms, without spellings",
    ),
    recordings(
        "recordings_search",
        r#"{"tool":"recordings_search","query":"words","kinds":["meetings","files","dictations"],"since":"2026-11-01","until":"2026-11-08","latest":5}"#,
        "excerpts of the person's meetings, Files texts, and dictations that match some words; \
         every argument but one of query, since, latest is optional; since and until take a \
         date, `yesterday`, or `7d`",
    ),
    recordings(
        "meetings_list",
        r#"{"tool":"meetings_list","since":"7d","query":"words of a title","limit":10}"#,
        "the person's meetings, newest first, with ids (at most 20); every argument optional",
    ),
    recordings(
        "meeting_notes",
        r#"{"tool":"meeting_notes","id":"…"}"#,
        "one meeting's notes, by its id, or by \"n\", the number of an excerpt of it",
    ),
    recordings(
        "meeting_transcript",
        r#"{"tool":"meeting_transcript","id":"…","from":"12:30","to":"20:00"}"#,
        "a stretch of one meeting's transcript in order, by id or \"n\"; from and to optional",
    ),
    recordings(
        "files_list",
        r#"{"tool":"files_list","query":"words of a name"}"#,
        "the Files texts, newest first, with ids; query optional",
    ),
    recordings(
        "file_read",
        r#"{"tool":"file_read","id":"…","from":"0:00","to":"10:00"}"#,
        "a stretch of one Files text in order, by id or \"n\"; from and to optional",
    ),
    recordings(
        "dictation_read",
        r#"{"tool":"dictation_read","n":3}"#,
        "one dictation whole, heard and written, by \"n\", its excerpt's number, or its id",
    ),
    recordings(
        "usage_insights",
        r#"{"tool":"usage_insights"}"#,
        "what their use of VibeHex shows: counts, shares, apps; never what they said",
    ),
    tool(
        "release_notes",
        r#"{"tool":"release_notes","version":"3.3.0-vibe.5","history":false}"#,
        "VibeHex's release notes: since the version they ran before when no argument is given, \
         one version's, or every release with \"history\":true",
    ),
];

/// The tools as the instructions list them.
pub fn listed() -> String {
    TOOLS
        .iter()
        .map(|tool| format!("- `{}`: {}.\n", tool.call, tool.what))
        .collect()
}
