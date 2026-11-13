//! What an agent turn did before it answered, in order: the articles the
//! app laid out first, then each thing the model asked to read. A turn
//! keeps it (`Turn::trace`) and the pane lists it under the answer, as it
//! lists a recordings search's steps; while the answer comes, the same
//! list grows live.
//!
//! It names what was read, never its text: an article's id, a setting's
//! key, a meeting's title (as an answer's places already keep it), counts.
//! A build that doesn't know a kind of step reads it as [`Did::Unknown`]
//! and shows nothing for it.

use serde::{Deserialize, Serialize};

use crate::assistant::recordings::Step;

/// One thing a turn read, or was refused.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "did", rename_all = "snake_case")]
pub enum Did {
    /// Guide articles, by id: `app` for the ones the app laid out before
    /// the model said anything, else the model asked for them.
    Articles {
        #[serde(default)]
        ids: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        app: bool,
    },
    /// A search of the guide, and how many articles it named.
    GuideSearch {
        #[serde(default)]
        query: String,
        #[serde(default)]
        found: usize,
    },
    /// Settings' lines of the catalog, by key; none named: the state of
    /// this install.
    Settings {
        #[serde(default)]
        keys: Vec<String>,
    },
    /// The cleanup instructions.
    Instructions,
    /// Vocabulary entries: how many it read, of how many there are.
    Vocabulary {
        #[serde(default)]
        named: usize,
        #[serde(default)]
        total: usize,
    },
    /// A search of the recordings, or a list of them, as its steps.
    Recordings {
        #[serde(default)]
        steps: Vec<Step>,
    },
    /// One meeting's notes.
    Notes {
        #[serde(default)]
        title: String,
    },
    /// A stretch of one meeting's transcript.
    Transcript {
        #[serde(default)]
        title: String,
    },
    /// A stretch of one Files text.
    File {
        #[serde(default)]
        title: String,
    },
    /// One dictation, whole.
    Dictation,
    /// What the use shows: counts, never text.
    Usage,
    /// Release notes, by version.
    Releases {
        #[serde(default)]
        versions: Vec<String>,
    },
    /// The person was asked, mid-answer, whether the recordings may be
    /// read, and what they said.
    Consent {
        #[serde(default)]
        allowed: bool,
    },
    /// A call that read nothing, and why ([`Why`]'s name).
    Refused {
        #[serde(default)]
        tool: String,
        #[serde(default)]
        why: String,
    },
    /// A step a later build wrote.
    #[serde(other)]
    Unknown,
}

/// What an agent turn is doing now, for the line under its steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Doing {
    /// The model has the question and what was read so far.
    Thinking,
    /// The app is running this tool.
    Reading(&'static str),
}

/// Why a call read nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Why {
    /// No tool has that name.
    Unknown,
    /// The recordings may not be read.
    NotAllowed,
    /// The person was asked whether the recordings may be read and gave
    /// no answer in time.
    Unanswered,
    /// The person's settings aren't sent (`assistant.send_settings`).
    NotSent,
    /// The turn has read as much as it may.
    Budget,
    /// The call named nothing the tool could read (a wrong id, a time it
    /// couldn't make out).
    Failed,
}

impl Why {
    pub fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NotAllowed => "not_allowed",
            Self::Unanswered => "unanswered",
            Self::NotSent => "not_sent",
            Self::Budget => "budget",
            Self::Failed => "failed",
        }
    }
}

impl Did {
    pub fn refused(tool: &str, why: Why) -> Self {
        Self::Refused {
            tool: tool.to_owned(),
            why: why.name().to_owned(),
        }
    }

    /// The tool it names when it was refused for `why`.
    pub fn refused_for(&self, why: Why) -> Option<&str> {
        match self {
            Self::Refused { tool, why: known } if known == why.name() => Some(tool),
            _ => None,
        }
    }

    /// It holds something of the person's recordings: excerpts found, a
    /// list that named any, a stretch read.
    pub fn read_recordings(&self) -> bool {
        match self {
            Self::Recordings { steps } => steps.iter().any(|step| match step {
                Step::Meetings { found, .. }
                | Step::Files { found, .. }
                | Step::Dictations { found, .. } => *found > 0,
                Step::Read { notes, passages } => notes + passages > 0,
                Step::Looked {
                    dictations,
                    meetings,
                } => dictations + meetings > 0,
                Step::Guide { .. } => false,
            }),
            Self::Notes { .. } | Self::Transcript { .. } | Self::File { .. } | Self::Dictation => {
                true
            }
            _ => false,
        }
    }

    /// It holds something of the person's own that never goes online:
    /// their recordings, what their use shows, their cleanup instructions,
    /// or Vocabulary entries.
    pub fn personal(&self) -> bool {
        self.read_recordings()
            || match self {
                Self::Usage | Self::Instructions => true,
                Self::Vocabulary { named, .. } => *named > 0,
                _ => false,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_a_later_build_wrote_reads_as_unknown() {
        let read: Vec<Did> = serde_json::from_str(
            r#"[{"did":"articles","ids":["topics/meetings"],"app":true},
                {"did":"teleported","to":"mars"},
                {"did":"notes"},
                {"did":"consent","allowed":false}]"#,
        )
        .unwrap();
        assert_eq!(
            read,
            [
                Did::Articles {
                    ids: vec!["topics/meetings".into()],
                    app: true
                },
                Did::Unknown,
                Did::Notes {
                    title: String::new()
                },
                Did::Consent { allowed: false },
            ]
        );
    }

    #[test]
    fn only_what_is_the_persons_own_counts_as_personal() {
        let search = |found| Did::Recordings {
            steps: vec![Step::Meetings { searched: 9, found }],
        };
        assert!(search(2).read_recordings() && search(2).personal());
        assert!(!search(0).read_recordings() && !search(0).personal());
        assert!(
            Did::Transcript {
                title: String::new()
            }
            .personal()
        );
        assert!(Did::Usage.personal() && !Did::Usage.read_recordings());
        assert!(Did::Instructions.personal());
        assert!(Did::Vocabulary { named: 1, total: 9 }.personal());
        for public in [
            Did::Vocabulary { named: 0, total: 9 },
            Did::Settings { keys: Vec::new() },
            Did::Articles {
                ids: Vec::new(),
                app: true,
            },
            Did::Releases {
                versions: Vec::new(),
            },
            Did::Consent { allowed: true },
            Did::refused("recordings_search", Why::NotAllowed),
        ] {
            assert!(!public.personal(), "{public:?}");
        }
    }
}
