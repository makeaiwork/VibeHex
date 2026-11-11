//! Assistant's setting-change cards and the Settings row that lets it read
//! the current settings, keyed by the English source string: the six
//! columns of [`super::super::TRANSLATIONS`] (Polish, Chinese (Simplified),
//! Japanese, German, Spanish, Ukrainian), then French, Brazilian
//! Portuguese, and Russian. VibeHex's own, so a merge from Hex touches none
//! of them.

#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("Your current settings", ["Twoje bieżące ustawienia", "你当前的设置", "現在の設定", "Deine aktuellen Einstellungen", "Tus ajustes actuales", "Ваші поточні налаштування"]),
    ("So it knows what is already on. API keys are never sent", ["Aby wiedział, co już jest włączone. Klucze API nigdy nie są wysyłane", "让它知道哪些已经开启。API 密钥从不发送", "何がすでにオンかを把握するためです。API キーは送信されません", "Damit er weiß, was schon an ist. API-Schlüssel werden nie gesendet", "Para que sepa qué está activado. Las claves de API nunca se envían", "Щоб знав, що вже ввімкнено. Ключі API ніколи не надсилаються"]),
];

/// French.
#[rustfmt::skip]
pub(in super::super) const FRENCH: &[(&str, &str)] = &[
    ("Your current settings", "Vos réglages actuels"),
    ("So it knows what is already on. API keys are never sent", "Pour qu’il sache ce qui est déjà activé. Les clés API ne sont jamais envoyées"),
];

/// Brazilian Portuguese.
#[rustfmt::skip]
pub(in super::super) const PORTUGUESE: &[(&str, &str)] = &[
    ("Your current settings", "Suas configurações atuais"),
    ("So it knows what is already on. API keys are never sent", "Para saber o que já está ativado. Chaves de API nunca são enviadas"),
];

/// Russian.
#[rustfmt::skip]
pub(in super::super) const RUSSIAN: &[(&str, &str)] = &[
    ("Your current settings", "Ваши текущие настройки"),
    ("So it knows what is already on. API keys are never sent", "Чтобы знать, что уже включено. Ключи API не отправляются"),
];
