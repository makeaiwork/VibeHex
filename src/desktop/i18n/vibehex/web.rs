//! Assistant's web search: its row in Assistant › Settings, the field's Web
//! toggle, the steps an answer took on the web, and its "From the web"
//! chips, keyed by the English source string: the six columns of
//! [`super::super::TRANSLATIONS`] (Polish, Chinese (Simplified), Japanese,
//! German, Spanish, Ukrainian), then French, Brazilian Portuguese, and
//! Russian.

#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("The web", ["Internet", "网络", "ウェブ", "Das Web", "La web", "Інтернет"]),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", ["Tylko w Codex, gdy przewodnik nie ma odpowiedzi. Wyszukiwania idą przez OpenAI, dostawcę Codex, i nigdy nie zawierają Twoich nagrań ani ustawień", "仅限 Codex，且仅在指南没有答案时。搜索经由 Codex 的提供方 OpenAI 进行，绝不包含你的录音或设置", "Codex のみ、ガイドに答えがないときだけ。検索は Codex の提供元 OpenAI を経由し、録音や設定は決して含まれません", "Nur mit Codex, wenn der Leitfaden keine Antwort hat. Suchen laufen über OpenAI, den Anbieter von Codex, und enthalten nie deine Aufnahmen oder Einstellungen", "Solo con Codex, cuando la guía no tiene la respuesta. Las búsquedas pasan por OpenAI, el proveedor de Codex, y nunca llevan tus grabaciones ni tus ajustes", "Лише в Codex, коли посібник не має відповіді. Пошук іде через OpenAI, постачальника Codex, і ніколи не містить ваших записів чи налаштувань"]),
];

/// French.
#[rustfmt::skip]
pub(in super::super) const FRENCH: &[(&str, &str)] = &[
    ("The web", "Le web"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Avec Codex seulement, quand le guide n'a pas la réponse. Les recherches passent par OpenAI, le fournisseur de Codex, et ne contiennent jamais vos enregistrements ni vos réglages"),
];

/// Brazilian Portuguese.
#[rustfmt::skip]
pub(in super::super) const PORTUGUESE: &[(&str, &str)] = &[
    ("The web", "A web"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Só no Codex, quando o guia não tem a resposta. As buscas passam pela OpenAI, a provedora do Codex, e nunca levam suas gravações nem seus ajustes"),
];

/// Russian.
#[rustfmt::skip]
pub(in super::super) const RUSSIAN: &[(&str, &str)] = &[
    ("The web", "Интернет"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Только в Codex и только когда в справке нет ответа. Поиск идёт через OpenAI, поставщика Codex, и никогда не включает ваши записи и настройки"),
];
