//! `hex settings` keys for Labs › Collapsible sidebar and Listen to the
//! recording.

use super::{Key, Kind, key};

pub(super) const SIDEBAR_KEYS: &[Key] = &[
    key(
        "labs.collapsible_sidebar",
        Kind::Bool,
        "Labs: a button beside the logo folds the sidebar into a narrow column of icons",
    ),
    key(
        "labs.sidebar_collapsed",
        Kind::Bool,
        "The sidebar is folded into icons (while labs.collapsible_sidebar is on)",
    ),
    key(
        "labs.playback",
        Kind::Bool,
        "Labs: Play beside the requests plays a meeting or a file from its transcript; a click on a line plays from there (macOS)",
    ),
];
