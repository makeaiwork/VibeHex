//! What this computer has that some features need: a screen with a notch
//! (Labs › Notch indicator), a mouse with buttons past the two main ones
//! (Labs › Mouse shortcuts). Read when a question or setup round starts,
//! never per frame; a scene sets it as a fixture.
//!
//! On macOS the screens come from `NSScreen` (a notch is a non-zero top
//! safe-area inset, as the HUD measures it; built in by
//! `CGDisplayIsBuiltin`), the pointing devices from IOKit's HID registry
//! without opening any (product name, vendor, and the buttons its report
//! descriptor declares), and the model from `hw.model`. Elsewhere, and
//! whatever can't be read, stays unknown: a feature is then never left out
//! for it.

/// What a Labs feature needs (`needs:` in its guide article).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Need {
    /// A screen with a notch.
    Notch,
    /// A mouse with a button besides the left and right ones.
    MouseButtons,
}

impl Need {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim() {
            "notch" => Some(Self::Notch),
            "mouse_buttons" => Some(Self::MouseButtons),
            _ => None,
        }
    }

    /// What it needs, in English, for the model.
    pub fn words(self) -> &'static str {
        match self {
            Self::Notch => "a screen with a notch",
            Self::MouseButtons => "a mouse with extra buttons",
        }
    }
}

/// The screens connected.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Screens {
    /// A built-in screen (a MacBook's, an iMac's) is among them.
    pub built_in: bool,
    /// How many others there are.
    pub external: usize,
    /// Any of them has a notch.
    pub notch: bool,
}

/// A mouse or trackpad.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Pointer {
    pub name: String,
    /// It has, or may have, buttons besides the left and right ones.
    pub extra_buttons: bool,
}

/// What was read; `None` where it couldn't be.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hardware {
    pub screens: Option<Screens>,
    pub pointers: Option<Vec<Pointer>>,
    /// `hw.model`, like `Mac16,10` or `MacBookPro18,3`.
    pub model: Option<String>,
}

impl Hardware {
    /// Whether `need` is met here; `None` while unknown.
    pub fn has(&self, need: Need) -> Option<bool> {
        match need {
            Need::Notch => self.screens.as_ref().map(|screens| screens.notch),
            Need::MouseButtons => self
                .pointers
                .as_ref()
                .map(|pointers| pointers.iter().any(|pointer| pointer.extra_buttons)),
        }
    }

    /// Whether every one of `needs` may be met: false only when one is
    /// known not to be.
    pub fn allows(&self, needs: &[Need]) -> bool {
        needs.iter().all(|need| self.has(*need) != Some(false))
    }

    /// The state line's words: "screens: 1 external, no notch; pointing:
    /// Magic Mouse, no extra buttons"; empty when nothing is known.
    pub fn line(&self) -> String {
        let mut parts = Vec::new();
        if let Some(model) = &self.model {
            parts.push(format!("model: {model}"));
        }
        if let Some(screens) = &self.screens {
            let mut kinds = Vec::new();
            if screens.built_in {
                kinds.push("built-in".to_owned());
            }
            if screens.external > 0 {
                kinds.push(format!("{} external", screens.external));
            }
            if kinds.is_empty() {
                kinds.push("none".to_owned());
            }
            let notch = if screens.notch {
                "with a notch"
            } else {
                "no notch"
            };
            parts.push(format!("screens: {}, {notch}", kinds.join(" and ")));
        }
        if let Some(pointers) = &self.pointers {
            let named: Vec<String> = pointers
                .iter()
                .map(|pointer| {
                    let buttons = if pointer.extra_buttons {
                        "may have extra buttons"
                    } else {
                        "no extra buttons"
                    };
                    format!("{}, {buttons}", pointer.name)
                })
                .collect();
            parts.push(match named.is_empty() {
                true => "pointing: none found".to_owned(),
                false => format!("pointing: {}", named.join("; ")),
            });
        }
        parts.join("; ")
    }
}

/// Whether a pointing device may have buttons past the two main ones:
/// Apple's (Magic Mouse, Magic Trackpad, a built-in trackpad) never do;
/// another reporting more than two buttons, or not saying, may.
// Only the macOS reader calls it; its tests run everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn extra_buttons(name: &str, apple: bool, buttons: Option<usize>) -> bool {
    let lower = name.to_lowercase();
    if apple || lower.contains("magic") || lower.contains("trackpad") {
        return false;
    }
    buttons.is_none_or(|buttons| buttons > 2)
}

/// A device's name without whose it is ("Anna's Magic Mouse" is Magic
/// Mouse): the name may go into a prompt, the owner's never.
// Only the macOS reader calls it; its tests run everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn plain_name(name: &str) -> String {
    let name = name.trim();
    ["'s ", "’s "]
        .iter()
        .filter_map(|mark| name.find(mark).map(|at| &name[at + mark.len()..]))
        .next()
        .unwrap_or(name)
        .trim()
        .to_owned()
}

/// This computer's hardware, read now. Screens only on the main thread.
pub fn detect() -> Hardware {
    #[cfg(target_os = "macos")]
    {
        macos::detect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Hardware::default()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{CStr, c_void};
    use std::ptr;

    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2_core_foundation::{
        CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType,
    };

    use super::{Hardware, Pointer, Screens, extra_buttons};

    const GENERIC_DESKTOP: i32 = 0x01;
    const POINTER: i32 = 0x01;
    const MOUSE: i32 = 0x02;
    const BUTTON_PAGE: u32 = 0x09;
    /// Apple's USB vendor id, and its Bluetooth one.
    const APPLE: [i64; 2] = [0x05ac, 0x004c];

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> *mut c_void;
        fn IOHIDManagerSetDeviceMatchingMultiple(manager: *mut c_void, multiple: *const c_void);
        fn IOHIDManagerCopyDevices(manager: *mut c_void) -> *mut c_void;
        fn IOHIDDeviceGetProperty(device: *const c_void, key: *const c_void) -> *const c_void;
        fn IOHIDDeviceCopyMatchingElements(
            device: *mut c_void,
            matching: *const c_void,
            options: u32,
        ) -> *mut c_void;
        fn IOHIDElementGetUsagePage(element: *mut c_void) -> u32;
        fn IOHIDElementGetUsage(element: *mut c_void) -> u32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFSetGetCount(set: *const c_void) -> isize;
        fn CFSetGetValues(set: *const c_void, values: *mut *const c_void);
        fn CFRelease(value: *const c_void);
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayIsBuiltin(display: u32) -> u32;
    }

    pub(super) fn detect() -> Hardware {
        Hardware {
            screens: screens(),
            pointers: pointers(),
            model: model(),
        }
    }

    fn screens() -> Option<Screens> {
        let mtm = MainThreadMarker::new()?;
        let mut found = Screens::default();
        for screen in NSScreen::screens(mtm).iter() {
            let number = crate::rune_hud::macos::screen_number(&screen);
            // SAFETY: a plain query on a display id.
            let built_in = number.is_some_and(|id| unsafe { CGDisplayIsBuiltin(id) } != 0);
            if built_in {
                found.built_in = true;
            } else {
                found.external += 1;
            }
            found.notch |= screen.safeAreaInsets().top > 0.0;
        }
        Some(found)
    }

    fn model() -> Option<String> {
        let mut buffer = [0u8; 64];
        let mut size = buffer.len();
        // SAFETY: the name is NUL-terminated and the buffer holds `size`.
        let status = unsafe {
            libc::sysctlbyname(
                c"hw.model".as_ptr(),
                buffer.as_mut_ptr().cast(),
                &mut size,
                ptr::null_mut(),
                0,
            )
        };
        (status == 0).then_some(())?;
        CStr::from_bytes_until_nul(&buffer)
            .ok()
            .map(|model| model.to_string_lossy().into_owned())
            .filter(|model| !model.is_empty())
    }

    fn matching(usage: i32) -> CFRetained<CFDictionary<CFString, CFNumber>> {
        // `kIOHIDDeviceUsagePageKey` and `kIOHIDDeviceUsageKey`, by content.
        let page_key = CFString::from_static_str("DeviceUsagePage");
        let usage_key = CFString::from_static_str("DeviceUsage");
        CFDictionary::from_slices(
            &[&page_key, &usage_key],
            &[
                &CFNumber::new_i32(GENERIC_DESKTOP),
                &CFNumber::new_i32(usage),
            ],
        )
    }

    /// The mice and trackpads connected, read from the registry without
    /// opening one (no Input Monitoring is asked for).
    fn pointers() -> Option<Vec<Pointer>> {
        // SAFETY: IOKit returns an owned manager, released below.
        let manager = unsafe { IOHIDManagerCreate(ptr::null(), 0) };
        if manager.is_null() {
            return None;
        }
        let mice = matching(MOUSE);
        let pointers = matching(POINTER);
        let multiple = CFArray::from_retained_objects(&[mice, pointers]);
        // SAFETY: `manager` is live; the array is a valid CFArray of
        // dictionaries, which the manager retains.
        let devices = unsafe {
            IOHIDManagerSetDeviceMatchingMultiple(
                manager,
                (&*multiple as *const CFArray<_>).cast(),
            );
            IOHIDManagerCopyDevices(manager)
        };
        let mut found: Vec<Pointer> = Vec::new();
        if !devices.is_null() {
            // SAFETY: `devices` is a live CFSet of IOHIDDevices; `values`
            // holds a slot for each.
            let count = unsafe { CFSetGetCount(devices.cast_const()) }.max(0) as usize;
            let mut values: Vec<*const c_void> = vec![ptr::null(); count];
            unsafe { CFSetGetValues(devices.cast_const(), values.as_mut_ptr()) };
            for device in values.into_iter().filter(|device| !device.is_null()) {
                let pointer = describe(device);
                if !found.iter().any(|known| known == &pointer) {
                    found.push(pointer);
                }
            }
            // SAFETY: the copy is ours, released exactly once.
            unsafe { CFRelease(devices.cast_const()) };
        }
        // SAFETY: created above, released exactly once.
        unsafe { CFRelease(manager.cast_const()) };
        Some(found)
    }

    fn describe(device: *const c_void) -> Pointer {
        let property = |key: &'static str| {
            let key = CFString::from_static_str(key);
            // SAFETY: a get-rule property of a live device.
            let value =
                unsafe { IOHIDDeviceGetProperty(device, (&*key as *const CFString).cast()) };
            (!value.is_null()).then(|| unsafe { &*value.cast::<CFType>() })
        };
        let name = property("Product")
            .and_then(|value| value.downcast_ref::<CFString>().map(ToString::to_string))
            .unwrap_or_default();
        let vendor = property("VendorID")
            .and_then(|value| value.downcast_ref::<CFNumber>().and_then(|n| n.as_i64()))
            .unwrap_or_default();
        let built_in = property("Built-In")
            .and_then(|value| value.downcast_ref::<CFBoolean>().map(|b| b.as_bool()))
            .unwrap_or(false);
        let name = match (name.trim(), built_in) {
            ("", true) => "Built-in trackpad".to_owned(),
            ("", false) => "Mouse".to_owned(),
            (name, _) => super::plain_name(name),
        };
        let apple = built_in || APPLE.contains(&vendor);
        let extra_buttons = extra_buttons(&name, apple, buttons(device));
        Pointer {
            name,
            extra_buttons,
        }
    }

    /// How many buttons the device's report descriptor declares, if it
    /// says.
    fn buttons(device: *const c_void) -> Option<usize> {
        // SAFETY: a +1 CFArray of the device's elements, or null.
        let elements =
            unsafe { IOHIDDeviceCopyMatchingElements(device.cast_mut(), ptr::null(), 0) };
        if elements.is_null() {
            return None;
        }
        // SAFETY: owned, released by the wrapper.
        let elements: CFRetained<CFArray<CFType>> =
            unsafe { CFRetained::from_raw(ptr::NonNull::new_unchecked(elements.cast())) };
        let mut usages: Vec<u32> = Vec::new();
        for element in elements.iter() {
            let element = (&*element as *const CFType).cast::<c_void>().cast_mut();
            // SAFETY: each value is a live IOHIDElement.
            let (page, usage) = unsafe {
                (
                    IOHIDElementGetUsagePage(element),
                    IOHIDElementGetUsage(element),
                )
            };
            if page == BUTTON_PAGE && usage > 0 && !usages.contains(&usage) {
                usages.push(usage);
            }
        }
        (!usages.is_empty()).then_some(usages.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What this computer reads as, off the main thread (so no screens):
    /// `cargo test --bin hex hardware::tests::this_computer -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads this computer's devices"]
    fn this_computer() {
        println!("{:#?}\n{}", detect(), detect().line());
    }

    #[test]
    fn apples_pointers_have_no_extra_buttons_and_others_may() {
        assert!(!extra_buttons("Magic Mouse", true, Some(3)));
        assert!(!extra_buttons("Magic Trackpad", false, None));
        assert!(!extra_buttons("Built-in trackpad", true, None));
        assert!(extra_buttons("MX Master 3S", false, Some(8)));
        assert!(extra_buttons("USB Optical Mouse", false, None));
        assert!(!extra_buttons("USB Optical Mouse", false, Some(2)));
        assert_eq!(plain_name("Anna's Magic Mouse"), "Magic Mouse");
        assert_eq!(plain_name("Anna’s MX Master 3S"), "MX Master 3S");
        assert_eq!(plain_name("MX Master 3S"), "MX Master 3S");
    }

    #[test]
    fn unknown_hardware_allows_everything_and_the_line_says_what_is_known() {
        let unknown = Hardware::default();
        assert!(unknown.allows(&[Need::Notch, Need::MouseButtons]));
        assert_eq!(unknown.line(), "");
        let mini = Hardware {
            screens: Some(Screens {
                built_in: false,
                external: 1,
                notch: false,
            }),
            pointers: Some(vec![Pointer {
                name: "Magic Mouse".into(),
                extra_buttons: false,
            }]),
            model: None,
        };
        assert!(!mini.allows(&[Need::Notch]));
        assert!(!mini.allows(&[Need::MouseButtons]));
        assert_eq!(
            mini.line(),
            "screens: 1 external, no notch; pointing: Magic Mouse, no extra buttons"
        );
        assert_eq!(Need::parse("notch"), Some(Need::Notch));
        assert_eq!(Need::parse("touch_id"), None);
    }
}
