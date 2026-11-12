//! A plan's Vocabulary terms as drawn: one row while folded, a row each
//! once unfolded, a long term and a long "sounds like" list inside their
//! row; and the start page's "like Denis" row, there with Russian
//! recognition alone.

use super::*;

fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
    inner[0] >= outer[0] - 0.5
        && inner[1] >= outer[1] - 0.5
        && inner[0] + inner[2] <= outer[0] + outer[2] + 0.5
        && inner[1] + inner[3] <= outer[1] + outer[3] + 0.5
}

#[test]
fn folded_terms_are_one_row_and_unfold_to_a_row_each() {
    let mut rt = runtime("assistant-plan-terms", false, PANE_SIZE);
    settle(&mut rt);
    let shown = texts(&rt);
    assert!(
        shown.iter().any(|t| t == "Add 7 of 8 Vocabulary terms"),
        "{shown:?}"
    );
    // The terms are counted apart from the two settings.
    for line in [
        "2 changes and 8 terms, 2 and 7 selected",
        "Apply 2 changes and 7 terms",
    ] {
        assert!(shown.iter().any(|t| t == line), "{line}: {shown:?}");
    }
    assert!(shown.iter().any(|t| t.ends_with("and 4 more")), "{shown:?}");
    assert!(rt.find("term2").is_none(), "folded: no row per term");
    // The chevron unfolds it: a row each, and the names line goes.
    tap(&mut rt, "terms/words/names/fold");
    for index in 2..10 {
        assert!(rt.find(&format!("term{index}")).is_some(), "term{index}");
    }
    assert!(rt.find("terms/words/named").is_none());
    // Unchecking a term there changes the one row's count and the plan's
    // count of terms, never its count of settings.
    tap(&mut rt, "term2/check");
    let shown = texts(&rt);
    for line in [
        "Add 6 of 8 Vocabulary terms",
        "2 changes and 8 terms, 2 and 6 selected",
        "Apply 2 changes and 6 terms",
    ] {
        assert!(shown.iter().any(|t| t == line), "{line}: {shown:?}");
    }
    // The row's own checkbox takes them all.
    tap(&mut rt, "terms/lead/check");
    assert!(
        texts(&rt).iter().any(|t| t == "Add 8 Vocabulary terms"),
        "all checked"
    );
    tap(&mut rt, "terms/lead/check");
    assert!(
        texts(&rt).iter().any(|t| t == "Apply 2 changes"),
        "no term checked: two settings left"
    );
}

#[test]
fn a_long_term_and_a_long_sounds_like_list_wrap_inside_their_row() {
    for width in [980.0, 760.0] {
        let mut rt = runtime("assistant-plan-terms-open", false, (width, 1200.0));
        settle(&mut rt);
        for index in 2..10 {
            let line = element(&rt, &format!("term{index}")).rect;
            let column = element(&rt, &format!("term{index}/term")).rect;
            let name = element(&rt, &format!("term{index}/term/name")).rect;
            let about = element(&rt, &format!("term{index}/about")).rect;
            assert!(inside(name, column), "term{index} name {name:?} {column:?}");
            assert!(inside(column, line), "term{index} column");
            assert!(inside(about, line), "term{index} about {about:?} {line:?}");
            assert!(
                column[0] + column[2] <= about[0] + 0.5,
                "term{index}: the term runs over how it sounds"
            );
        }
        // Wrapped, not cut: the long ones are taller than one line.
        let short = element(&rt, "term2/term/name").rect[3];
        for long in [4, 7] {
            let tall = element(&rt, &format!("term{long}/term/name")).rect[3];
            assert!(tall > short * 1.5, "term{long} is {tall} tall");
        }
        let list = element(&rt, "term5/about/sounds-like").rect[3];
        let one = element(&rt, "term2/about/sounds-like").rect[3];
        assert!(list > one * 1.5, "a long list wraps: {list}");
    }
}

#[test]
fn the_presets_terms_are_one_row_until_unfolded() {
    let mut rt = runtime("assistant-setup-preset", false, TALL_SIZE);
    settle(&mut rt);
    let rows = |rt: &Runtime<Ui>| {
        rt.elements()
            .into_iter()
            .filter(|e| {
                e.key.as_deref().is_some_and(|key| {
                    key.strip_prefix("term")
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                })
            })
            .count()
    };
    assert_eq!(rows(&rt), 0, "folded");
    tap(&mut rt, "terms/words/names/fold");
    assert!(rows(&rt) > 80, "unfolded: {} rows", rows(&rt));
}

#[test]
fn like_denis_is_on_the_start_page_with_russian_recognition_alone() {
    let mut english = runtime("assistant-start", false, PANE_SIZE);
    settle(&mut english);
    assert!(english.find("like-denis").is_none());
    let mut russian = runtime("assistant-start-russian", false, PANE_SIZE);
    settle(&mut russian);
    assert!(russian.find("like-denis").is_some());
    // Auto on a model of many languages: no row either.
    russian.state.settings.transcription.language = "auto".into();
    russian.invalidate();
    settle(&mut russian);
    assert!(russian.find("like-denis").is_none());
}
