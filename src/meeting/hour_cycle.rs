//! Whether times read on a 24-hour clock, as the Mac's region shows them.

/// Whether the Mac shows times on a 24-hour clock (System Settings ›
/// General › Date & Time, or the region's own format), read once. Other OSes
/// and tests keep the 12-hour clock.
fn twenty_four_hour() -> bool {
    #[cfg(all(target_os = "macos", not(test)))]
    {
        static TWENTY_FOUR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *TWENTY_FOUR.get_or_init(|| {
            use objc2_foundation::{NSDateFormatter, NSLocale, NSString};
            let locale = NSLocale::currentLocale();
            let format = NSDateFormatter::dateFormatFromTemplate_options_locale(
                &NSString::from_str("j"),
                0,
                Some(&locale),
            );
            format.is_some_and(|format| !format.to_string().contains('a'))
        })
    }
    #[cfg(not(all(target_os = "macos", not(test))))]
    {
        false
    }
}

/// `14:05` on a 24-hour clock, else `2:05 PM`.
pub(super) fn clock(hour: u32, minute: u32) -> String {
    if twenty_four_hour() {
        return format!("{hour}:{minute:02}");
    }
    let period = if hour < 12 { "AM" } else { "PM" };
    let hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour}:{minute:02} {period}")
}
