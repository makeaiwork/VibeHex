pub const TELEMOST_APP: &str = "ru.yandex.desktop.telemost";
pub const TELEMOST_SITE: &str = "telemost.yandex.ru";

/// The call services Settings always lists, by provider key (the app's
/// canonical bundle ID, or the website host for browser services) and
/// English name.
pub const KNOWN_MEETING_PROVIDERS: [(&str, &str); 9] = [
    ("com.tinyspeck.slackmacgap", "Slack huddles"),
    ("us.zoom.xos", "Zoom"),
    ("meet.google.com", "Google Meet"),
    ("com.microsoft.teams2", "Microsoft Teams"),
    ("com.apple.FaceTime", "FaceTime"),
    ("net.whatsapp.WhatsApp", "WhatsApp"),
    ("com.gather.GatherV2", "Gather"),
    (TELEMOST_APP, "Yandex Telemost"),
    (TELEMOST_SITE, "Yandex Telemost (web)"),
];

