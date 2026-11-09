//! The application menu bar shown while the window is active.

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};

use super::MenuTarget;

/// The menu bar while the window is active: Hex, File, Edit, View (with
/// the Labs), Window.
/// Edit's items have no target: the window's view answers them (Rune's
/// `standard_menus`, see `rune_ui::build`) by handing its field the
/// ⌘-key itself. AppKit matches a menu's key equivalents under any layout,
/// so ⌘C/⌘V work with a Cyrillic layout, where the key alone reads as
/// ⌘с/⌘м and Rune's fields would ignore it.
pub(super) fn install_main_menu(target: &MenuTarget, mtm: MainThreadMarker) {
    let main = NSMenu::new(mtm);
    let name = crate::app_paths::app_name();
    let (hide, quit) = (format!("Hide {name}"), format!("Quit {name}"));
    let mut application = vec![];
    if !crate::updater::managed_by_policy() {
        application.push(Some(("Check for Updates…", sel!(checkForUpdates:), "")));
        application.push(None);
    }
    application.extend([
        Some(("Settings…", sel!(showSettings:), ",")),
        None,
        Some((hide.as_str(), sel!(hideHex:), "h")),
        None,
        Some((quit.as_str(), sel!(quitHex:), "q")),
    ]);
    let mut menus = vec![
        (name, application),
        (
            "File",
            vec![Some(("Close Window", sel!(closeWindow:), "w"))],
        ),
    ];
    let edit_menu = edit_menu(mtm);
    // Rune's own View › Playground item, which this menu bar replaces. A
    // capital letter adds Shift: ⇧⌘D. It opens the playground only while
    // Labs › Playground is on.
    menus.push((
        "View",
        vec![Some(("Playground", sel!(togglePlayground:), "D"))],
    ));
    menus.push((
        "Window",
        vec![Some(("Minimize", sel!(minimizeWindow:), "m"))],
    ));
    for (title, items) in menus {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
        for item in items {
            match item {
                Some((title, action, key)) => {
                    menu.addItem(&menu_item(target, title, action, key, mtm));
                }
                None => menu.addItem(&NSMenuItem::separatorItem(mtm)),
            }
        }
        let heading = NSMenuItem::new(mtm);
        heading.setSubmenu(Some(&menu));
        main.addItem(&heading);
        if title == "File" {
            let heading = NSMenuItem::new(mtm);
            heading.setSubmenu(Some(&edit_menu));
            main.addItem(&heading);
        }
    }
    NSApplication::sharedApplication(mtm).setMainMenu(Some(&main));
}

/// Undo, Redo, Cut, Copy, Paste, and Select All for the focused text
/// field, sent to the first responder. A capital letter adds Shift.
fn edit_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Edit"));
    let items = [
        Some(("Undo", sel!(undo:), "z")),
        Some(("Redo", sel!(redo:), "Z")),
        None,
        Some(("Cut", sel!(cut:), "x")),
        Some(("Copy", sel!(copy:), "c")),
        Some(("Paste", sel!(paste:), "v")),
        Some(("Select All", sel!(selectAll:), "a")),
    ];
    for item in items {
        let Some((title, action, key)) = item else {
            menu.addItem(&NSMenuItem::separatorItem(mtm));
            continue;
        };
        // SAFETY: a plain menu item with no target: AppKit sends its action
        // along the responder chain to the window's view.
        let entry = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        entry.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        menu.addItem(&entry);
    }
    menu
}

fn menu_item(
    target: &MenuTarget,
    title: &str,
    action: Sel,
    key: &str,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    // SAFETY: a plain menu item whose action the target implements.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    };
    item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
    // SAFETY: the target lives as long as the app (kept in PUMP).
    unsafe { item.setTarget(Some(target)) };
    item
}
