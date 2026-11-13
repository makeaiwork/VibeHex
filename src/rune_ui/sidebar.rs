//! The sidebar: every pane in navigation order, their items, and at the
//! bottom an update to take, Assistant, and Quit on Linux. The version and
//! the update check are About's (`about::update`).

use super::*;

static SETTINGS_ICON: Icon = Icon {
    name: "hex-settings",
    view: 24.0,
    body: include_str!("../../assets/icons/settings.svg"),
};
static MODES_ICON: Icon = Icon {
    name: "hex-modes",
    view: 24.0,
    body: include_str!("../../assets/icons/modes.svg"),
};
static OUTPUT_ICON: Icon = Icon {
    name: "hex-output",
    view: 24.0,
    body: include_str!("../../assets/icons/output.svg"),
};
static HISTORY_ICON: Icon = Icon {
    name: "hex-history",
    view: 24.0,
    body: include_str!("../../assets/icons/history.svg"),
};
static ACTIVITY_ICON: Icon = Icon {
    name: "hex-activity",
    view: 24.0,
    body: include_str!("../../assets/icons/activity.svg"),
};
static HUD_LAB_ICON: Icon = Icon {
    name: "hex-hud-lab",
    view: 24.0,
    body: include_str!("../../assets/icons/hud-lab.svg"),
};
pub(super) static VOICE_FILTER_ICON: Icon = Icon {
    name: "hex-voice-filter",
    view: 24.0,
    body: include_str!("../../assets/icons/voice-filter.svg"),
};
static MEETINGS_ICON: Icon = Icon {
    name: "hex-meetings",
    view: 24.0,
    body: include_str!("../../assets/icons/meetings.svg"),
};
/// A page with a sound wave on it.
static FILES_ICON: Icon = Icon {
    name: "hex-files",
    view: 16.0,
    body: r#"<path d="M4 1.8h5l3.5 3.5v8.9H4z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M9 1.8v3.5h3.5M6.5 9v3M8.3 8v5M10.1 9.5v2" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};
/// A window with its sidebar edge: Close sidebar, and Labs' row.
pub(super) static CLOSE_SIDEBAR_ICON: Icon = Icon {
    name: "hex-close-sidebar",
    view: 16.0,
    body: r#"<rect x="2" y="2.5" width="12" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M6 2.5v11" fill="none" stroke="currentColor" stroke-width="1.3"/>"#,
};
/// The same window with a chevron pointing out of its sidebar.
static OPEN_SIDEBAR_ICON: Icon = Icon {
    name: "hex-open-sidebar",
    view: 16.0,
    body: r#"<rect x="2" y="2.5" width="12" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M6 2.5v11M8.5 6.5 10 8l-1.5 1.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};
#[cfg(hex_capture)]
pub(super) static SCRAWL_ICON: Icon = Icon {
    name: "hex-scrawl",
    view: 24.0,
    body: include_str!("../../assets/icons/scrawl.svg"),
};

/// Every pane, in navigation order. Which ones show depends on the OS's
/// [`platform::Capabilities`] and opt-ins ([`Pane::shown`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Pane {
    /// Dictation, output, and application basics.
    #[default]
    General,
    /// What Hex does to every dictation and in some apps, read-only: the
    /// person's agent edits it.
    Output,
    /// The old per-app editor, kept for its scenes until it is removed.
    Modes,
    /// Recorded calls and their settings, or the opt-in (macOS beta).
    Meetings,
    /// Audio and video files transcribed and reworked (Labs › Files, macOS).
    Files,
    History,
    /// The shots and recordings Labs › Screenshots saved (macOS, while
    /// it's on).
    #[cfg(hex_capture)]
    Captures,
    /// Annotate (Scrawl's captures inside Hex): the switch, shortcuts,
    /// permission, transcriber, folder, retention, and Scrawl's own app.
    /// Dev builds, macOS and Linux only.
    #[cfg(hex_capture)]
    Annotate,
    /// The Hex Microphone's driver, tuning, and try-it demo (macOS, while
    /// its Labs toggle is on).
    VoiceFilter,
    /// The dictation indicator's tuning scene (Playground only).
    HudLab,
    /// Legacy event-log inspector scene (Playground only).
    Activity,
    /// VibeHex, its source, and its developer: after Labs.
    About,
    /// Questions about VibeHex, answered from its guide: at the bottom,
    /// apart from the navigation (on by default, not in Labs).
    Assistant,
}

/// The open pane's actions for its title bar, and its body.
pub(super) fn body(ui: &Ui, p: Paint, reduced: bool, layer: &mut Vec<El>) -> (Vec<El>, El) {
    match ui.pane {
        _ if ui.labs_open => (vec![labs::tabs(ui, p)], labs::page(ui, p)),
        Pane::General => (Vec::new(), general::pane(ui, p, layer)),
        Pane::Output => (Vec::new(), output::pane(ui, p, reduced, layer)),
        Pane::Modes => modes::pane(ui, p, layer),
        Pane::History => history::pane(ui, p, layer),
        #[cfg(hex_capture)]
        Pane::Captures => (Vec::new(), captures::pane(ui, p)),
        Pane::Meetings => meetings::pane(ui, p, layer),
        Pane::Files => files::pane(ui, p, layer),
        #[cfg(hex_capture)]
        Pane::Annotate => (Vec::new(), scrawl::pane(ui, p, layer)),
        Pane::VoiceFilter => (Vec::new(), fun::pane(ui, p)),
        Pane::HudLab => hud_lab::pane(ui, p),
        Pane::Activity => activity::pane(ui, p),
        Pane::About => (Vec::new(), about::pane(ui, p)),
        Pane::Assistant => assistant::pane(ui, p, reduced, layer),
    }
}

impl Pane {
    #[cfg(all(test, target_os = "linux"))]
    pub(super) const ALL: &[Pane] = &[
        Pane::General,
        Pane::Output,
        Pane::Modes,
        Pane::Meetings,
        Pane::Files,
        Pane::History,
        #[cfg(hex_capture)]
        Pane::Captures,
        #[cfg(hex_capture)]
        Pane::Annotate,
        Pane::VoiceFilter,
        Pane::HudLab,
        Pane::Activity,
        Pane::About,
        Pane::Assistant,
    ];

    pub(super) fn shown(self, ui: &Ui) -> bool {
        let caps = ui.platform.caps;
        match self {
            Pane::Modes | Pane::HudLab | Pane::Activity => false,
            Pane::Meetings => caps.meetings && meetings::available(ui),
            Pane::Files => super::files::available(ui),
            // On from Labs (dev builds), or while its driver is installed.
            Pane::VoiceFilter => caps.fun && fun::running(ui),
            #[cfg(hex_capture)]
            Pane::Annotate => ui.platform.scrawl.is_some(),
            // The Lab turns on only on macOS.
            #[cfg(hex_capture)]
            Pane::Captures => ui.settings.labs.screenshots,
            Pane::Assistant => ui.settings.assistant.enabled,
            _ => true,
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Pane::General => tr("General"),
            Pane::Output => tr("Output"),
            Pane::Modes => tr("Modes"),
            Pane::History => tr("History"),
            #[cfg(hex_capture)]
            Pane::Captures => tr("Captures"),
            Pane::Meetings => tr("Meetings"),
            Pane::Files => tr("Files"),
            #[cfg(hex_capture)]
            Pane::Annotate => tr("Annotate"),
            Pane::VoiceFilter => tr("Voice Filter"),
            Pane::HudLab => "HUD Lab",
            Pane::Activity => tr("Activity"),
            Pane::About => tr("About"),
            Pane::Assistant => tr("Assistant"),
        }
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            Pane::General => "nav-general",
            Pane::Output => "nav-output",
            Pane::Modes => "nav-modes",
            Pane::History => "nav-history",
            #[cfg(hex_capture)]
            Pane::Captures => "nav-captures",
            Pane::Meetings => "nav-meetings",
            Pane::Files => "nav-files",
            #[cfg(hex_capture)]
            Pane::Annotate => "nav-annotate",
            Pane::VoiceFilter => "nav-voice-filter",
            Pane::HudLab => "nav-hud-lab",
            Pane::Activity => "nav-activity",
            Pane::About => "nav-about",
            Pane::Assistant => "nav-assistant",
        }
    }

    pub(super) fn icon(self) -> &'static Icon {
        match self {
            Pane::General => &SETTINGS_ICON,
            Pane::Output => &OUTPUT_ICON,
            Pane::Modes => &MODES_ICON,
            Pane::History => &HISTORY_ICON,
            #[cfg(hex_capture)]
            Pane::Captures => &captures::CAPTURES_ICON,
            Pane::Meetings => &MEETINGS_ICON,
            Pane::Files => &FILES_ICON,
            #[cfg(hex_capture)]
            Pane::Annotate => &SCRAWL_ICON,
            Pane::VoiceFilter => &VOICE_FILTER_ICON,
            Pane::HudLab => &HUD_LAB_ICON,
            Pane::Activity => &ACTIVITY_ICON,
            Pane::About => &super::icons::INFO,
            Pane::Assistant => &super::assistant::face::FACE,
        }
    }
}

/// macOS draws the traffic lights over the sidebar (Rune's window has a
/// transparent, full-size title bar), so the navigation starts below them.
const SIDEBAR_TOP: f32 = if cfg!(target_os = "macos") {
    40.0
} else {
    12.0
};

/// Sidebar rows follow OpenCode Desktop's settings navigation: 28 pt rows
/// with a 4 pt gap, so each row's highlight stays its own.
pub(super) const NAV_ROW_HEIGHT: f32 = 28.0;
pub(super) const NAV_ROW_GAP: f32 = 4.0;
/// Between groups: the brand and the items, Assistant and Quit.
const GROUP_GAP: f32 = 10.0;

/// The logo's top below the sidebar's top edge, expanded or folded (41 pt
/// on macOS, 13 elsewhere): the brand row's top, and the mark centred in
/// it. The bottom item's glyph keeps the same distance from the bottom edge.
pub(super) const LOGO_INSET: f32 =
    SIDEBAR_TOP - NAV_ROW_GAP / 2.0 + (super::brand::HEADER_HEIGHT - super::brand::MARK_SIZE) / 2.0;

/// The expanded sidebar's bottom padding. Assistant's face sits as far
/// above the bottom edge as the logo below the top. Linux ends with Quit,
/// a row of words: its row stands as far from the bottom as the brand row
/// from the top.
fn bottom_inset(ui: &Ui) -> f32 {
    if !host::QUIT_IN_WINDOW && Pane::Assistant.shown(ui) {
        LOGO_INSET - super::assistant::nav::SLOT_FACE_INSET
    } else {
        SIDEBAR_TOP - NAV_ROW_GAP / 2.0
    }
}

/// On macOS the sidebar and the pane header stand in for the title bar the
/// window draws none of: pressing their background moves the window.
pub(super) fn title_bar(element: El) -> El {
    if cfg!(target_os = "macos") {
        element.window_drag()
    } else {
        element
    }
}

/// The title bar of the content panel: the title, a view switch that stays
/// beside it, and the pane's actions on the right. The title is the pane's
/// level 1 heading. `back`: Back to Assistant, before the title, when the
/// pane was opened from an answer.
pub(super) fn pane_header(
    p: Paint,
    back: Option<El>,
    title: &str,
    switch: Option<El>,
    actions: Vec<El>,
) -> El {
    title_bar(row())
        .key("pane-header")
        .h(52.0)
        .px(METRICS.gutter)
        .gap(8.0)
        .center_y()
        // Back to Assistant is a box: it hangs past the gutter as fields
        // and cards do, so its edge lines up with theirs below it.
        .children(back.map(|back| back.ml(-METRICS.row_px).mr(4.0)))
        .child(
            text(title)
                .key("pane-title")
                .role("heading")
                .level(1)
                .font(p.k.ty.body_medium)
                .color(p.text())
                .shrink()
                .truncate(),
        )
        .children(switch.map(|switch| switch.ml(8.0)))
        .child(el().key("header-spacer").grow(1.0))
        .children(actions)
}

/// `still`: Reduce Motion is on, so Assistant's face holds still.
pub(super) fn sidebar(ui: &Ui, p: Paint, still: bool, layer: &mut Vec<El>) -> El {
    if folded(ui) {
        return rail(ui, p, still, layer);
    }
    let mut nav = lists::source_list(col().key("nav"));
    for pane in labs::main_panes(ui) {
        nav = nav.child(nav_slot(
            p,
            pane,
            ui.pane == pane && !ui.labs_open,
            NAV_ROW_GAP / 2.0,
            NAV_ROW_GAP / 2.0,
        ));
    }
    nav = nav.children(labs::playground_nav(ui, p));
    nav = nav.child(labs::nav(ui, p));
    nav = nav.child(nav_slot(
        p,
        Pane::About,
        ui.pane == Pane::About && !ui.labs_open,
        NAV_ROW_GAP / 2.0,
        NAV_ROW_GAP / 2.0,
    ));
    title_bar(col())
        .key("sidebar")
        .role("navigation")
        .w(196.0)
        .h_full()
        .pt(SIDEBAR_TOP - NAV_ROW_GAP / 2.0)
        .pb(bottom_inset(ui))
        .pl(10.0)
        .pr(6.0)
        .child(
            row()
                .key("brand-row")
                .mb(GROUP_GAP)
                .center_y()
                .child(super::brand::header(p).grow(1.0).shrink())
                .children(foldable(ui).then(|| close_button(ui, p, layer))),
        )
        .child(nav)
        .child(el().key("spacer").grow(1.0))
        .children(sidebar_update::group(ui, p))
        .children(super::assistant::nav::item(ui, p, still))
        .children(host::QUIT_IN_WINDOW.then(|| quit_item(p)))
}

/// Labs › Collapsible sidebar is on: the sidebar can fold into icons.
fn foldable(ui: &Ui) -> bool {
    crate::labs::Feature::CollapsibleSidebar.on(&ui.settings)
}

/// The sidebar shows as its narrow column of icons.
pub(super) fn folded(ui: &Ui) -> bool {
    foldable(ui) && ui.settings.labs.sidebar_collapsed
}

/// Folds the sidebar into icons, or opens it again, and keeps it so.
fn fold(ui: &mut Ui, folded: bool) {
    ui.sidebar_tip = None;
    ui.update(|settings| settings.labs.sidebar_collapsed = folded);
}

/// The folded sidebar's width, as the mockups draw it.
const RAIL_WIDTH: f32 = 56.0;

/// `trigger` with a tooltip saying `words` while the pointer is on it and
/// no menu is open.
fn tip(
    ui: &Ui,
    p: Paint,
    key: &'static str,
    words: &str,
    placement: rune_kit::Placement,
    trigger: El,
    layer: &mut Vec<El>,
) -> El {
    let trigger = trigger.on_hover_change(move |ui: &mut Ui, inside, _| {
        if inside {
            ui.sidebar_tip = Some(key);
        } else if ui.sidebar_tip == Some(key) {
            ui.sidebar_tip = None;
        }
    });
    rune_kit::Tooltip::new(format!("{key}-tip"))
        .trigger(trigger)
        .text(p.k, words)
        .placement(placement)
        .open(ui.sidebar_tip == Some(key) && ui.open.is_none())
        .build(p.k, layer)
}

/// Close sidebar, at the brand row's right end.
fn close_button(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let button = row()
        .key("sidebar-close")
        .role("button")
        .label(tr("Close sidebar"))
        .size(28.0, 28.0)
        .center()
        .radius(6.0)
        .hover(c.hover)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .child(
            icon(&CLOSE_SIDEBAR_ICON, 16.0)
                .key("icon")
                .color(c.icon_muted),
        )
        .on_press(|ui: &mut Ui| fold(ui, true));
    tip(
        ui,
        p,
        "sidebar-close",
        tr("Close sidebar"),
        rune_kit::Placement::Bottom,
        button,
        layer,
    )
}

/// One of the folded sidebar's 36 by 32 pt buttons: an icon, its name only
/// as a tooltip and the accessible label.
fn rail_item(p: Paint, key: &'static str, glyph: &'static Icon, label: &str, on: bool) -> El {
    let c = &p.k.c;
    let item = row()
        .key(key)
        .role("button")
        .label(label)
        .size(36.0, 32.0)
        .center()
        .radius(7.0)
        .hover(c.hover)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .child(
            icon(glyph, 16.0)
                .key("icon")
                .color(if on { c.icon_base } else { c.icon_muted }),
        );
    if on { item.fill(c.layer_02) } else { item }
}

/// The folded sidebar's top: VibeHex's logo, which turns into Open sidebar
/// while the pointer is on it.
fn open_button(p: Paint) -> El {
    let c = &p.k.c;
    row()
        .key("sidebar-open")
        .role("button")
        .label(tr("Open sidebar"))
        .size(36.0, 32.0)
        .center()
        .radius(7.0)
        .hover_group()
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .child(
            el().key("hover")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(7.0)
                .fill(c.layer_02)
                .group_opacity(0.0, 1.0),
        )
        .child(super::brand::mark(p, "logo", 22.0).group_opacity(1.0, 0.0))
        .child(
            row()
                .key("open")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .center()
                .group_opacity(0.0, 1.0)
                .child(
                    icon(&OPEN_SIDEBAR_ICON, 16.0)
                        .key("icon")
                        .color(c.icon_base),
                ),
        )
}

/// The folded sidebar: Open sidebar, every pane, Labs, and About as icons,
/// and at the bottom an update to take and Assistant, whose face sits as
/// far above the bottom edge as the logo below the top.
fn rail(ui: &Ui, p: Paint, still: bool, layer: &mut Vec<El>) -> El {
    let right = rune_kit::Placement::Right;
    let mut rail = title_bar(col())
        .key("sidebar")
        .role("navigation")
        .w(RAIL_WIDTH)
        .h_full()
        .pt(SIDEBAR_TOP - 4.0)
        .pb(LOGO_INSET - super::assistant::nav::RAIL_FACE_INSET)
        .gap(NAV_ROW_GAP)
        .items_center();
    let open = open_button(p).on_press(|ui: &mut Ui| fold(ui, false));
    rail = rail.child(
        tip(
            ui,
            p,
            "sidebar-open",
            tr("Open sidebar"),
            right,
            open,
            layer,
        )
        .mb(10.0),
    );
    let mut panes = labs::main_panes(ui);
    panes.push(Pane::About);
    let about = panes.len() - 1;
    for (index, pane) in panes.into_iter().enumerate() {
        if index == about {
            let on = ui.labs_open;
            let item = rail_item(p, "nav-labs", &labs::LABS_ICON, tr("Labs"), on)
                .on_press_tx(labs::press_nav);
            rail = rail.child(tip(ui, p, "nav-labs", tr("Labs"), right, item, layer));
        }
        let on = ui.pane == pane && !ui.labs_open;
        let item = rail_item(p, pane.key(), pane.icon(), pane.title(), on)
            .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| ui.select_pane(pane, tx));
        rail = rail.child(tip(ui, p, pane.key(), pane.title(), right, item, layer));
    }
    rail = rail.child(el().key("spacer").grow(1.0));
    rail = rail.children(update_item(ui, p, layer));
    rail.children(super::assistant::nav::rail_item(ui, p, still, layer))
}

/// The folded sidebar's update button, while there is an update to take:
/// in the accent's dot, its words as the tooltip. The check is About's.
fn update_item(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Option<El> {
    let (words, action) = sidebar_update::action(ui)?;
    let item = rail_item(p, "sidebar-update", &super::icons::REFRESH, &words, false)
        .on_press(move |ui: &mut Ui| ui.send(Request::Desktop(action.clone())))
        .child(
            controls::status_dot(p.accent())
                .key("dot")
                .pin(None, Some(5.0), Some(5.0), None),
        );
    let right = rune_kit::Placement::Right;
    Some(tip(ui, p, "sidebar-update", &words, right, item, layer))
}

/// A sidebar item inside its slot. Slots tile the item column, each
/// reaching halfway into the gaps around its item, so wherever the pointer
/// is in the column, the nearest item shows its hover, and a press there
/// selects it.
fn nav_slot(p: Paint, pane: Pane, on: bool, above: f32, below: f32) -> El {
    col()
        .key(format!("{}-slot", pane.key()))
        // Assistive technology sees the item alone.
        .role("none")
        .pt(above)
        .pb(below)
        .hover_group()
        .on_drag(move |ui: &mut Ui, d: &mut rune::Drag, tx: &mut Tx| {
            select_on_down(ui, d, tx, pane)
        })
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| ui.select_pane(pane, tx))
        .child(nav_item(p, pane, on))
}

/// Switch on pointer down, like a native sidebar; the press keeps keyboard
/// activation.
fn select_on_down(ui: &mut Ui, d: &mut rune::Drag, tx: &mut Tx, pane: Pane) {
    if d.phase == rune::Phase::Down {
        ui.select_pane(pane, tx);
    }
}

fn nav_item(p: Paint, pane: Pane, on: bool) -> El {
    let c = &p.k.c;
    // A row of the navigation list, selected while its pane shows.
    let mut item = row()
        .key(pane.key())
        .role("listitem")
        .label(pane.title())
        .h(NAV_ROW_HEIGHT)
        .px(6.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.sm)
        .press(c.pressed)
        .on_drag(move |ui: &mut Ui, d: &mut rune::Drag, tx: &mut Tx| {
            select_on_down(ui, d, tx, pane)
        })
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| ui.select_pane(pane, tx))
        // The slot's hover, drawn on the item.
        .child(
            el().key("hover")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(p.k.radius.sm)
                .fill(c.hover)
                .group_opacity(0.0, 1.0)
                .a11y_hidden(),
        )
        .child(
            icon(pane.icon(), 16.0)
                .key("icon")
                .color(if on { c.icon_base } else { c.icon_muted })
                .a11y_hidden(),
        )
        .child(
            text(pane.title())
                .key("label")
                .font(p.k.ty.ui)
                .color(if on { c.text_base } else { c.text_muted })
                .truncate()
                .a11y_hidden(),
        );
    if on {
        item = item.fill(c.layer_02);
    }
    lists::source_item(item, on)
}

/// Linux has no tray: the window's way to stop Hex. Closing the window
/// keeps Hex dictating.
fn quit_item(p: Paint) -> El {
    let c = &p.k.c;
    row()
        .key("quit")
        .role("button")
        .label(tr("Quit VibeHex"))
        .h(NAV_ROW_HEIGHT)
        .px(6.0)
        // Apart from Assistant's row above, so the bottom isn't one block.
        .mt(GROUP_GAP)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .press(c.pressed)
        .on_press(|ui: &mut Ui| ui.quit())
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .child(
            text(tr("Quit VibeHex"))
                .key("label")
                .font(p.k.ty.ui)
                .color(c.text_muted)
                .truncate()
                .a11y_hidden(),
        )
}
