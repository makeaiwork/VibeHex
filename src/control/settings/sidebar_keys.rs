//! `hex settings` keys for Labs › Collapsible sidebar.

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
];
