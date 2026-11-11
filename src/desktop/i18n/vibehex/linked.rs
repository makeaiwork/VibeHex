//! Strings of questions in Requests that reach past their recording: the
//! six columns of [`super::super::TRANSLATIONS`] (Polish, Chinese
//! (Simplified), Japanese, German, Spanish, Ukrainian), then French,
//! Brazilian Portuguese, and Russian.

#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("Back to Requests", ["Wróć do zapytań", "返回请求", "リクエストに戻る", "Zurück zu Anfragen", "Volver a Solicitudes", "Назад до запитів"]),
    ("From your other recordings", ["Z Twoich innych nagrań", "来自你的其他录音", "あなたのほかの録音から", "Aus deinen anderen Aufnahmen", "De tus otras grabaciones", "З ваших інших записів"]),
    ("This question reaches past this recording. To answer, I need to read your other recordings.", ["To pytanie wykracza poza to nagranie. Aby odpowiedzieć, muszę przeczytać Twoje inne nagrania.", "这个问题超出了这段录音。要回答它，我需要读取你的其他录音。", "この質問はこの録音の外にも及びます。答えるには、ほかの録音を読む必要があります。", "Diese Frage geht über diese Aufnahme hinaus. Um sie zu beantworten, muss ich deine anderen Aufnahmen lesen.", "Esta pregunta va más allá de esta grabación. Para responderla, necesito leer tus otras grabaciones.", "Це питання виходить за межі цього запису. Щоб відповісти, мені треба прочитати ваші інші записи."]),
    ("1 other meeting", ["1 inne spotkanie", "1 个其他会议", "ほかの会議 1 件", "1 anderes Meeting", "1 reunión más", "1 інша зустріч"]),
    ("{} other meetings", ["inne spotkania ({})", "{} 个其他会议", "ほかの会議 {} 件", "{} andere Meetings", "{} reuniones más", "інші зустрічі ({})"]),
    ("1 other file", ["1 inny plik", "1 个其他文件", "ほかのファイル 1 件", "1 andere Datei", "1 archivo más", "1 інший файл"]),
    ("{} other files", ["inne pliki ({})", "{} 个其他文件", "ほかのファイル {} 件", "{} andere Dateien", "{} archivos más", "інші файли ({})"]),
];

#[rustfmt::skip]
pub(in super::super) const FRENCH: &[(&str, &str)] = &[
    ("Back to Requests", "Retour aux demandes"),
    ("From your other recordings", "De vos autres enregistrements"),
    ("This question reaches past this recording. To answer, I need to read your other recordings.", "Cette question dépasse cet enregistrement. Pour y répondre, je dois lire vos autres enregistrements."),
    ("1 other meeting", "1 autre réunion"),
    ("{} other meetings", "{} autres réunions"),
    ("1 other file", "1 autre fichier"),
    ("{} other files", "{} autres fichiers"),
];

#[rustfmt::skip]
pub(in super::super) const PORTUGUESE: &[(&str, &str)] = &[
    ("Back to Requests", "Voltar às solicitações"),
    ("From your other recordings", "Das suas outras gravações"),
    ("This question reaches past this recording. To answer, I need to read your other recordings.", "Esta pergunta vai além desta gravação. Para responder, preciso ler suas outras gravações."),
    ("1 other meeting", "1 outra reunião"),
    ("{} other meetings", "{} outras reuniões"),
    ("1 other file", "1 outro arquivo"),
    ("{} other files", "{} outros arquivos"),
];

#[rustfmt::skip]
pub(in super::super) const RUSSIAN: &[(&str, &str)] = &[
    ("Back to Requests", "Назад к запросам"),
    ("From your other recordings", "Из других ваших записей"),
    ("This question reaches past this recording. To answer, I need to read your other recordings.", "Этот вопрос выходит за пределы этой записи. Чтобы ответить, мне нужно прочитать другие ваши записи."),
    ("1 other meeting", "1 другая встреча"),
    ("{} other meetings", "другие встречи ({})"),
    ("1 other file", "1 другой файл"),
    ("{} other files", "другие файлы ({})"),
];
