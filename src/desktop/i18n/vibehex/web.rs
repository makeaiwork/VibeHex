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
    ("Web", ["Sieć", "网页", "ウェブ", "Web", "Web", "Веб"]),
    ("Searches the web when the guide has no answer", ["Szuka w internecie, gdy przewodnik nie ma odpowiedzi", "指南没有答案时搜索网络", "ガイドに答えがないときはウェブを検索します", "Sucht im Web, wenn der Leitfaden keine Antwort hat", "Busca en la web cuando la guía no tiene la respuesta", "Шукає в інтернеті, коли посібник не має відповіді"]),
    ("Answers from the guide only", ["Odpowiada tylko z przewodnika", "只根据指南回答", "ガイドだけから答えます", "Antwortet nur aus dem Leitfaden", "Responde solo con la guía", "Відповідає лише з посібника"]),
    ("Searched the web for “{}”", ["Wyszukano w internecie „{}”", "已在网络上搜索“{}”", "ウェブで「{}」を検索しました", "Im Web nach „{}“ gesucht", "Buscado en la web: «{}»", "Пошук в інтернеті: «{}»"]),
    ("Read {}", ["Przeczytano {}", "已阅读 {}", "{} を読みました", "{} gelesen", "Leído {}", "Прочитано {}"]),
    ("Searching the web", ["Szukam w internecie", "正在搜索网络", "ウェブを検索しています", "Suche im Web", "Buscando en la web", "Шукаю в інтернеті"]),
    ("Read the guide and searched the web in {} s", ["Przeczytano przewodnik i przeszukano internet w {} s", "已在 {} 秒内阅读指南并搜索网络", "{} 秒でガイドを読み、ウェブを検索しました", "Leitfaden gelesen und im Web gesucht in {} s", "Leída la guía y buscado en la web en {} s", "Прочитано посібник і пошукано в інтернеті за {} с"]),
    ("Read the guide and 1 web page in {} s", ["Przeczytano przewodnik i 1 stronę internetową w {} s", "已在 {} 秒内阅读指南和 1 个网页", "{} 秒でガイドとウェブページ 1 件を読みました", "Leitfaden und 1 Webseite gelesen in {} s", "Leída la guía y 1 página web en {} s", "Прочитано посібник і 1 вебсторінку за {} с"]),
    ("Read the guide and {pages} web pages in {seconds} s", ["Przeczytano przewodnik i strony internetowe ({pages}) w {seconds} s", "已在 {seconds} 秒内阅读指南和 {pages} 个网页", "{seconds} 秒でガイドとウェブページ {pages} 件を読みました", "Leitfaden und {pages} Webseiten gelesen in {seconds} s", "Leída la guía y {pages} páginas web en {seconds} s", "Прочитано посібник і вебсторінок: {pages} за {seconds} с"]),
    ("Searched the guide:", ["Przeszukano przewodnik:", "已搜索指南：", "ガイドを検索：", "Leitfaden durchsucht:", "Buscado en la guía:", "Переглянуто посібник:"]),
    ("The guide doesn't cover all of it", ["Przewodnik nie obejmuje wszystkiego", "指南没有涵盖全部内容", "ガイドにはすべては載っていません", "Der Leitfaden deckt nicht alles ab", "La guía no lo cubre todo", "Посібник охоплює не все"]),
    ("Searched the web:", ["Przeszukano internet:", "已搜索网络：", "ウェブを検索：", "Im Web gesucht:", "Buscado en la web:", "Пошук в інтернеті:"]),
    ("1 page read", ["przeczytano 1 stronę", "已阅读 1 个网页", "1 ページを読みました", "1 Seite gelesen", "1 página leída", "прочитано 1 сторінку"]),
    ("{} pages read", ["przeczytane strony: {}", "已阅读 {} 个网页", "{} ページを読みました", "{} Seiten gelesen", "{} páginas leídas", "прочитано сторінок: {}"]),
    ("1 search", ["1 wyszukiwanie", "1 次搜索", "1 回検索", "1 Suche", "1 búsqueda", "1 пошук"]),
    ("{} searches", ["wyszukiwania: {}", "{} 次搜索", "{} 回検索", "{} Suchen", "{} búsquedas", "пошуків: {}"]),
    ("From the web", ["Z internetu", "来自网络", "ウェブより", "Aus dem Web", "De la web", "З інтернету"]),
];

/// French.
#[rustfmt::skip]
pub(in super::super) const FRENCH: &[(&str, &str)] = &[
    ("The web", "Le web"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Avec Codex seulement, quand le guide n'a pas la réponse. Les recherches passent par OpenAI, le fournisseur de Codex, et ne contiennent jamais vos enregistrements ni vos réglages"),
    ("Web", "Web"),
    ("Searches the web when the guide has no answer", "Cherche sur le web quand le guide n'a pas la réponse"),
    ("Answers from the guide only", "Répond avec le guide seulement"),
    ("Searched the web for “{}”", "Recherché sur le web : « {} »"),
    ("Read {}", "Lu : {}"),
    ("Searching the web", "Recherche sur le web"),
    ("Read the guide and searched the web in {} s", "Guide lu et web consulté en {} s"),
    ("Read the guide and 1 web page in {} s", "Guide et 1 page web lus en {} s"),
    ("Read the guide and {pages} web pages in {seconds} s", "Guide et {pages} pages web lus en {seconds} s"),
    ("Searched the guide:", "Guide parcouru :"),
    ("The guide doesn't cover all of it", "Le guide ne couvre pas tout"),
    ("Searched the web:", "Recherche sur le web :"),
    ("1 page read", "1 page lue"),
    ("{} pages read", "{} pages lues"),
    ("1 search", "1 recherche"),
    ("{} searches", "{} recherches"),
    ("From the web", "Du web"),
];

/// Brazilian Portuguese.
#[rustfmt::skip]
pub(in super::super) const PORTUGUESE: &[(&str, &str)] = &[
    ("The web", "A web"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Só no Codex, quando o guia não tem a resposta. As buscas passam pela OpenAI, a provedora do Codex, e nunca levam suas gravações nem seus ajustes"),
    ("Web", "Web"),
    ("Searches the web when the guide has no answer", "Busca na web quando o guia não tem a resposta"),
    ("Answers from the guide only", "Responde só com o guia"),
    ("Searched the web for “{}”", "Buscou na web: “{}”"),
    ("Read {}", "Leu {}"),
    ("Searching the web", "Buscando na web"),
    ("Read the guide and searched the web in {} s", "Leu o guia e buscou na web em {} s"),
    ("Read the guide and 1 web page in {} s", "Leu o guia e 1 página da web em {} s"),
    ("Read the guide and {pages} web pages in {seconds} s", "Leu o guia e {pages} páginas da web em {seconds} s"),
    ("Searched the guide:", "Buscou no guia:"),
    ("The guide doesn't cover all of it", "O guia não cobre tudo"),
    ("Searched the web:", "Buscou na web:"),
    ("1 page read", "1 página lida"),
    ("{} pages read", "{} páginas lidas"),
    ("1 search", "1 busca"),
    ("{} searches", "{} buscas"),
    ("From the web", "Da web"),
];

/// Russian.
#[rustfmt::skip]
pub(in super::super) const RUSSIAN: &[(&str, &str)] = &[
    ("The web", "Интернет"),
    ("On Codex only, when the guide has no answer. Searches go through OpenAI, Codex's provider, and never carry your recordings or settings", "Только в Codex и только когда в руководстве нет ответа. Поиск идёт через OpenAI, поставщика Codex, и никогда не включает ваши записи и настройки"),
    ("Web", "Интернет"),
    ("Searches the web when the guide has no answer", "Ищет в интернете, когда в руководстве нет ответа"),
    ("Answers from the guide only", "Отвечает только по руководству"),
    ("Searched the web for “{}”", "Поиск в интернете: «{}»"),
    ("Read {}", "Прочитано: {}"),
    ("Searching the web", "Ищу в интернете"),
    ("Read the guide and searched the web in {} s", "Прочитал руководство и поискал в интернете за {} с"),
    ("Read the guide and 1 web page in {} s", "Прочитал руководство и 1 страницу в интернете за {} с"),
    ("Read the guide and {pages} web pages in {seconds} s", "Прочитал руководство и страниц в интернете: {pages} за {seconds} с"),
    ("Searched the guide:", "Поиск по руководству:"),
    ("The guide doesn't cover all of it", "В руководстве есть не всё"),
    ("Searched the web:", "Поиск в интернете:"),
    ("1 page read", "прочитана 1 страница"),
    ("{} pages read", "прочитано страниц: {}"),
    ("1 search", "1 запрос"),
    ("{} searches", "запросов: {}"),
    ("From the web", "Из интернета"),
];
