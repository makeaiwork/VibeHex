//! A layout check for any laid-out window, from what its runtime holds
//! after a frame (`Runtime::elements`, and the glyphs it would draw): no
//! pixels, no golden image, the same on every OS. It fails a window where
//!
//! - an element's box leaves its parent's, or a text's glyphs leave every
//!   text box (`Kind::Outside`): a title wider than its card, a row taller
//!   than its list, a word no line could break;
//! - a text is cut by a box above it (`Kind::Clipped`), the window's edge
//!   included, or by its own "…" down to less than half of itself: a
//!   title squeezed to three dots, an option nobody can read;
//! - two siblings that show something overlap (`Kind::Overlap`): a hint
//!   over a title, a button over a value.
//!
//! A text's box is the room its layout gave it, whatever it then drew, so
//! boxes alone miss the word that runs past its card: the glyphs say.
//!
//! A scroll container is excepted by its key ([`Rules::scrolls`]): what
//! it holds may be longer than it, and is cut by it on purpose. A text
//! that ends in "…" and keeps most of itself is cut on purpose too.
//! What a pane overlaps or hangs outside by design is named with why
//! ([`Rules::floats`]), never skipped silently.

use std::collections::HashMap;

use rune::render::KIND_GLYPH;
use rune::{ElementInfo, Runtime};

/// A box may leave another's by this much, in points: rounding, a
/// hairline, a focus ring's outset.
const SLACK: f32 = 1.0;
/// A glyph's quad is a little larger than its ink, and a line box a
/// little smaller than its tallest letters.
const INK: f32 = 3.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Outside,
    Clipped,
    Overlap,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub kind: Kind,
    /// The element's path, and the other's for an overlap.
    pub path: String,
    pub detail: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            Kind::Outside => "outside its box",
            Kind::Clipped => "text clipped",
            Kind::Overlap => "siblings overlap",
        };
        write!(f, "{kind}: {} ({})", self.path, self.detail)
    }
}

/// What a window may do that the check would otherwise fail.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rules {
    /// Keys of scroll containers: their content may leave them.
    pub scrolls: &'static [&'static str],
    /// Keys of elements placed over or outside their parent by design (a
    /// popover, a badge, a mark drawn over its text), each with why. One
    /// starting with `#` matches any key that holds it (`#link-`).
    pub floats: &'static [(&'static str, &'static str)],
    /// Texts that show only their start by design, each with why: the end
    /// of its path, numbers as `#` (`from/label`, `was#/label`).
    pub cuts: &'static [(&'static str, &'static str)],
}

impl Rules {
    fn floating(&self, path: &str) -> bool {
        path.split('/').any(|part| {
            self.floats
                .iter()
                .any(|(key, _)| match key.starts_with('#') {
                    true => part.contains(key),
                    false => part == *key,
                })
        })
    }

    fn cut(&self, path: &str) -> bool {
        let path = numberless(path);
        self.cuts.iter().any(|(end, _)| path.ends_with(end))
    }

    fn scroll(&self, path: &str) -> bool {
        self.scrolls.contains(&key_of(path))
    }
}

/// A glyph as it would be drawn, in points: its quad, and the box it is
/// cut to.
#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub rect: [f32; 4],
    pub clip: [f32; 4],
}

/// A window laid out: its elements and its glyphs.
pub struct Laid {
    pub elements: Vec<ElementInfo>,
    pub glyphs: Vec<Glyph>,
}

impl Laid {
    /// `rt` as its last frame left it, drawn at `scale` pixels a point.
    pub fn of<S: 'static>(rt: &Runtime<S>, scale: f32) -> Self {
        let glyphs = rt
            .instances
            .iter()
            .filter(|instance| instance.params[1] == KIND_GLYPH && instance.fill[3] > 0.01)
            .map(|instance| {
                let [x0, y0, x1, y1] = instance.clip.map(|edge| edge / scale);
                Glyph {
                    rect: instance.rect.map(|side| side / scale),
                    clip: [x0, y0, x1 - x0, y1 - y0],
                }
            })
            .collect();
        Self {
            elements: rt.elements(),
            glyphs,
        }
    }
}

fn right(rect: [f32; 4]) -> f32 {
    rect[0] + rect[2]
}

fn bottom(rect: [f32; 4]) -> f32 {
    rect[1] + rect[3]
}

/// How far `inner` leaves `outer` on each side: left, top, right, bottom.
fn sides(inner: [f32; 4], outer: [f32; 4]) -> [f32; 4] {
    [
        outer[0] - inner[0],
        outer[1] - inner[1],
        right(inner) - right(outer),
        bottom(inner) - bottom(outer),
    ]
}

/// How far `inner` leaves `outer` on its worst side.
fn leaves(inner: [f32; 4], outer: [f32; 4]) -> f32 {
    sides(inner, outer).into_iter().fold(0.0, f32::max)
}

/// The size of what two boxes share, zero when they only touch.
fn shared(a: [f32; 4], b: [f32; 4]) -> (f32, f32) {
    let w = right(a).min(right(b)) - a[0].max(b[0]);
    let h = bottom(a).min(bottom(b)) - a[1].max(b[1]);
    (w.max(0.0), h.max(0.0))
}

/// Drawn at all, whatever room it got.
fn drawn(element: &ElementInfo) -> bool {
    !element.exiting && !element.virtual_extent && element.opacity > 0.01
}

fn shows(element: &ElementInfo) -> bool {
    drawn(element) && element.rect[2] > 0.5 && element.rect[3] > 0.5
}

fn words(element: &ElementInfo) -> &str {
    element.text.as_deref().unwrap_or_default().trim()
}

fn key_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn parent_of(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(parent, _)| parent)
}

fn start(text: &str) -> String {
    text.chars().take(40).collect()
}

/// Everything wrong with `laid`'s layout under `rules`.
pub fn problems(laid: &Laid, rules: Rules) -> Vec<Problem> {
    let mut problems = boxes(&laid.elements, rules);
    problems.extend(ink(laid, rules));
    problems
}

/// The shortest text the ellipsis rule reads: a word or two may be all
/// ligatures and marks.
const READ: usize = 6;

/// A path with every number in it as `#`: the rows of a list are one.
pub fn numberless(path: &str) -> String {
    let mut same = String::new();
    let mut digits = false;
    for c in path.chars() {
        match (c.is_ascii_digit(), digits) {
            (true, false) => same.push('#'),
            (true, true) => {}
            _ => same.push(c),
        }
        digits = c.is_ascii_digit();
    }
    same
}

/// The faults the boxes show: one outside its parent, a text's box cut,
/// two siblings sharing room.
fn boxes(elements: &[ElementInfo], rules: Rules) -> Vec<Problem> {
    let by_path: HashMap<&str, &ElementInfo> = elements
        .iter()
        .map(|element| (element.path.as_str(), element))
        .collect();
    let shown: Vec<&ElementInfo> = elements.iter().filter(|element| shows(element)).collect();
    // Which elements show words of their own or hold one that does.
    let mut holds: HashMap<&str, bool> = HashMap::new();
    for element in shown.iter().filter(|element| !words(element).is_empty()) {
        let mut path = Some(element.path.as_str());
        while let Some(at) = path {
            holds.insert(at, true);
            path = parent_of(at);
        }
    }
    let mut problems = Vec::new();
    let mut children: HashMap<&str, Vec<&ElementInfo>> = HashMap::new();
    for element in &shown {
        let path = element.path.as_str();
        let floating = rules.floating(path);
        let scrolled = path.split('/').any(|part| rules.scrolls.contains(&part));
        let parent = parent_of(path).and_then(|parent| by_path.get(parent).copied());
        if let Some(parent) = parent {
            children
                .entry(parent.path.as_str())
                .or_default()
                .push(element);
            let out = leaves(element.rect, parent.rect);
            if out > SLACK && !floating && !rules.scroll(&parent.path) && shows(parent) {
                problems.push(Problem {
                    kind: Kind::Outside,
                    path: path.to_owned(),
                    detail: format!(
                        "{out:.1} pt past its parent {:?}, itself {:?}",
                        parent.rect, element.rect
                    ),
                });
            }
        }
        let cut = leaves(element.rect, element.visible);
        if !words(element).is_empty() && !floating && !scrolled && cut > SLACK {
            problems.push(Problem {
                kind: Kind::Clipped,
                path: path.to_owned(),
                detail: format!("{cut:.1} pt of \"{}\" cut", start(words(element))),
            });
        }
    }
    for element in elements {
        let Some(siblings) = children.get(element.path.as_str()) else {
            continue;
        };
        let content = |element: &ElementInfo| {
            holds.get(element.path.as_str()).copied().unwrap_or(false)
                && !rules.floating(&element.path)
        };
        for (at, one) in siblings.iter().enumerate() {
            for other in &siblings[at + 1..] {
                let (w, h) = shared(one.rect, other.rect);
                if content(one) && content(other) && w > SLACK && h > SLACK {
                    problems.push(Problem {
                        kind: Kind::Overlap,
                        path: format!("{} and {}", one.path, key_of(&other.path)),
                        detail: format!("{w:.1} × {h:.1} pt shared"),
                    });
                }
            }
        }
    }
    problems
}

/// The faults only the glyphs show: one drawn outside every text's box,
/// one cut by a box that is no scroll container. One problem a text.
fn ink(laid: &Laid, rules: Rules) -> Vec<Problem> {
    // What draws glyphs: a text (a field's placeholder while it is empty),
    // or an icon (no text, and a colour for it).
    let texts: Vec<&ElementInfo> = laid
        .elements
        .iter()
        .filter(|element| {
            drawn(element) && (element.text.is_some() || element.text_color.is_some())
        })
        .collect();
    let scrolls: Vec<[f32; 4]> = laid
        .elements
        .iter()
        .filter(|element| rules.scroll(&element.path))
        .map(|element| element.visible)
        .collect();
    // The text a glyph belongs to: the one whose box holds its middle,
    // else the nearest.
    let owner = |glyph: &Glyph| {
        let middle = [
            glyph.rect[0] + glyph.rect[2] / 2.0,
            glyph.rect[1] + glyph.rect[3] / 2.0,
            0.0,
            0.0,
        ];
        texts
            .iter()
            .map(|text| (leaves(middle, text.rect), *text))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    };
    let mut found: Vec<Problem> = Vec::new();
    let mut add = |kind: Kind, text: &ElementInfo, detail: String| {
        let known = found
            .iter()
            .any(|problem| problem.kind == kind && problem.path == text.path);
        if !known && !rules.floating(&text.path) {
            found.push(Problem {
                kind,
                path: text.path.clone(),
                detail,
            });
        }
    };
    let mut shown: HashMap<&str, usize> = HashMap::new();
    for glyph in &laid.glyphs {
        let Some((away, text)) = owner(glyph) else {
            continue;
        };
        *shown.entry(text.path.as_str()).or_default() += 1;
        // A text squeezed to nothing has no box to be outside of: it is
        // reported below, as cut.
        if !shows(text) {
            continue;
        }
        if away > INK {
            add(
                Kind::Outside,
                text,
                format!(
                    "a glyph at {:?} is {away:.1} pt outside the box {:?} of \"{}\"",
                    glyph.rect,
                    text.rect,
                    start(words(text))
                ),
            );
            continue;
        }
        // Cut on a side that is no scroll container's edge.
        let cut = sides(glyph.rect, glyph.clip);
        let edges = |scroll: &[f32; 4]| [scroll[0], scroll[1], right(*scroll), bottom(*scroll)];
        let clip = edges(&glyph.clip);
        let meant = |side: usize| {
            scrolls
                .iter()
                .any(|scroll| (edges(scroll)[side] - clip[side]).abs() <= SLACK + 0.5)
        };
        if let Some(side) = (0..4).find(|&side| cut[side] > SLACK && !meant(side)) {
            add(
                Kind::Clipped,
                text,
                format!(
                    "{:.1} pt of a glyph of \"{}\" cut",
                    cut[side],
                    start(words(text))
                ),
            );
        }
    }
    // Texts cut by their own "…" to less than half of their letters.
    for text in &texts {
        let letters = words(text).chars().filter(|c| !c.is_whitespace()).count();
        let drawn = shown.get(text.path.as_str()).copied().unwrap_or(0);
        // Cut by a box above it: a scroll container's business, or
        // reported as that already.
        let whole = leaves(text.rect, text.visible) <= SLACK;
        if letters >= READ && whole && drawn * 2 < letters && !rules.cut(&text.path) {
            add(
                Kind::Clipped,
                text,
                format!(
                    "{drawn} of the {letters} letters of \"{}\" shown",
                    start(words(text))
                ),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use rune::text::TextSystem;
    use rune::{Cx, El, Env, TextStyle, col, row, text};

    use super::*;

    /// A card 200 wide; `shape` picks what is wrong inside it.
    fn view(shape: &&'static str, _: &mut Cx<&'static str>) -> El {
        let font = TextStyle::new(13.0, 400.0);
        let long = "Donaudampfschifffahrtsgesellschaftskapitän";
        let card = col().key("card").w(200.0).gap(4.0);
        let card = match *shape {
            "fits" => card
                .child(text("Title").key("title").font(font))
                .child(text(long).key("words").font(font).wrap_anywhere().w_full()),
            // A box wider than the card.
            "outside" => card.child(
                row()
                    .key("wide")
                    .w(260.0)
                    .h(20.0)
                    .child(text("Title").font(font)),
            ),
            // One unbreakable word: its box is the card's, its glyphs not.
            "word" => card.child(text(long.repeat(2)).key("title").font(font)),
            // The same, cut by the card.
            "clipped" => card
                .clip_rounded(0.0)
                .child(text(long.repeat(2)).key("title").font(font)),
            // A hint placed over the end of the title.
            "overlap" => card.child(
                row()
                    .key("line")
                    .w_full()
                    .h(20.0)
                    .child(text("A title of some length").key("title").font(font))
                    .child(text("hint").key("hint").font(font).absolute(100.0, 0.0)),
            ),
            // A list longer than its scroll: cut on purpose.
            "scroll" => card.child(
                col()
                    .key("scroll")
                    .w_full()
                    .h(30.0)
                    .scroll(rune::ScrollSpec {
                        stick_bottom: false,
                    })
                    .children((0..8).map(|n| text(format!("Row {n}")).font(font))),
            ),
            // The word cut by itself: an ellipsis, most of it left.
            "truncated" => card.child(
                row()
                    .w_full()
                    .child(text(long).key("title").font(font).truncate().shrink()),
            ),
            // The same in a quarter of the room: three dots and little else.
            "squeezed" => card.child(
                row()
                    .key("row")
                    .w(60.0)
                    .child(text(long).key("title").font(font).truncate().shrink()),
            ),
            other => panic!("no shape {other}"),
        };
        col().key("window").size(400.0, 300.0).child(card)
    }

    fn kinds(shape: &'static str, rules: Rules) -> Vec<Kind> {
        let env = Env {
            system_dark: false,
            scale: 2.0,
        };
        let fonts = TextSystem::new(rune_theme_opencode::FONTS, rune_theme_opencode::FAMILIES);
        let mut rt = Runtime::with_text(shape, view, (400.0, 300.0), env, fonts);
        rt.settle_all();
        rt.frame(0.0);
        let found = problems(&Laid::of(&rt, 2.0), rules);
        let mut kinds = Vec::new();
        for problem in found {
            if !kinds.contains(&problem.kind) {
                kinds.push(problem.kind);
            }
        }
        kinds
    }

    /// Each fault is found as its own kind, and what is cut or placed on
    /// purpose is none.
    #[test]
    fn each_fault_is_found_and_what_is_meant_is_not() {
        let scrolls = Rules {
            scrolls: &["scroll"],
            ..Rules::default()
        };
        let named = Rules {
            floats: &[("hint", "a mark over the title")],
            cuts: &[("card/row/title", "a preview of a long value")],
            ..Rules::default()
        };
        let table: [(&str, Rules, &[Kind]); 11] = [
            ("fits", scrolls, &[]),
            ("outside", scrolls, &[Kind::Outside]),
            ("word", scrolls, &[Kind::Outside]),
            ("clipped", scrolls, &[Kind::Clipped]),
            ("overlap", scrolls, &[Kind::Overlap]),
            ("scroll", scrolls, &[]),
            ("truncated", scrolls, &[]),
            ("squeezed", scrolls, &[Kind::Clipped]),
            // A scroll the rules don't name is a list cut short.
            ("scroll", Rules::default(), &[Kind::Outside, Kind::Clipped]),
            // What is placed over its box by design is named.
            ("overlap", named, &[]),
            // And so is a text that shows only its start.
            ("squeezed", named, &[]),
        ];
        for (shape, rules, want) in table {
            assert_eq!(kinds(shape, rules), want, "{shape}");
        }
    }
}
