//! Assistant on the web, in the pane (`crate::assistant::web`): the field's
//! Web toggle, shown only while the conversation runs on Codex, the only
//! engine that searches; the steps an answer took there; and its "From the
//! web" chips, one per site, which open their page in the browser.

use rune::{El, col, icon, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::icons;
use crate::app_settings::AppSettings;
use crate::assistant::guide;
use crate::assistant::store::Turn;
use crate::assistant::web::site;
use crate::assistant::worker::Answering;
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::files::rewrite::Engine;

/// The guide articles a web answer's steps name at most.
const GUIDE_TITLES: usize = 2;

/// Whether a conversation's answers may search the web: it runs on Codex,
/// and its own Web choice, else Assistant › Settings', is on.
pub(super) fn on(settings: &AppSettings, chosen: Option<bool>, engine: &Engine) -> bool {
    engine.searches_web() && chosen.unwrap_or(settings.assistant.web)
}

/// The open conversation's Web choice, else the next new one's.
pub(super) fn chosen(ui: &Ui) -> Option<bool> {
    let pane = &ui.assistant;
    match pane.conversation() {
        Some(conversation) => conversation.web,
        None => pane.web,
    }
}

/// The Web toggle after the model and reasoning pills; none for an engine
/// that can't search.
pub(super) fn pill(ui: &Ui, p: Paint, engine: &Engine) -> Option<El> {
    if !engine.searches_web() {
        return None;
    }
    let c = &p.k.c;
    let lit = on(&ui.settings, chosen(ui), engine);
    let ink = if lit {
        p.accent()
    } else {
        rune::theme::for_blend(c.text_base.alpha(0.58), c.bg_base)
    };
    Some(
        row()
            .key("web")
            .role("button")
            .label(tr("Web"))
            .checked(lit)
            .help(if lit {
                tr("Searches the web when the guide has no answer")
            } else {
                tr("Answers from the guide only")
            })
            .focusable(true)
            .focus_ring(2.0, 0.0, c.border_focus)
            .h(24.0)
            .px(6.0)
            .gap(4.0)
            .center_y()
            .radius(6.0)
            .hover(c.hover)
            .press(c.pressed)
            .child(icon(&icons::GLOBE, 13.0).key("glyph").color(ink))
            .child(
                text(tr("Web"))
                    .key("label")
                    .font(rune::TextStyle {
                        size: 12.0,
                        ..p.k.ty.ui
                    })
                    .color(ink)
                    .max_lines(1),
            )
            .on_press(move |ui: &mut Ui| toggle(ui, !lit)),
    )
}

/// Saves the Web choice: with the open conversation, or for the next one.
fn toggle(ui: &mut Ui, web: bool) {
    ui.open = None;
    let pane = &mut ui.assistant;
    match pane.open {
        Some(id) => pane.store.set_web(id, Some(web)),
        None => pane.web = Some(web),
    }
}

/// While it answers: each search and page with a check, then what it is
/// doing. None before it goes online.
pub(super) fn live(p: Paint, answering: &Answering) -> Option<El> {
    let web = &answering.web;
    if web.is_empty() {
        return None;
    }
    let mut list = col().key("web-steps").gap(6.0);
    for (index, query) in web.searches.iter().enumerate() {
        list = list.child(super::recall::step_line(
            p,
            &format!("search{index}"),
            true,
            &tr_fill("Searched the web for “{}”", query),
        ));
    }
    for (index, page) in web.pages.iter().enumerate() {
        list = list.child(super::recall::step_line(
            p,
            &format!("page{index}"),
            true,
            &tr_fill("Read {}", site(page)),
        ));
    }
    let doing = if web.busy() {
        tr("Searching the web")
    } else if answering.draft.trim().is_empty() && answering.steps.is_empty() {
        tr("Writing the answer")
    } else {
        return Some(list);
    };
    Some(list.child(super::recall::step_line(p, "doing", false, doing)))
}

/// The folded line of an answer that went online: "Read the guide and 2
/// web pages in 9 s".
pub(super) fn folded(turn: &Turn, seconds: &str) -> String {
    match turn.web.read() {
        0 => tr_fill("Read the guide and searched the web in {} s", seconds),
        1 => tr_fill("Read the guide and 1 web page in {} s", seconds),
        pages => tr_named(
            "Read the guide and {pages} web pages in {seconds} s",
            &[("pages", &pages.to_string()), ("seconds", seconds)],
        ),
    }
}

/// The open steps of an answer that went online: the guide it read, that
/// the guide didn't cover it all, and what the web gave, with each search.
pub(super) fn steps(p: Paint, turn: &Turn) -> Vec<El> {
    let web = &turn.web;
    if web.is_empty() {
        return Vec::new();
    }
    let c = &p.k.c;
    let mut lines = Vec::new();
    // The articles the answer names, else the first ones it read.
    let cited: Vec<&str> = turn.sources.iter().map(String::as_str).collect();
    let read: Vec<&str> = web.guide.iter().map(String::as_str).collect();
    let ids = if cited.is_empty() { read } else { cited };
    let titles: Vec<&str> = ids
        .into_iter()
        .filter_map(guide::article)
        .map(|article| article.title.as_str())
        .take(GUIDE_TITLES)
        .collect();
    if !titles.is_empty() {
        lines.push(line(
            p,
            "guide",
            (&icons::BOOK, c.icon_muted),
            tr("Searched the guide:"),
            Some(titles.join(", ")),
        ));
    }
    lines.push(line(
        p,
        "missing",
        (&rune_theme_opencode::icons::CIRCLE_EXCLAMATION, p.warning()),
        tr("The guide doesn't cover all of it"),
        None,
    ));
    let found = match (web.read(), web.searches.len()) {
        (1, _) => tr("1 page read").to_owned(),
        (0, 1) => tr("1 search").to_owned(),
        (0, searches) => tr_fill("{} searches", &searches.to_string()),
        (pages, _) => tr_fill("{} pages read", &pages.to_string()),
    };
    lines.push(line(
        p,
        "web",
        (&icons::GLOBE, c.icon_muted),
        tr("Searched the web:"),
        Some(found),
    ));
    // What it searched for, so the person sees nothing of theirs went out.
    for (index, query) in web.searches.iter().enumerate() {
        lines.push(
            text(format!("“{query}”"))
                .key(format!("query{index}"))
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap()
                .pl(20.0),
        );
    }
    lines
}

/// One step: its icon, its words, and what it found, brighter.
fn line(
    p: Paint,
    key: &str,
    (glyph, tint): (&'static rune::Icon, rune::Color),
    words: &str,
    found: Option<String>,
) -> El {
    let missing = key == "missing";
    let mut words_row = row().key("words").gap(4.0).shrink().flex_wrap().child(
        text(words.to_owned())
            .key("what")
            .font(p.k.ty.small)
            .color(if missing { p.warning() } else { p.faint() }),
    );
    if let Some(found) = found {
        words_row = words_row.child(
            text(found)
                .key("found")
                .font(p.k.ty.small)
                .color(p.muted())
                .wrap()
                .shrink(),
        );
    }
    row()
        .key(key.to_owned())
        .gap(8.0)
        .center_y()
        .child(icon(glyph, 13.0).key("mark").color(tint))
        .child(words_row)
}

/// "From the web" and a chip per site the answer cites, wrapping as one:
/// the site's name, the whole address as its description and tooltip, and
/// a press opens the page. A soft warm tint sets them apart from the guide.
pub(super) fn chips(p: Paint, sources: &[String]) -> Option<El> {
    if sources.is_empty() {
        return None;
    }
    let c = &p.k.c;
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    let warm = c.warning.fg;
    let mut chips = row()
        .key("web-sources")
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .child(
            text(tr("From the web"))
                .key("label")
                .font(small)
                .color(p.faint()),
        );
    for (index, url) in sources.iter().enumerate() {
        let name = site(url).to_owned();
        chips = chips.child(
            row()
                .key(format!("web{index}"))
                .href(url.clone())
                .label(name.clone())
                .description(url.clone())
                .help(url.clone())
                .focusable(true)
                .focus_ring(2.0, 0.0, c.border_focus)
                .h(22.0)
                .px(7.0)
                .gap(5.0)
                .center_y()
                .radius(p.k.radius.sm)
                .fill(p.k.alpha(warm, 0.08))
                .border(p.hair, p.k.alpha(warm, 0.28))
                .hover(p.k.alpha(warm, 0.16))
                .press(p.k.alpha(warm, 0.22))
                .child(icon(&icons::GLOBE, 12.0).key("glyph").color(warm))
                .child(text(name).key("site").font(small).color(warm).max_lines(1)),
        );
    }
    Some(chips)
}
