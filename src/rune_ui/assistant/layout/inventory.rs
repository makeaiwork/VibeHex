//! The feature doc's inventory of Assistant's surfaces against the code:
//! every scene and every row of the matrix is listed there, and nothing
//! listed there is gone.

use super::matrix::SURFACES;
use super::scenes;

const DOC: &str = include_str!("../../../../docs/features/assistant.md");

/// The inventory: from its heading to the end of the doc or the next one.
fn inventory() -> &'static str {
    let heading = "## Surfaces, scenes, and tests";
    let from = DOC
        .find(heading)
        .unwrap_or_else(|| panic!("docs/features/assistant.md has no \"{heading}\""));
    let rest = &DOC[from + heading.len()..];
    rest.find("\n## ").map_or(rest, |end| &rest[..end])
}

/// What the inventory names in backticks.
fn named() -> Vec<&'static str> {
    inventory().split('`').skip(1).step_by(2).collect()
}

/// The table rows, as their cells.
fn rows() -> Vec<Vec<&'static str>> {
    inventory()
        .lines()
        .filter(|line| line.starts_with("| "))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}

#[test]
fn every_scene_has_its_line() {
    let named = named();
    let missing: Vec<&str> = scenes()
        .map(|(name, _)| name)
        .filter(|name| !named.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "scenes missing from the inventory in docs/features/assistant.md: {missing:?}"
    );
}

#[test]
fn every_surface_has_its_row() {
    let rows = rows();
    for surface in SURFACES {
        let row = rows
            .iter()
            .find(|row| row.first() == Some(&surface.name))
            .unwrap_or_else(|| panic!("no row for \"{}\" in the inventory", surface.name));
        let cells = row.join(" ");
        for scene in surface.scenes {
            assert!(
                cells.contains(&format!("`{scene}`")),
                "{}: its row lacks `{scene}`",
                surface.name
            );
        }
        assert!(
            cells.contains(&format!("`{}`", surface.test)),
            "{}: its row lacks its test `{}`",
            surface.name,
            surface.test
        );
    }
}

#[test]
fn nothing_listed_is_gone() {
    let scenes: Vec<&str> = scenes().map(|(name, _)| name).collect();
    let gone: Vec<&str> = named()
        .into_iter()
        .filter(|name| *name == "assistant" || name.starts_with("assistant-"))
        .filter(|name| !scenes.contains(name))
        .collect();
    assert!(
        gone.is_empty(),
        "the inventory lists scenes that are gone: {gone:?}"
    );
    let rows = rows();
    let tests: Vec<&str> = SURFACES.iter().map(|surface| surface.test).collect();
    for row in rows.iter().filter(|row| row.len() == 3) {
        let test = row[2].trim_matches('`');
        let heading = test.starts_with("Test in") || test.starts_with("---");
        assert!(
            heading || tests.contains(&test),
            "the inventory lists a test that is gone: {test}"
        );
    }
}
