use std::ffi::{CStr, c_char, c_void};
use std::ptr;

type CfTypeRef = *const c_void;
type CfStringRef = *const c_void;
type AxUiElementRef = *const c_void;

const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const MESSAGING_TIMEOUT_SECS: f32 = 0.25;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AxUiElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AxUiElementRef,
        attribute: CfStringRef,
        value: *mut CfTypeRef,
    ) -> i32;
    fn AXUIElementSetMessagingTimeout(element: AxUiElementRef, timeout_in_seconds: f32) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AxUiElementRef,
        attribute: CfStringRef,
        value: CfTypeRef,
    ) -> i32;
    fn AXUIElementIsAttributeSettable(
        element: AxUiElementRef,
        attribute: CfStringRef,
        settable: *mut u8,
    ) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AxUiElementRef,
        parameterized_attribute: CfStringRef,
        parameter: CfTypeRef,
        result: *mut CfTypeRef,
    ) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueCreate(value_type: u32, value: *const c_void) -> CfTypeRef;
    fn AXValueGetValue(value: CfTypeRef, value_type: u32, value_out: *mut c_void) -> bool;
}

/// `kAXValueTypeCFRange`.
const AX_VALUE_CF_RANGE: u32 = 4;
/// `kAXValueTypeCGRect`.
const AX_VALUE_CG_RECT: u32 = 3;

/// Core Graphics' `CGRect`: origin, then size, in points.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CgRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Core Foundation's `CFRange`, in UTF-16 code units.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CfRange {
    location: isize,
    length: isize,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFGetTypeID(value: CfTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringGetLength(value: CfStringRef) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(
        value: CfStringRef,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        string: *const c_char,
        encoding: u32,
    ) -> CfStringRef;
    fn CFURLGetTypeID() -> usize;
    fn CFURLGetString(url: CfTypeRef) -> CfStringRef;
    fn CFRelease(value: CfTypeRef);
    fn CFRetain(value: CfTypeRef) -> CfTypeRef;
    fn CFEqual(left: CfTypeRef, right: CfTypeRef) -> u8;
    fn CFArrayGetTypeID() -> usize;
    fn CFArrayGetCount(array: CfTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CfTypeRef, index: isize) -> CfTypeRef;
    static kCFBooleanTrue: CfTypeRef;
    fn CFNumberGetTypeID() -> usize;
    fn CFNumberGetValue(number: CfTypeRef, number_type: isize, value: *mut c_void) -> u8;
}

/// `kCFNumberSInt64Type`.
const CF_NUMBER_SINT64: isize = 4;

pub fn focused_window_title(pid: i32) -> Option<String> {
    let application = foreign_application(pid)?;
    let window = copy_element(&application, c"AXFocusedWindow")?;
    copy_string(&window, c"AXTitle").filter(|title| !title.is_empty())
}

/// The page address of the web area holding the application's focused
/// element, for browsers without a scripting dictionary (Firefox, Zen).
/// The outermost web area wins, so focus inside an iframe still reports
/// the page itself.
pub fn focused_web_url(pid: i32) -> Option<String> {
    const MAX_DEPTH: usize = 64;
    let application = foreign_application(pid)?;
    let mut element = copy_element(&application, c"AXFocusedUIElement")?;
    let mut url = None;
    for _ in 0..MAX_DEPTH {
        if copy_string(&element, c"AXRole").as_deref() == Some("AXWebArea") {
            url = copy_url(&element, c"AXURL").or(url);
        }
        match copy_element(&element, c"AXParent") {
            Some(parent) => element = parent,
            None => break,
        }
    }
    url
}

/// Every page a browser's windows show, for a call's address or tab title
/// when its scripting dictionary can't be asked (#831). Chromium (Arc,
/// Chrome, Brave, Edge) builds its web accessibility tree only for an
/// assistive app that asks, which `AXManualAccessibility` does, once per
/// process; without it a browser behind another app shows windows but no
/// web areas.
pub fn browser_pages(pid: i32) -> crate::dictation::browser_page::BrowserPages {
    let Some(application) = foreign_application(pid) else {
        return Default::default();
    };
    request_web_accessibility(pid, &application);
    let mut windows: Vec<CfObject> = Vec::new();
    let candidates = [
        copy_element(&application, c"AXFocusedWindow"),
        copy_element(&application, c"AXMainWindow"),
    ]
    .into_iter()
    .flatten()
    .chain(copy_elements(&application, c"AXWindows"));
    for window in candidates {
        // SAFETY: Both are live AXUIElements.
        if !windows
            .iter()
            .any(|known| unsafe { CFEqual(known.0, window.0) } != 0)
        {
            windows.push(window);
        }
    }
    let mut pages = crate::dictation::browser_page::pages(windows.into_iter().map(Element));
    // The focused web area, when there is one, is the likeliest page.
    if let Some(url) = focused_web_url(pid) {
        pages.urls.retain(|known| known != &url);
        pages.urls.insert(0, url);
    }
    pages
}

/// Asks a Chromium browser for its web accessibility tree, once per
/// process. Other apps ignore the attribute.
fn request_web_accessibility(pid: i32, application: &CfObject) {
    static ASKED: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());
    let mut asked = ASKED.lock().unwrap_or_else(|error| error.into_inner());
    if asked.contains(&pid) {
        return;
    }
    asked.push(pid);
    let Some(attribute) = cf_string_literal(c"AXManualAccessibility").map(CfObject) else {
        return;
    };
    // SAFETY: A live application element, a CFString name, and the shared
    // kCFBooleanTrue constant.
    let status =
        unsafe { AXUIElementSetAttributeValue(application.0, attribute.0, kCFBooleanTrue) };
    if status != 0 {
        tracing::debug!(
            status,
            pid,
            "the browser has no manual accessibility switch"
        );
    }
}

/// An element as `browser_page` walks it.
struct Element(CfObject);

impl crate::dictation::browser_page::AxNode for Element {
    fn role(&self) -> Option<String> {
        copy_string(&self.0, c"AXRole")
    }

    fn title(&self) -> Option<String> {
        copy_string(&self.0, c"AXTitle")
    }

    fn url(&self) -> Option<String> {
        copy_url(&self.0, c"AXURL")
    }

    fn children(&self) -> Vec<Self> {
        copy_elements(&self.0, c"AXChildren")
            .into_iter()
            .map(Element)
            .collect()
    }
}

/// An attribute holding elements (`AXWindows`, `AXChildren`), each bounded.
fn copy_elements(element: &CfObject, name: &CStr) -> Vec<CfObject> {
    let Some(array) = copy_attribute(element, name) else {
        return Vec::new();
    };
    if unsafe { CFGetTypeID(array.0) } != unsafe { CFArrayGetTypeID() } {
        return Vec::new();
    }
    let count = unsafe { CFArrayGetCount(array.0) };
    (0..count)
        .filter_map(|index| {
            // SAFETY: Get rule: the array owns the item, so it is retained
            // before the array goes.
            let item = unsafe { CFArrayGetValueAtIndex(array.0, index) };
            if item.is_null() {
                return None;
            }
            let item = CfObject(unsafe { CFRetain(item) });
            (unsafe { AXUIElementSetMessagingTimeout(item.0, MESSAGING_TIMEOUT_SECS) } == 0)
                .then_some(item)
        })
        .collect()
}

/// The character just before the insertion point of the application's
/// focused text element, for Labs › Smart spacing. `None` when text is
/// selected, the cursor is at the start, or the app doesn't say.
pub fn char_before_caret(pid: i32) -> Option<char> {
    let application = foreign_application(pid)?;
    let focused = copy_element(&application, c"AXFocusedUIElement")?;
    let selection = copy_range(&focused, c"AXSelectedTextRange")?;
    if selection.length != 0 || selection.location <= 0 {
        return None;
    }
    // One UTF-16 unit, or two when it is half of a surrogate pair.
    (1..=2.min(selection.location)).find_map(|units| {
        let range = CfRange {
            location: selection.location - units,
            length: units,
        };
        copy_string_for_range(&focused, range)?.chars().next_back()
    })
}

/// Text selected in an application's focused element, kept so Labs › Add
/// to vocabulary can put the right spelling in its place.
pub struct Selection {
    element: CfObject,
    /// The application it is in.
    pub pid: i32,
    /// What is selected, trimmed.
    pub text: String,
    /// Whether the element is text that can be edited: a text field or
    /// area, or one whose selection or value takes new text. A message
    /// already sent is neither.
    pub editable: bool,
    /// Where the selected text is on screen, `[x, y, width, height]` in
    /// global top-left points, when the app says.
    pub bounds: Option<[f64; 4]>,
    /// The text just before and just after the selection, when the app
    /// says: which of several dictations it came from.
    pub around: Option<(String, String)>,
}

// SAFETY: An AXUIElement is an immutable Core Foundation reference to
// another process's element; the Accessibility API may be called on it from
// any thread.
unsafe impl Send for Selection {}

impl Selection {
    /// Whether the same element still has the same text selected, so a
    /// paste replaces exactly it.
    pub fn still_selected(&self) -> bool {
        let Some(application) = foreign_application(self.pid) else {
            return false;
        };
        let Some(focused) = copy_element(&application, c"AXFocusedUIElement") else {
            return false;
        };
        // SAFETY: Two live Core Foundation objects.
        (unsafe { CFEqual(focused.0, self.element.0) } != 0)
            && copy_string(&focused, c"AXSelectedText").is_some_and(|text| text.trim() == self.text)
    }
}

/// The roles of text a person types in.
const TEXT_ROLES: [&str; 4] = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"];

/// The text selected in application `pid`'s focused element, through
/// Accessibility, never the clipboard. Chromium-based apps (Electron
/// editors, browsers) build that tree only once asked, as `browser_pages`
/// does. `None` when nothing is selected or the app doesn't say.
pub fn selection(pid: i32) -> Option<Selection> {
    let application = foreign_application(pid)?;
    request_web_accessibility(pid, &application);
    let element = copy_element(&application, c"AXFocusedUIElement")?;
    let text = copy_string(&element, c"AXSelectedText")?;
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let editable = copy_string(&element, c"AXRole")
        .is_some_and(|role| TEXT_ROLES.contains(&role.as_str()))
        || settable(&element, c"AXSelectedText")
        || settable(&element, c"AXValue");
    let bounds = selection_bounds(&element);
    let around = text_around_selection(&element);
    Some(Selection {
        element,
        pid,
        text,
        editable,
        bounds,
        around,
    })
}

/// Up to `AROUND` UTF-16 units either side of `element`'s selection.
fn text_around_selection(element: &CfObject) -> Option<(String, String)> {
    const AROUND: isize = 300;
    let selection = copy_range(element, c"AXSelectedTextRange")?;
    let start = (selection.location - AROUND).max(0);
    let before = copy_string_for_range(
        element,
        CfRange {
            location: start,
            length: selection.location - start,
        },
    )
    .unwrap_or_default();
    let after = copy_string_for_range(
        element,
        CfRange {
            location: selection.location + selection.length,
            length: AROUND,
        },
    )
    // Past the end, an app may refuse the whole range; ask for less.
    .or_else(|| {
        copy_string_for_range(
            element,
            CfRange {
                location: selection.location + selection.length,
                length: AROUND / 10,
            },
        )
    })
    .unwrap_or_default();
    (!before.is_empty() || !after.is_empty()).then_some((before, after))
}

fn settable(element: &CfObject, name: &CStr) -> bool {
    cf_string_literal(name)
        .map(CfObject)
        .is_some_and(|attribute| {
            let mut settable = 0_u8;
            // SAFETY: A live element, a CFString name, and a byte for the Boolean.
            let status =
                unsafe { AXUIElementIsAttributeSettable(element.0, attribute.0, &mut settable) };
            status == 0 && settable != 0
        })
}

/// Whether application `pid`'s ⌘X menu item, Edit › Cut found by its
/// shortcut so any language matches, is enabled: what is selected there
/// can be edited. `None` when the app has no such item or doesn't say.
pub fn can_cut(pid: i32) -> Option<bool> {
    let application = foreign_application(pid)?;
    let bar = copy_element(&application, c"AXMenuBar")?;
    copy_elements(&bar, c"AXChildren")
        .iter()
        .flat_map(|menu| copy_elements(menu, c"AXChildren"))
        .flat_map(|list| copy_elements(&list, c"AXChildren"))
        .find(|item| {
            copy_string(item, c"AXMenuItemCmdChar").as_deref() == Some("X")
                && copy_number(item, c"AXMenuItemCmdModifiers") == Some(0)
        })
        .map(|item| {
            // SAFETY: A live value against the immortal kCFBooleanTrue.
            copy_attribute(&item, c"AXEnabled")
                .is_some_and(|enabled| unsafe { CFEqual(enabled.0, kCFBooleanTrue) } != 0)
        })
}

fn copy_number(element: &CfObject, name: &CStr) -> Option<i64> {
    let value = copy_attribute(element, name)?;
    if unsafe { CFGetTypeID(value.0) } != unsafe { CFNumberGetTypeID() } {
        return None;
    }
    let mut number = 0_i64;
    // SAFETY: A CFNumber read into an i64 of the requested type.
    (unsafe { CFNumberGetValue(value.0, CF_NUMBER_SINT64, (&mut number as *mut i64).cast()) } != 0)
        .then_some(number)
}

/// The selected range's rectangle (`AXBoundsForRange`). Apps that answer
/// with nothing useful (an empty or screen-sized rectangle) count as silent.
fn selection_bounds(element: &CfObject) -> Option<[f64; 4]> {
    let range = copy_range(element, c"AXSelectedTextRange")?;
    // SAFETY: AXValueCreate copies the CFRange and returns a retained object.
    let parameter = unsafe { AXValueCreate(AX_VALUE_CF_RANGE, (&range as *const CfRange).cast()) };
    if parameter.is_null() {
        return None;
    }
    let parameter = CfObject(parameter);
    let attribute = CfObject(cf_string_literal(c"AXBoundsForRange")?);
    let mut value = ptr::null();
    let status = unsafe {
        AXUIElementCopyParameterizedAttributeValue(element.0, attribute.0, parameter.0, &mut value)
    };
    if status != 0 || value.is_null() {
        return None;
    }
    let value = CfObject(value);
    if unsafe { CFGetTypeID(value.0) } != unsafe { AXValueGetTypeID() } {
        return None;
    }
    let mut rect = CgRect::default();
    // SAFETY: `value` is an AXValue and `rect` has CGRect's layout.
    let read =
        unsafe { AXValueGetValue(value.0, AX_VALUE_CG_RECT, (&mut rect as *mut CgRect).cast()) };
    (read && rect.height > 0.0 && rect.height < 600.0 && rect.width < 4000.0).then_some([
        rect.x,
        rect.y,
        rect.width,
        rect.height,
    ])
}

fn copy_range(element: &CfObject, name: &CStr) -> Option<CfRange> {
    let value = copy_attribute(element, name)?;
    if unsafe { CFGetTypeID(value.0) } != unsafe { AXValueGetTypeID() } {
        return None;
    }
    let mut range = CfRange::default();
    // SAFETY: `value` is an AXValue and `range` has CFRange's layout.
    unsafe {
        AXValueGetValue(
            value.0,
            AX_VALUE_CF_RANGE,
            (&mut range as *mut CfRange).cast(),
        )
    }
    .then_some(range)
}

fn copy_string_for_range(element: &CfObject, range: CfRange) -> Option<String> {
    // SAFETY: AXValueCreate copies the CFRange and returns a retained object.
    let parameter = unsafe { AXValueCreate(AX_VALUE_CF_RANGE, (&range as *const CfRange).cast()) };
    if parameter.is_null() {
        return None;
    }
    let parameter = CfObject(parameter);
    let attribute = CfObject(cf_string_literal(c"AXStringForRange")?);
    let mut value = ptr::null();
    let status = unsafe {
        AXUIElementCopyParameterizedAttributeValue(element.0, attribute.0, parameter.0, &mut value)
    };
    if status != 0 || value.is_null() {
        return None;
    }
    let value = CfObject(value);
    cf_string(value.0).filter(|text| !text.is_empty())
}

struct CfObject(CfTypeRef);

impl Drop for CfObject {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

/// The Accessibility application element for another process, never for
/// Hex's own: its window serves that tree from the same runtime its main
/// thread updates, so a read from the foreground-context thread (or Smart
/// spacing) while an update runs can panic and take Hex down (#1402).
fn foreign_application(pid: i32) -> Option<CfObject> {
    if pid == std::process::id() as i32 {
        return None;
    }
    // SAFETY: AXUIElementCreateApplication returns a retained Core Foundation object.
    bounded_element(unsafe { AXUIElementCreateApplication(pid) })
}

fn bounded_element(element: AxUiElementRef) -> Option<CfObject> {
    if element.is_null() {
        return None;
    }
    let element = CfObject(element);
    (unsafe { AXUIElementSetMessagingTimeout(element.0, MESSAGING_TIMEOUT_SECS) } == 0)
        .then_some(element)
}

fn copy_attribute(element: &CfObject, name: &CStr) -> Option<CfObject> {
    let attribute = CfObject(cf_string_literal(name)?);
    let mut value = ptr::null();
    let status = unsafe { AXUIElementCopyAttributeValue(element.0, attribute.0, &mut value) };
    // `then`, not `then_some`: dropping a `CfObject(null)` would CFRelease(NULL).
    (status == 0 && !value.is_null()).then(|| CfObject(value))
}

fn copy_element(element: &CfObject, name: &CStr) -> Option<CfObject> {
    let child = copy_attribute(element, name)?;
    (unsafe { AXUIElementSetMessagingTimeout(child.0, MESSAGING_TIMEOUT_SECS) } == 0)
        .then_some(child)
}

fn copy_string(element: &CfObject, name: &CStr) -> Option<String> {
    let value = copy_attribute(element, name)?;
    cf_string(value.0)
}

fn copy_url(element: &CfObject, name: &CStr) -> Option<String> {
    let value = copy_attribute(element, name)?;
    if unsafe { CFGetTypeID(value.0) } != unsafe { CFURLGetTypeID() } {
        return cf_string(value.0);
    }
    // SAFETY: CFURLGetString follows the Get rule; `value` keeps it alive.
    let string = unsafe { CFURLGetString(value.0) };
    (!string.is_null()).then(|| cf_string(string)).flatten()
}

fn cf_string_literal(value: &CStr) -> Option<CfStringRef> {
    let string =
        unsafe { CFStringCreateWithCString(ptr::null(), value.as_ptr(), CF_STRING_ENCODING_UTF8) };
    (!string.is_null()).then_some(string)
}

fn cf_string(value: CfTypeRef) -> Option<String> {
    if unsafe { CFGetTypeID(value) } != unsafe { CFStringGetTypeID() } {
        return None;
    }
    let value = value.cast();
    let length = unsafe { CFStringGetLength(value) };
    let capacity = unsafe { CFStringGetMaximumSizeForEncoding(length, CF_STRING_ENCODING_UTF8) }
        .saturating_add(1);
    let mut bytes = vec![0_u8; capacity as usize];
    if !unsafe {
        CFStringGetCString(
            value,
            bytes.as_mut_ptr().cast(),
            capacity,
            CF_STRING_ENCODING_UTF8,
        )
    } {
        return None;
    }
    let length = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    bytes.truncate(length);
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_attribute_is_none() {
        assert_eq!(focused_window_title(i32::MAX), None);
        assert_eq!(focused_web_url(i32::MAX), None);
        assert_eq!(char_before_caret(i32::MAX), None);
        assert!(selection(i32::MAX).is_none());
    }

    /// Hex's own tree belongs to the main thread that updates it: the
    /// foreground-context poller and Smart spacing must never ask for it
    /// (#1402).
    #[test]
    fn hex_never_reads_its_own_accessibility_tree() {
        let own_pid = std::process::id() as i32;
        assert!(foreign_application(own_pid).is_none());
        assert_eq!(focused_window_title(own_pid), None);
        assert_eq!(focused_web_url(own_pid), None);
        assert_eq!(char_before_caret(own_pid), None);
        assert!(browser_pages(own_pid).urls.is_empty());
    }
}
