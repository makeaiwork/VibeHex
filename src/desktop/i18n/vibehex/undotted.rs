//! Hex's strings that joined two parts with a middle dot, said without
//! it: VibeHex never puts "·" between words. Where the parts were one
//! thought the dot became a comma, a full stop or brackets; where they were
//! separate facts each is a string of its own now, and the window sets them
//! apart with air (`rune_ui::lists::apart`). Hex's dotted rows stay in its
//! tables, unused.

/// Columns: Polish, Chinese (Simplified), Japanese, German, Spanish, Ukrainian.
#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("Local build, updates off", ["Lokalna kompilacja, aktualizacje wyłączone", "本地构建，更新已关闭", "ローカルビルド、更新はオフ", "Lokaler Build, Updates aus", "Compilación local, actualizaciones desactivadas", "Локальна збірка, оновлення вимкнено"]),
    ("{rate} (lower is better)", ["{rate} (im niżej, tym lepiej)", "{rate}（越低越好）", "{rate}（低いほど良い）", "{rate} (niedriger ist besser)", "{rate} (cuanto menor, mejor)", "{rate} (що менше, то краще)"]),
    ("Full screen. Click to record", ["Cały ekran. Kliknij, aby nagrać", "全屏。点按以录制", "全画面。クリックで収録", "Ganzer Bildschirm. Klicken zum Aufzeichnen", "Pantalla completa. Haz clic para grabar", "Увесь екран. Клацніть, щоб записати"]),
    ("Full screen. Click to capture", ["Cały ekran. Kliknij, aby uchwycić", "全屏。点按以截图", "全画面。クリックで撮影", "Ganzer Bildschirm. Klicken zum Aufnehmen", "Pantalla completa. Haz clic para capturar", "Увесь екран. Клацніть, щоб зняти"]),
    ("Ready anywhere. Hold {} to dictate", ["Gotowy wszędzie. Przytrzymaj {}, aby dyktować", "随处可用。按住 {} 进行听写", "どこでも使えます。{} を押している間に音声入力", "Überall bereit. {} halten zum Diktieren", "Listo en cualquier lugar. Mantén {} para dictar", "Готово всюди. Утримуйте {}, щоб диктувати"]),
    ("Ready anywhere. Press {} to start and stop dictating", ["Gotowy wszędzie. Naciśnij {}, aby zacząć i zakończyć dyktowanie", "随处可用。按 {} 开始和停止听写", "どこでも使えます。{} を押して音声入力を開始・停止", "Überall bereit. {} drücken zum Starten und Beenden des Diktats", "Listo en cualquier lugar. Pulsa {} para empezar y terminar de dictar", "Готово всюди. Натисніть {}, щоб почати й завершити диктування"]),
    ("Ready anywhere. Use your desktop's shortcuts to dictate", ["Gotowe wszędzie. Dyktuj skrótami pulpitu", "随处可用。使用桌面快捷键听写", "どこでも利用可能。デスクトップのショートカットで音声入力", "Überall bereit. Diktiere mit den Tastenkürzeln des Desktops", "Listo en cualquier lugar. Dicta con los atajos del escritorio", "Готово всюди. Диктуйте за допомогою системних комбінацій клавіш"]),
    ("Loading model ({}s)", ["Wczytywanie modelu ({} s)", "正在加载模型（{} 秒）", "モデルを読み込み中（{} 秒）", "Modell wird geladen ({} s)", "Cargando modelo ({} s)", "Завантаження моделі в памʼять ({} с)"]),
    ("Ready: {}", ["Gotowe: {}", "已就绪：{}", "完了：{}", "Fertig: {}", "Lista: {}", "Готово: {}"]),
    ("Dictations: {count}", ["Dyktowania: {count}", "听写：{count}", "音声入力: {count}", "Diktate: {count}", "Dictados: {count}", "Диктувань: {count}"]),
    ("Words: {words}", ["Słowa: {words}", "字词：{words}", "単語: {words}", "Wörter: {words}", "Palabras: {words}", "Слів: {words}"]),
    ("Words per minute: {wpm}", ["Słowa na minutę: {wpm}", "每分钟字词：{wpm}", "毎分の単語数: {wpm}", "Wörter pro Minute: {wpm}", "Palabras por minuto: {wpm}", "Слів за хвилину: {wpm}"]),
    ("Changed by Output: {percent}%", ["Zmienione przez Wyjście: {percent}%", "被输出更改：{percent}%", "出力による変更: {percent}%", "Von der Ausgabe geändert: {percent} %", "Cambiados por Salida: {percent} %", "Змінено Виведенням: {percent}%"]),
    ("Daily streak: {days}", ["Seria dni: {days}", "连续天数：{days}", "連続日数: {days}", "Tage in Folge: {days}", "Días seguidos: {days}", "Днів поспіль: {days}"]),
    ("{audio} ms audio", ["{audio} ms dźwięku", "音频 {audio} 毫秒", "音声 {audio} ms", "{audio} ms Audio", "{audio} ms de audio", "{audio} мс аудіо"]),
    ("{inference} ms inference", ["{inference} ms wnioskowania", "推理 {inference} 毫秒", "推論 {inference} ms", "{inference} ms Inferenz", "{inference} ms de inferencia", "{inference} мс обробки"]),
    ("Language: {language}", ["Język: {language}", "语言：{language}", "言語: {language}", "Sprache: {language}", "Idioma: {language}", "Мова: {language}"]),
    ("Started automatically", ["Rozpoczęto automatycznie", "已自动开始", "自動で開始", "Automatisch gestartet", "Iniciada automáticamente", "Розпочато автоматично"]),
    ("VibeHex records automatically", ["VibeHex nagra automatycznie", "VibeHex 会自动录制", "VibeHex が自動で録音します", "VibeHex nimmt automatisch auf", "VibeHex grabará automáticamente", "VibeHex записує автоматично"]),
];

#[rustfmt::skip]
pub(super) const FRENCH: &[(&str, &str)] = &[
    ("Local build, updates off", "Build local, mises à jour désactivées"),
    ("{rate} (lower is better)", "{rate} (plus bas, c'est mieux)"),
    ("Full screen. Click to record", "Plein écran. Cliquez pour enregistrer"),
    ("Full screen. Click to capture", "Plein écran. Cliquez pour capturer"),
    ("Ready anywhere. Hold {} to dictate", "Prêt partout. Maintenez {} pour dicter"),
    ("Ready anywhere. Press {} to start and stop dictating", "Prêt partout. Appuyez sur {} pour commencer et arrêter la dictée"),
    ("Ready anywhere. Use your desktop's shortcuts to dictate", "Prêt partout. Utilisez les raccourcis de votre bureau pour dicter"),
    ("Loading model ({}s)", "Chargement du modèle ({} s)"),
    ("Ready: {}", "Prête : {}"),
    ("Dictations: {count}", "Dictées : {count}"),
    ("Words: {words}", "Mots : {words}"),
    ("Words per minute: {wpm}", "Mots par minute : {wpm}"),
    ("Changed by Output: {percent}%", "Modifiées par Sortie : {percent} %"),
    ("Daily streak: {days}", "Jours d'affilée : {days}"),
    ("{audio} ms audio", "{audio} ms d'audio"),
    ("{inference} ms inference", "{inference} ms d'inférence"),
    ("Language: {language}", "Langue : {language}"),
    ("Started automatically", "Démarré automatiquement"),
    ("VibeHex records automatically", "VibeHex enregistre automatiquement"),
];

#[rustfmt::skip]
pub(super) const PORTUGUESE: &[(&str, &str)] = &[
    ("Local build, updates off", "Build local, atualizações desativadas"),
    ("{rate} (lower is better)", "{rate} (quanto menor, melhor)"),
    ("Full screen. Click to record", "Tela inteira. Clique para gravar"),
    ("Full screen. Click to capture", "Tela inteira. Clique para capturar"),
    ("Ready anywhere. Hold {} to dictate", "Pronto em qualquer lugar. Segure {} para ditar"),
    ("Ready anywhere. Press {} to start and stop dictating", "Pronto em qualquer lugar. Pressione {} para começar e parar de ditar"),
    ("Ready anywhere. Use your desktop's shortcuts to dictate", "Pronto em qualquer lugar. Use os atalhos do ambiente de trabalho para ditar"),
    ("Loading model ({}s)", "Carregando modelo ({} s)"),
    ("Ready: {}", "Pronta: {}"),
    ("Dictations: {count}", "Ditados: {count}"),
    ("Words: {words}", "Palavras: {words}"),
    ("Words per minute: {wpm}", "Palavras por minuto: {wpm}"),
    ("Changed by Output: {percent}%", "Alterados pela Saída: {percent}%"),
    ("Daily streak: {days}", "Dias seguidos: {days}"),
    ("{audio} ms audio", "{audio} ms de áudio"),
    ("{inference} ms inference", "{inference} ms de inferência"),
    ("Language: {language}", "Idioma: {language}"),
    ("Started automatically", "Iniciada automaticamente"),
    ("VibeHex records automatically", "O VibeHex grava automaticamente"),
];

#[rustfmt::skip]
pub(super) const RUSSIAN: &[(&str, &str)] = &[
    ("Local build, updates off", "Локальная сборка, обновления выключены"),
    ("{rate} (lower is better)", "{rate} (чем меньше, тем лучше)"),
    ("Full screen. Click to record", "Весь экран. Нажмите, чтобы записать"),
    ("Full screen. Click to capture", "Весь экран. Нажмите, чтобы снять"),
    ("Ready anywhere. Hold {} to dictate", "Готово везде. Удерживайте {}, чтобы диктовать"),
    ("Ready anywhere. Press {} to start and stop dictating", "Готово везде. Нажмите {}, чтобы начать и закончить диктовку"),
    ("Ready anywhere. Use your desktop's shortcuts to dictate", "Готово везде. Диктуйте с помощью сочетаний клавиш окружения"),
    ("Loading model ({}s)", "Загрузка модели в память ({} с)"),
    ("Ready: {}", "Готово: {}"),
    ("Dictations: {count}", "Диктовок: {count}"),
    ("Words: {words}", "Слов: {words}"),
    ("Words per minute: {wpm}", "Слов в минуту: {wpm}"),
    ("Changed by Output: {percent}%", "Изменено Выводом: {percent}%"),
    ("Daily streak: {days}", "Дней подряд: {days}"),
    ("{audio} ms audio", "{audio} мс звука"),
    ("{inference} ms inference", "{inference} мс обработки"),
    ("Language: {language}", "Язык: {language}"),
    ("Started automatically", "Запущено автоматически"),
    ("VibeHex records automatically", "VibeHex записывает автоматически"),
];

#[cfg(test)]
mod tests {
    use super::super::naming::tests::{sources, translated_literals};

    /// A middle dot between two words.
    const JOIN: &str = " · ";

    /// Sources that may write one: it is another format's, or a developer's
    /// tool, not the interface's.
    const KEPT_SOURCES: &[&str] = &[
        // The `hex` command's output, and meeting Markdown and reports.
        "src/control/",
        "src/control.rs",
        "src/meeting/",
        "src/meeting.rs",
        // The Playground's story names and tuning labels.
        "src/rune_ui/playground.rs",
        "src/rune_ui/story_link.rs",
        "src/rune_ui/hud_stories.rs",
        "src/rune_ui/onboarding_kit/stories.rs",
        "src/rune_ui/capture_capsule/playground.rs",
        "src/rune_ui/capture_capsule/playground/",
        "src/rune_ui/capture_capsule/tuning.rs",
    ];

    /// A speaker's line in a transcript's Markdown (`**You** · 00:01`): the
    /// file's format, which Copy transcript writes too.
    const TRANSCRIPT_LINE: &str = "** · ";

    #[test]
    fn no_translated_string_joins_words_with_a_middle_dot() {
        let six: std::collections::HashMap<_, _> =
            super::super::super::all_translations().copied().collect();
        let others = [
            super::super::super::french::table(),
            super::super::super::portuguese::table(),
            super::super::super::russian::table(),
        ];
        let mut checked = 0;
        for (path, source) in sources() {
            for literal in translated_literals(&source) {
                let english = literal.as_str();
                let translated = six.get(english).into_iter().flatten().copied();
                let translated = translated.chain(
                    others
                        .iter()
                        .filter_map(|table| table.get(english).copied()),
                );
                for text in std::iter::once(english).chain(translated) {
                    assert!(
                        !text.contains(JOIN),
                        "{} shows {text:?}: set the parts apart instead (rune_ui::lists::apart), \
                         or join them with a comma or brackets",
                        path.display()
                    );
                }
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} translated strings found");
    }

    /// The same for a line built with `format!` or `join`.
    #[test]
    fn no_interface_source_writes_a_middle_dot_between_words() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for (path, source) in sources() {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if KEPT_SOURCES.iter().any(|kept| name.starts_with(kept)) {
                continue;
            }
            for line in source.lines() {
                assert!(
                    !line.replace(TRANSCRIPT_LINE, "").contains(JOIN),
                    "{name} joins words with a middle dot: {}",
                    line.trim()
                );
            }
        }
    }
}
