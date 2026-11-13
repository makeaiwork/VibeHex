//! Cleanup on this computer in Assistant's cards and setup's plan
//! (`rune_ui::assistant::built_in`): the built-in models to pick from,
//! what each costs here, and its download. Keyed by the English source
//! string: the six columns of [`super::super::TRANSLATIONS`] (Polish,
//! Chinese (Simplified), Japanese, German, Spanish, Ukrainian), then
//! French, Brazilian Portuguese, and Russian.

#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("None of these fits this Mac's memory well: AI cleanup with Codex suits it better", ["Żaden z nich nie mieści się dobrze w pamięci tego Maca: lepiej pasuje porządkowanie przez AI z Codex", "这台 Mac 的内存放不下其中任何一个：用 Codex 做 AI 整理更合适", "どれもこの Mac のメモリには重すぎます。Codex による AI による整えのほうが向いています", "Keines davon passt gut in den Speicher dieses Macs: KI-Bereinigung mit Codex passt besser", "Ninguno cabe bien en la memoria de este Mac: la limpieza con IA mediante Codex le va mejor", "Жодна з них не вміщається добре в пам'ять цього Mac: краще підійде впорядкування ШІ через Codex"]),
    ("None of these fits this computer's memory well: AI cleanup with Codex suits it better", ["Żaden z nich nie mieści się dobrze w pamięci tego komputera: lepiej pasuje porządkowanie przez AI z Codex", "这台电脑的内存放不下其中任何一个：用 Codex 做 AI 整理更合适", "どれもこのコンピュータのメモリには重すぎます。Codex による AI による整えのほうが向いています", "Keines davon passt gut in den Speicher dieses Computers: KI-Bereinigung mit Codex passt besser", "Ninguno cabe bien en la memoria de este ordenador: la limpieza con IA mediante Codex le va mejor", "Жодна з них не вміщається добре в пам'ять цього комп'ютера: краще підійде впорядкування ШІ через Codex"]),
    ("about {} of memory while it runs", ["około {} pamięci podczas działania", "运行时约占 {} 内存", "動作中のメモリ約 {}", "etwa {} Speicher im Betrieb", "unos {} de memoria mientras funciona", "близько {} пам'яті під час роботи"]),
    ("May slow this Mac: needs about {need} of {total} memory", ["Może spowolnić tego Maca: potrzebuje około {need} z {total} pamięci", "可能拖慢这台 Mac：需要约 {need}，总内存 {total}", "この Mac が遅くなるかもしれません。メモリ {total} のうち約 {need} を使います", "Kann diesen Mac bremsen: braucht etwa {need} von {total} Speicher", "Puede ralentizar este Mac: necesita unos {need} de {total} de memoria", "Може сповільнити цей Mac: потребує близько {need} із {total} пам'яті"]),
    ("May slow this computer: needs about {need} of {total} memory", ["Może spowolnić ten komputer: potrzebuje około {need} z {total} pamięci", "可能拖慢这台电脑：需要约 {need}，总内存 {total}", "このコンピュータが遅くなるかもしれません。メモリ {total} のうち約 {need} を使います", "Kann diesen Computer bremsen: braucht etwa {need} von {total} Speicher", "Puede ralentizar este ordenador: necesita unos {need} de {total} de memoria", "Може сповільнити цей комп'ютер: потребує близько {need} із {total} пам'яті"]),
    ("Not enough disk space: needs {need}, {free} free", ["Za mało miejsca na dysku: potrzeba {need}, wolne {free}", "磁盘空间不足：需要 {need}，可用 {free}", "ディスクの空きが足りません。必要 {need}、空き {free}", "Nicht genug Speicherplatz: braucht {need}, {free} frei", "No hay espacio suficiente en el disco: necesita {need}, hay {free} libres", "Замало місця на диску: потрібно {need}, вільно {free}"]),
    ("Downloading {model}, {done} of {total}. AI cleanup uses it once it is in", ["Pobieranie {model}, {done} z {total}. Porządkowanie przez AI użyje go po pobraniu", "正在下载 {model}，{done} / {total}。下载完成后 AI 整理就会使用它", "{model} をダウンロード中、{done} / {total}。届いたら AI による整えで使われます", "{model} wird geladen, {done} von {total}. Die KI-Bereinigung nutzt es, sobald es da ist", "Descargando {model}, {done} de {total}. La limpieza con IA lo usará en cuanto llegue", "Завантаження {model}, {done} із {total}. Впорядкування ШІ використає її, щойно вона з'явиться"]),
];

/// French.
#[rustfmt::skip]
pub(in super::super) const FRENCH: &[(&str, &str)] = &[
    ("None of these fits this Mac's memory well: AI cleanup with Codex suits it better", "Aucun ne tient bien dans la mémoire de ce Mac : le nettoyage par IA avec Codex lui convient mieux"),
    ("None of these fits this computer's memory well: AI cleanup with Codex suits it better", "Aucun ne tient bien dans la mémoire de cet ordinateur : le nettoyage par IA avec Codex lui convient mieux"),
    ("about {} of memory while it runs", "environ {} de mémoire pendant son fonctionnement"),
    ("May slow this Mac: needs about {need} of {total} memory", "Peut ralentir ce Mac : il lui faut environ {need} sur {total} de mémoire"),
    ("May slow this computer: needs about {need} of {total} memory", "Peut ralentir cet ordinateur : il lui faut environ {need} sur {total} de mémoire"),
    ("Not enough disk space: needs {need}, {free} free", "Pas assez d'espace disque : il faut {need}, {free} libres"),
    ("Downloading {model}, {done} of {total}. AI cleanup uses it once it is in", "Téléchargement de {model}, {done} sur {total}. Le nettoyage par IA l'utilisera dès qu'il sera là"),
];

/// Brazilian Portuguese.
#[rustfmt::skip]
pub(in super::super) const PORTUGUESE: &[(&str, &str)] = &[
    ("None of these fits this Mac's memory well: AI cleanup with Codex suits it better", "Nenhum cabe bem na memória deste Mac: a limpeza com IA pelo Codex combina melhor com ele"),
    ("None of these fits this computer's memory well: AI cleanup with Codex suits it better", "Nenhum cabe bem na memória deste computador: a limpeza com IA pelo Codex combina melhor com ele"),
    ("about {} of memory while it runs", "cerca de {} de memória enquanto roda"),
    ("May slow this Mac: needs about {need} of {total} memory", "Pode deixar este Mac lento: precisa de cerca de {need} de {total} de memória"),
    ("May slow this computer: needs about {need} of {total} memory", "Pode deixar este computador lento: precisa de cerca de {need} de {total} de memória"),
    ("Not enough disk space: needs {need}, {free} free", "Espaço em disco insuficiente: precisa de {need}, {free} livres"),
    ("Downloading {model}, {done} of {total}. AI cleanup uses it once it is in", "Baixando {model}, {done} de {total}. A limpeza com IA vai usá-lo assim que chegar"),
];

/// Russian.
#[rustfmt::skip]
pub(in super::super) const RUSSIAN: &[(&str, &str)] = &[
    ("None of these fits this Mac's memory well: AI cleanup with Codex suits it better", "Ни одна из них не помещается в память этого Mac как следует: ему лучше подойдёт ИИ-очистка через Codex"),
    ("None of these fits this computer's memory well: AI cleanup with Codex suits it better", "Ни одна из них не помещается в память этого компьютера как следует: ему лучше подойдёт ИИ-очистка через Codex"),
    ("about {} of memory while it runs", "около {} памяти во время работы"),
    ("May slow this Mac: needs about {need} of {total} memory", "Может замедлить этот Mac: нужно около {need} из {total} памяти"),
    ("May slow this computer: needs about {need} of {total} memory", "Может замедлить этот компьютер: нужно около {need} из {total} памяти"),
    ("Not enough disk space: needs {need}, {free} free", "Мало места на диске: нужно {need}, свободно {free}"),
    ("Downloading {model}, {done} of {total}. AI cleanup uses it once it is in", "Скачивается {model}, {done} из {total}. ИИ-очистка начнёт использовать её, как только она скачается"),
];
