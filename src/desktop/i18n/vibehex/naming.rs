//! Hex's interface strings that name the app, said again with VibeHex's
//! name: the English source is the key, as everywhere, so each string that
//! says "VibeHex" has its own rows here, and Hex's rows stay as they were
//! (unused) for the next merge. The name itself is
//! [`crate::app_paths::PRODUCT_NAME`]. "Hex Microphone" keeps its name: it is
//! the audio device's.

/// Columns: Polish, Chinese (Simplified), Japanese, German, Spanish, Ukrainian.
#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("VibeHex picks a fast model", ["VibeHex wybiera szybki model", "由 VibeHex 选择快速模型", "VibeHex が高速なモデルを選ぶ", "VibeHex wählt ein schnelles Modell", "VibeHex elige un modelo rápido", "VibeHex вибирає швидку модель"]),
    ("Summary models show once VibeHex connects to OpenCode.", ["Modele podsumowań pojawią się, gdy VibeHex połączy się z OpenCode.", "VibeHex 连接到 OpenCode 后会显示摘要模型。", "VibeHex が OpenCode に接続すると要約モデルが表示されます。", "Modelle für Zusammenfassungen erscheinen, sobald VibeHex mit OpenCode verbunden ist.", "Los modelos de resumen aparecen cuando VibeHex se conecta a OpenCode.", "Моделі для підсумків з'являться, щойно VibeHex під'єднається до OpenCode."]),
    ("Start OpenCode to choose a summary model. VibeHex keeps the saved one.", ["Uruchom OpenCode, aby wybrać model podsumowań. VibeHex zachowuje zapisany.", "启动 OpenCode 以选择摘要模型。VibeHex 会保留已保存的模型。", "要約モデルを選ぶには OpenCode を起動してください。保存済みのモデルはそのまま残ります。", "Starte OpenCode, um ein Modell für Zusammenfassungen zu wählen. VibeHex behält das gespeicherte.", "Inicia OpenCode para elegir un modelo de resumen. VibeHex conserva el guardado.", "Запустіть OpenCode, щоб вибрати модель для підсумків. VibeHex зберігає збережену."]),
    ("Hide the VibeHex icon from the menu bar. Shortcuts keep working; open VibeHex again from Finder or Spotlight for Settings", ["Ukrywa ikonę VibeHex na pasku menu. Skróty nadal działają; otwórz VibeHex ponownie z Findera lub Spotlight, aby przejść do Ustawień", "从菜单栏隐藏 VibeHex 的图标。快捷键仍然有效；从访达或聚焦搜索再次打开 VibeHex 即可进入设置", "メニューバーから VibeHex のアイコンを隠します。ショートカットは引き続き使えます。設定を開くには Finder や Spotlight から VibeHex をもう一度開きます", "Blendet das VibeHex-Symbol in der Menüleiste aus. Kürzel funktionieren weiter; öffne VibeHex erneut über den Finder oder Spotlight, um zu den Einstellungen zu gelangen", "Oculta el icono de VibeHex en la barra de menús. Los atajos siguen funcionando; vuelve a abrir VibeHex desde el Finder o Spotlight para ir a Ajustes", "Приховує значок VibeHex у рядку меню. Комбінації клавіш і далі працюють; щоб відкрити Налаштування, знову відкрийте VibeHex із Finder або Spotlight"]),
    ("⌘Q closes the Settings window instead of quitting VibeHex. Dictation keeps running; quit from the menu bar icon", ["⌘Q zamyka okno Ustawień zamiast kończyć VibeHex. Dyktowanie działa dalej; aby zakończyć, użyj ikony na pasku menu", "按 ⌘Q 关闭设置窗口而不退出 VibeHex。听写会继续运行；要退出请使用菜单栏图标", "⌘Q は VibeHex を終了せず、設定ウインドウを閉じます。音声入力はそのまま続きます。終了するにはメニューバーのアイコンから操作してください", "⌘Q schließt das Einstellungsfenster, statt VibeHex zu beenden. Das Diktieren läuft weiter; beenden kannst du VibeHex über das Menüleistensymbol", "⌘Q cierra la ventana de Ajustes en lugar de salir de VibeHex. El dictado sigue funcionando; para salir, usa el icono de la barra de menús", "⌘Q закриває вікно налаштувань, а не завершує VibeHex. Диктування працює далі; щоб вийти, скористайтеся значком у рядку меню"]),
    ("Tell apps on this computer when a meeting's transcript is ready. They get only the meeting's ID, through the VibeHex local API", ["Powiadamiaj aplikacje na tym komputerze, gdy transkrypcja spotkania jest gotowa. Otrzymują tylko identyfikator spotkania, przez lokalne API VibeHex", "会议转写就绪时通知这台电脑上的应用。它们只会通过 VibeHex 的本地 API 收到会议的 ID", "会議の文字起こしができたら、このコンピュータ上のアプリに知らせます。アプリが受け取るのは VibeHex のローカル API 経由の会議 ID だけです", "Benachrichtigt Apps auf diesem Computer, wenn das Transkript eines Meetings fertig ist. Sie erhalten über die lokale API von VibeHex nur die ID des Meetings", "Avisa a las apps de este ordenador cuando la transcripción de una reunión está lista. Solo reciben el ID de la reunión, a través de la API local de VibeHex", "Повідомляє програми на цьому комп'ютері, коли розшифровка зустрічі готова. Вони отримують лише ID зустрічі через локальний API VibeHex"]),
    ("Write it as provider/model, such as openai/gpt-5.4-mini. Until then, VibeHex tries its list of fast models", ["Wpisz jako dostawca/model, np. openai/gpt-5.4-mini. Do tego czasu VibeHex próbuje swojej listy szybkich modeli", "请写成 提供方/模型，例如 openai/gpt-5.4-mini。在此之前，VibeHex 会尝试其快速模型列表", "プロバイダ/モデル の形で書いてください（例: openai/gpt-5.4-mini）。それまでは VibeHex が高速モデルの一覧を試します", "Schreibe es als Anbieter/Modell, etwa openai/gpt-5.4-mini. Bis dahin probiert VibeHex seine Liste schneller Modelle", "Escríbelo como proveedor/modelo, por ejemplo openai/gpt-5.4-mini. Mientras tanto, VibeHex prueba su lista de modelos rápidos", "Запишіть як постачальник/модель, наприклад openai/gpt-5.4-mini. Доти VibeHex пробує свій список швидких моделей"]),
    ("Use the § key, left of 1 on ISO keyboards, as a shortcut without a modifier. VibeHex takes § from every app while it's bound", ["Pozwala użyć klawisza § (na lewo od 1 na klawiaturach ISO) jako skrótu bez modyfikatora. Gdy jest przypisany, VibeHex przejmuje § od wszystkich aplikacji", "将 § 键（ISO 键盘上 1 左侧的键）单独用作快捷键。设为快捷键期间，所有应用都收不到 § 键，由 VibeHex 使用", "§ キー（ISO キーボードで 1 の左にあるキー）を修飾キーなしのショートカットとして使います。割り当てている間、VibeHex はすべてのアプリから § を受け取ります", "Die §-Taste links neben der 1 auf ISO-Tastaturen ohne Zusatztaste als Kürzel verwenden. Solange sie zugewiesen ist, nimmt VibeHex allen Apps die §-Taste weg", "Usa la tecla §, a la izquierda del 1 en teclados ISO, como atajo sin modificador. Mientras esté asignada, VibeHex toma la tecla § de todas las apps", "Використовуйте клавішу § (ліворуч від 1 на клавіатурах ISO) як комбінацію без модифікатора. Поки її призначено, VibeHex забирає § у всіх програм"]),
    ("This desktop doesn't let VibeHex put your clipboard back after a paste. Turn on Labs › Restore clipboard on GNOME to try it.", ["To środowisko nie pozwala VibeHex przywrócić schowka po wklejeniu. Aby spróbować, włącz Laboratorium › Przywracanie schowka w GNOME.", "此桌面不允许 VibeHex 在粘贴后恢复剪贴板。要试用，请打开实验室 › 在 GNOME 上恢复剪贴板。", "このデスクトップでは、VibeHex は貼り付け後にクリップボードを元に戻せません。試すには ラボ › GNOME でクリップボードを復元 をオンにしてください。", "Auf diesem Desktop kann VibeHex die Zwischenablage nach dem Einfügen nicht wiederherstellen. Zum Ausprobieren Labs › Zwischenablage unter GNOME wiederherstellen einschalten.", "Este escritorio no deja que VibeHex restaure tu portapapeles después de pegar. Para probarlo, activa Labs › Restaurar el portapapeles en GNOME.", "Це середовище не дає VibeHex відновити буфер обміну після вставлення. Щоб спробувати, увімкніть Лабораторію › Відновлення буфера обміну в GNOME."]),
    ("VibeHex can't listen until the speech model has downloaded", ["VibeHex nie może słuchać, dopóki model mowy się nie pobierze", "语音模型下载完成前，VibeHex 无法聆听", "音声モデルのダウンロードが終わるまで、VibeHex は聞き取れません", "VibeHex kann erst zuhören, wenn das Sprachmodell heruntergeladen ist", "VibeHex no puede escuchar hasta que se descargue el modelo de voz", "VibeHex не може слухати, доки мовна модель не завантажиться"]),
    ("Your desktop assigns this key itself, so VibeHex can't confirm it", ["Twój pulpit sam przypisuje ten klawisz, więc VibeHex nie może tego potwierdzić", "你的桌面自行分配此按键，因此 VibeHex 无法确认", "デスクトップがこのキーを独自に割り当てるため、VibeHex では確認できません", "Dein Desktop weist diese Taste selbst zu, daher kann VibeHex sie nicht bestätigen", "Tu escritorio asigna esta tecla por su cuenta, así que VibeHex no puede confirmarla", "Ваша стільниця сама призначає цю клавішу, тож VibeHex не може це підтвердити"]),
    ("Stop a recording VibeHex started on its own 30 s after the call app leaves the microphone and goes quiet", ["Zatrzymuje nagranie rozpoczęte samodzielnie przez VibeHex 30 s po tym, jak aplikacja rozmowy zwolni mikrofon i ucichnie", "VibeHex 自动开始的录制会在通话应用释放麦克风并静音 30 秒后停止", "VibeHex が自動で始めた録音を、通話アプリがマイクを離れて音が止まってから 30 秒後に停止します", "Beendet eine Aufnahme, die VibeHex selbst gestartet hat, 30 s nachdem die Anruf-App das Mikrofon freigibt und verstummt", "Detiene una grabación que VibeHex inició por su cuenta 30 s después de que la app de llamadas suelte el micrófono y se quede en silencio", "Зупиняє запис, який VibeHex почав сам, через 30 с після того, як програма дзвінка звільнить мікрофон і затихне"]),
    ("VibeHex stops recording in 30 s unless you keep recording.", ["VibeHex zatrzyma nagrywanie za 30 s, chyba że je kontynuujesz.", "除非你继续录制，VibeHex 将在 30 秒后停止录制。", "録音を続けない場合、VibeHex は 30 秒後に録音を停止します。", "VibeHex beendet die Aufnahme in 30 s, außer du nimmst weiter auf.", "VibeHex dejará de grabar en 30 s a menos que sigas grabando.", "VibeHex зупинить запис через 30 с, якщо ви не продовжите запис."]),
    ("While dictating with Pause media: YouTube and other tabs, resumed only if VibeHex paused them", ["Podczas dyktowania z Wstrzymaj multimedia: YouTube i inne karty, wznawiane tylko, jeśli wstrzymał je VibeHex", "听写时搭配“暂停媒体”：YouTube 等标签页，仅在由 VibeHex 暂停时才会恢复", "「メディアを一時停止」での音声入力中：YouTube などのタブ。VibeHex が止めた場合だけ再開します", "Beim Diktieren mit „Medien pausieren“: YouTube und andere Tabs, nur fortgesetzt, wenn VibeHex sie pausiert hat", "Al dictar con Pausar multimedia: YouTube y otras pestañas, reanudadas solo si VibeHex las pausó", "Під час диктування з «Призупиняти медіа»: YouTube та інші вкладки, відновлюються, лише якщо їх призупинив VibeHex"]),
    ("{model} is missing, so VibeHex uses {fallback} until you import it again or choose another model", ["Brakuje modelu {model}, więc VibeHex używa {fallback}, dopóki nie zaimportujesz go ponownie albo nie wybierzesz innego modelu", "{model} 不见了，因此在你重新导入或选择其他模型之前，VibeHex 将使用 {fallback}", "{model} が見つからないため、もう一度取り込むか別のモデルを選ぶまで VibeHex は {fallback} を使います", "{model} fehlt, daher verwendet VibeHex {fallback}, bis du es erneut importierst oder ein anderes Modell wählst", "Falta {model}, así que VibeHex usa {fallback} hasta que lo vuelvas a importar o elijas otro modelo", "{model} зникла, тож VibeHex використовує {fallback}, доки ви не імпортуєте її знову або не виберете іншу модель"]),
    ("{model} changed since it was added, so VibeHex uses {fallback} until you check it again in the speech model picker or choose another model", ["Model {model} zmienił się od dodania, więc VibeHex używa {fallback}, dopóki nie sprawdzisz go ponownie w wyborze modelu mowy albo nie wybierzesz innego modelu", "{model} 在添加后已更改，因此在你从语音模型列表重新检查它或选择其他模型之前，VibeHex 将使用 {fallback}", "{model} は追加後に変わったため、音声モデルの一覧でもう一度確認するか別のモデルを選ぶまで VibeHex は {fallback} を使います", "{model} hat sich seit dem Hinzufügen geändert, daher verwendet VibeHex {fallback}, bis du es in der Sprachmodell-Auswahl erneut prüfst oder ein anderes Modell wählst", "{model} cambió desde que se añadió, así que VibeHex usa {fallback} hasta que lo compruebes de nuevo en el selector de modelos de voz o elijas otro modelo", "{model} змінилася після додавання, тож VibeHex використовує {fallback}, доки ви не перевірите її знову у виборі мовної моделі або не виберете іншу модель"]),
    ("Not tested by VibeHex", ["Nie testowany przez VibeHex", "未经 VibeHex 测试", "VibeHex では未テスト", "Nicht von VibeHex getestet", "No probado por VibeHex", "Не перевірена VibeHex"]),
    ("On this computer, not tested by VibeHex", ["Na tym komputerze, nie testowany przez VibeHex", "在这台电脑上，未经 VibeHex 测试", "このコンピュータにあります、VibeHex では未テスト", "Auf diesem Computer, nicht von VibeHex getestet", "En este ordenador, no probado por VibeHex", "На цьому комп’ютері, не перевірена VibeHex"]),
    ("Experimental: VibeHex has run it only on Linux (Vulkan) so far.", ["Eksperymentalny: VibeHex uruchamiał go dotąd tylko na Linuksie (Vulkan).", "实验性：VibeHex 目前只在 Linux（Vulkan）上运行过它。", "試験的: VibeHex はこれまで Linux（Vulkan）でのみ動かしています。", "Experimentell: VibeHex hat es bisher nur unter Linux (Vulkan) ausgeführt.", "Experimental: VibeHex solo lo ha ejecutado en Linux (Vulkan) hasta ahora.", "Експериментально: VibeHex поки запускав її лише в Linux (Vulkan)."]),
    ("Published conversion benchmarks; not measured in VibeHex", ["Opublikowane testy konwersji; niezmierzone w VibeHex", "已发布的转换基准测试；未在 VibeHex 中测量", "公開された変換ベンチマーク。VibeHex では未測定", "Veröffentlichte Konvertierungs-Benchmarks; nicht in VibeHex gemessen", "Pruebas de conversión publicadas; no medidas en VibeHex", "Опубліковані тести конвертації; не виміряно в VibeHex"]),
    ("{model} needs Labs › Parakeet Ultra, so VibeHex uses {fallback} until you turn it on or choose another model", ["{model} wymaga Laboratorium › Parakeet Ultra, więc VibeHex używa {fallback}, dopóki go nie włączysz albo nie wybierzesz innego modelu", "{model} 需要开启实验室 › Parakeet Ultra，因此在你开启它或选择其他模型之前，VibeHex 将使用 {fallback}", "{model} にはラボ › Parakeet Ultra が必要なため、オンにするか別のモデルを選ぶまで VibeHex は {fallback} を使います", "{model} braucht Labs › Parakeet Ultra, daher verwendet VibeHex {fallback}, bis du es aktivierst oder ein anderes Modell wählst", "{model} necesita Labs › Parakeet Ultra, así que VibeHex usa {fallback} hasta que lo actives o elijas otro modelo", "{model} потребує Лабораторія › Parakeet Ultra, тож VibeHex використовує {fallback}, доки ви її не увімкнете або не виберете іншу модель"]),
    ("Off. VibeHex records nothing and keeps no shortcut until you turn this on.", ["Wyłączone. VibeHex niczego nie nagrywa i nie zajmuje skrótu, dopóki tego nie włączysz.", "已关闭。打开之前，VibeHex 不录制任何内容，也不占用快捷键。", "オフです。オンにするまで、VibeHex は何も録画せず、ショートカットも使いません。", "Aus. VibeHex nimmt nichts auf und belegt kein Kürzel, bis du es einschaltest.", "Desactivado. VibeHex no graba nada ni ocupa ningún atajo hasta que lo actives.", "Вимкнено. VibeHex нічого не записує й не займає комбінацію клавіш, доки ви це не ввімкнете."]),
    ("The model VibeHex dictates with, with a time for every word", ["Model, którym VibeHex dyktuje, z czasem każdego słowa", "VibeHex 听写所用的模型，并标注每个词的时间", "VibeHex が音声入力に使うモデル。単語ごとの時刻付き", "Das Modell, mit dem VibeHex diktiert, mit einer Zeit für jedes Wort", "El modelo con el que VibeHex dicta, con el tiempo de cada palabra", "Модель, якою диктує VibeHex, з часом для кожного слова"]),
    ("VibeHex leaves the Annotate shortcut to Scrawl until you switch", ["VibeHex zostawia skrót Adnotacji aplikacji Scrawl, dopóki nie przełączysz", "在你切换之前，VibeHex 会把 Annotate 快捷键留给 Scrawl", "切り替えるまで、VibeHex は Annotate のショートカットを Scrawl に任せます", "VibeHex überlässt Scrawl das Annotate-Kürzel, bis du wechselst", "VibeHex deja el atajo de Annotate a Scrawl hasta que cambies", "VibeHex залишає комбінацію Annotate для Scrawl, доки ви не перемкнетеся"]),
    ("Use VibeHex for annotations", ["Używaj VibeHex do adnotacji", "用 VibeHex 批注", "注釈に VibeHex を使う", "VibeHex für Annotationen verwenden", "Usar VibeHex para anotar", "Використовувати VibeHex для анотацій"]),
    ("Lets recordings, annotations included, show the VibeHex capsule, ink, and HUD, for demos", ["Pozwala nagraniom, także adnotacjom, pokazywać kapsułę, atrament i HUD VibeHex, do prezentacji", "让录屏（包括批注）显示 VibeHex 的胶囊、墨迹和 HUD，用于演示", "録画（注釈を含む）に VibeHex のカプセル、インク、HUD を映します。デモ用です", "Lässt Aufnahmen, auch Annotationen, Kapsel, Tinte und HUD von VibeHex zeigen, für Demos", "Permite que las grabaciones, anotaciones incluidas, muestren la cápsula, la tinta y el HUD de VibeHex, para demos", "Дозволяє записам, зокрема анотаціям, показувати капсулу, чорнило й HUD VibeHex, для демонстрацій"]),
    ("Change the keys your desktop gave VibeHex", ["Zmień klawisze, które pulpit przydzielił VibeHex", "更改桌面分配给 VibeHex 的按键", "デスクトップが VibeHex に割り当てたキーを変更", "Tasten ändern, die der Desktop VibeHex zugewiesen hat", "Cambia las teclas que el escritorio asignó a VibeHex", "Змінити клавіші, які система призначила VibeHex"]),
    ("Added to GNOME Settings › Keyboard › Custom Shortcuts while VibeHex is enabled. GNOME runs them on the key press, so your dictation shortcut toggles dictation. Change the keys there.", ["Dodane do Ustawień GNOME › Klawiatura › Własne skróty, gdy VibeHex jest włączony. GNOME uruchamia je przy naciśnięciu klawisza, więc skrót dyktowania je przełącza. Klawisze zmienisz tam.", "VibeHex 启用时已添加到 GNOME 设置 › 键盘 › 自定义快捷键。GNOME 在按下按键时运行它们，因此听写快捷键会切换听写。可在那里更改按键。", "VibeHex が有効な間、GNOME 設定 › キーボード › カスタムショートカットに追加されます。GNOME はキーを押したときに実行するため、音声入力のショートカットは切り替え式です。キーはそこで変更できます。", "Zu GNOME-Einstellungen › Tastatur › Eigene Tastenkürzel hinzugefügt, solange VibeHex aktiv ist. GNOME führt sie beim Tastendruck aus, daher schaltet dein Diktat-Kürzel das Diktat um. Ändere die Tasten dort.", "Añadidos a Configuración de GNOME › Teclado › Atajos personalizados mientras VibeHex está activado. GNOME los ejecuta al pulsar la tecla, así que tu atajo de dictado lo alterna. Cambia las teclas allí.", "Додано до Налаштувань GNOME › Клавіатура › Власні комбінації, поки VibeHex увімкнено. GNOME запускає їх під час натискання клавіші, тож ваша комбінація диктування перемикає його. Змініть клавіші там."]),
    ("GNOME uses {} itself, so it never reaches VibeHex. Record another shortcut, or change it in GNOME Settings › Keyboard › Keyboard Shortcuts.", ["GNOME używa {} do własnych celów, więc nie dociera on do VibeHex. Nagraj inny skrót albo zmień go w Ustawieniach GNOME › Klawiatura › Skróty klawiszowe.", "GNOME 自己使用 {}，所以它不会传到 VibeHex。请录制其他快捷键，或在 GNOME 设置 › 键盘 › 键盘快捷键中更改。", "GNOME が {} を使用しているため、VibeHex に届きません。別のショートカットを記録するか、GNOME 設定 › キーボード › キーボードショートカットで変更してください。", "GNOME verwendet {} selbst, daher erreicht es VibeHex nie. Nimm ein anderes Tastenkürzel auf oder ändere es in den GNOME-Einstellungen › Tastatur › Tastaturkürzel.", "GNOME usa {} para sí mismo, así que nunca llega a VibeHex. Graba otro atajo o cámbialo en Configuración de GNOME › Teclado › Atajos de teclado.", "GNOME використовує {} сам, тому вона не доходить до VibeHex. Запишіть іншу комбінацію або змініть її в Налаштуваннях GNOME › Клавіатура › Комбінації клавіш."]),
    ("VibeHex uses the CPU because the graphics driver's Vulkan is missing or too old. Update the GPU driver to use the GPU.", ["VibeHex używa CPU, bo sterownik grafiki nie ma obsługi Vulkan albo jest ona za stara. Zaktualizuj sterownik GPU, aby używać GPU.", "VibeHex 使用 CPU，因为显卡驱动缺少 Vulkan 或版本过旧。更新 GPU 驱动即可使用 GPU。", "グラフィックドライバーに Vulkan がないか古いため、VibeHex は CPU を使います。GPU を使うには GPU ドライバーを更新してください。", "VibeHex nutzt die CPU, weil der Grafiktreiber kein oder ein zu altes Vulkan hat. Aktualisiere den GPU-Treiber, um die GPU zu nutzen.", "VibeHex usa la CPU porque el controlador gráfico no tiene Vulkan o es demasiado antiguo. Actualiza el controlador de la GPU para usarla.", "VibeHex використовує CPU, бо графічний драйвер не має Vulkan або має застарілий. Оновіть драйвер GPU, щоб використовувати GPU."]),
    ("Tell your agent how VibeHex should write", ["Powiedz agentowi, jak VibeHex ma pisać", "告诉你的智能体 VibeHex 该怎么写", "VibeHex の書き方をエージェントに伝えましょう", "Sag deinem Agenten, wie VibeHex schreiben soll", "Dile a tu agente cómo debe escribir VibeHex", "Скажіть агенту, як має писати VibeHex"]),
    ("VibeHex asks again whenever the config changes. Until you allow it, they paste your words unchanged", ["VibeHex zapyta ponownie po każdej zmianie konfiguracji. Dopóki nie zezwolisz, wkleją twoje słowa bez zmian", "配置每次更改后 VibeHex 都会再次询问。在你允许之前，它们会原样粘贴你的话", "設定が変わるたびに VibeHex は再度確認します。許可するまで、言葉はそのまま貼り付けられます", "VibeHex fragt erneut, sobald sich die Konfiguration ändert. Bis du es erlaubst, fügen sie deine Worte unverändert ein", "VibeHex vuelve a preguntar cada vez que cambia la configuración. Hasta que lo permitas, pegan tus palabras sin cambios", "VibeHex питатиме знову після кожної зміни конфігурації. Доки ви не дозволите, вони вставлятимуть ваші слова без змін"]),
    ("No steps yet. VibeHex pastes your words as it heard them", ["Brak kroków. VibeHex wkleja twoje słowa tak, jak je usłyszał", "还没有步骤。VibeHex 会按听到的内容粘贴", "まだステップはありません。VibeHex は聞き取ったとおりに貼り付けます", "Noch keine Schritte. VibeHex fügt deine Worte so ein, wie es sie gehört hat", "Aún no hay pasos. VibeHex pega tus palabras tal como las oyó", "Кроків ще немає. VibeHex вставляє ваші слова так, як їх почув"]),
    ("What VibeHex pastes", ["Co wklei VibeHex", "VibeHex 粘贴的内容", "VibeHex が貼り付ける内容", "Was VibeHex einfügt", "Lo que pega VibeHex", "Що вставить VibeHex"]),
    ("Pastes your words as VibeHex heard them", ["Wkleja słowa tak, jak VibeHex je usłyszał", "按 VibeHex 听到的内容粘贴", "VibeHex が聞き取ったとおりに貼り付けます", "Fügt deine Worte so ein, wie VibeHex sie gehört hat", "Pega tus palabras tal como VibeHex las oyó", "Вставляє слова так, як VibeHex їх почув"]),
    ("Paste your words as VibeHex heard them, skipping every step", ["Wklej słowa tak, jak VibeHex je usłyszał, z pominięciem wszystkich kroków", "按 VibeHex 听到的内容粘贴，跳过所有步骤", "すべてのステップを省略し、VibeHex が聞き取ったとおりに貼り付けます", "Füge deine Worte so ein, wie VibeHex sie gehört hat, ohne jeden Schritt", "Pega tus palabras tal como VibeHex las oyó, sin ningún paso", "Вставити слова так, як VibeHex їх почув, без жодних кроків"]),
    ("Lets VibeHex hear whether a tab is playing", ["Pozwala VibeHex usłyszeć, czy karta odtwarza dźwięk", "让 VibeHex 能听出标签页是否在播放", "タブが再生中かどうかを VibeHex が聞き取れるようにします", "Damit VibeHex hört, ob ein Tab abspielt", "Permite a VibeHex oír si una pestaña está reproduciendo", "Дозволяє VibeHex чути, чи відтворює вкладка звук"]),
    ("Hex Microphone is the system default, so VibeHex listens to {}", ["Hex Microphone jest domyślny w systemie, więc VibeHex słucha przez {}", "Hex Microphone 是系统默认设备，因此 VibeHex 使用 {}", "Hex Microphone がシステムのデフォルトのため、VibeHex は {} を使います", "Hex Microphone ist der Systemstandard, daher nutzt VibeHex {}", "Hex Microphone es el predeterminado del sistema, así que VibeHex usa {}", "Hex Microphone — системний типовий мікрофон, тож VibeHex слухає {}"]),
    ("{built_in} is off while the lid is closed, so VibeHex listens to {input}", ["{built_in} jest wyłączony przy zamkniętej pokrywie, więc VibeHex słucha przez {input}", "合盖时 {built_in} 处于关闭状态，因此 VibeHex 使用 {input}", "蓋を閉じている間は {built_in} がオフのため、VibeHex は {input} を使います", "{built_in} ist bei geschlossenem Deckel aus, daher nutzt VibeHex {input}", "{built_in} está apagado con la tapa cerrada, así que VibeHex usa {input}", "{built_in} вимкнено, поки кришку закрито, тож VibeHex слухає {input}"]),
    ("VibeHex could not apply this change.", ["VibeHex nie mógł zastosować tej zmiany.", "VibeHex 无法应用此更改。", "VibeHex はこの変更を適用できませんでした。", "VibeHex konnte diese Änderung nicht übernehmen.", "VibeHex no pudo aplicar este cambio.", "VibeHex не вдалося застосувати цю зміну."]),
    ("Let VibeHex hear you", ["Pozwól VibeHex cię słyszeć", "让 VibeHex 听到你", "VibeHex にマイクを使わせてください", "Lass VibeHex dich hören", "Deja que VibeHex te escuche", "Дозвольте VibeHex чути вас"]),
    ("VibeHex could not load the OpenCode model catalog.", ["VibeHex nie mógł wczytać katalogu modeli OpenCode.", "VibeHex 无法加载 OpenCode 模型目录。", "VibeHex は OpenCode のモデル一覧を読み込めませんでした。", "VibeHex konnte den OpenCode-Modellkatalog nicht laden.", "VibeHex no pudo cargar el catálogo de modelos de OpenCode.", "VibeHex не вдалося завантажити каталог моделей OpenCode."]),
    ("Allow VibeHex under Privacy & Security › Calendars to get meeting reminders", ["Zezwól VibeHex w Prywatność i ochrona › Kalendarze, aby dostawać przypomnienia o spotkaniach", "在“隐私与安全性 › 日历”中允许 VibeHex 以获取会议提醒", "会議のリマインダーを受け取るには、「プライバシーとセキュリティ › カレンダー」で VibeHex を許可してください", "Erlaube VibeHex unter „Datenschutz & Sicherheit › Kalender“, um an Meetings erinnert zu werden", "Permite VibeHex en Privacidad y seguridad › Calendarios para recibir recordatorios de reuniones", "Дозвольте VibeHex у розділі Приватність і безпека › Календарі, щоб отримувати нагадування про зустрічі"]),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash", ["Nagrania, transkrypcje, notatki i kopia Markdown utworzona przez VibeHex trafiają do kosza", "音频、转写、笔记和 VibeHex 生成的 Markdown 副本会移到废纸篓", "音声、文字起こし、メモ、VibeHex が作成した Markdown のコピーはゴミ箱に移動します", "Audio, Transkripte, Notizen und die Markdown-Kopie von VibeHex kommen in den Papierkorb", "El audio, las transcripciones, las notas y la copia Markdown de VibeHex se mueven a la papelera", "Аудіо, транскрипти, нотатки й копія Markdown від VibeHex переміщуються до Смітника"]),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash. Meetings from before this setting stay until you choose", ["Nagrania, transkrypcje, notatki i kopia Markdown utworzona przez VibeHex trafiają do kosza. Spotkania sprzed tego ustawienia zostają, dopóki nie wybierzesz", "音频、转写、笔记和 VibeHex 生成的 Markdown 副本会移到废纸篓。此设置出现前的会议会保留，直到你做出选择", "音声、文字起こし、メモ、VibeHex が作成した Markdown のコピーはゴミ箱に移動します。この設定より前の会議は、選ぶまで残ります", "Audio, Transkripte, Notizen und die Markdown-Kopie von VibeHex kommen in den Papierkorb. Meetings von vor dieser Einstellung bleiben, bis du wählst", "El audio, las transcripciones, las notas y la copia Markdown de VibeHex se mueven a la papelera. Las reuniones anteriores a este ajuste se conservan hasta que elijas", "Аудіо, транскрипти, нотатки й копія Markdown від VibeHex переміщуються до Смітника. Зустрічі, записані до цього налаштування, лишаються, доки ви не оберете"]),
    ("OpenCode transformation will be available after VibeHex connects to OpenCode.", ["Transformacja OpenCode będzie dostępna, gdy VibeHex połączy się z OpenCode.", "VibeHex 连接到 OpenCode 后即可使用 OpenCode 转换。", "VibeHex が OpenCode に接続すると OpenCode 変換を使えます。", "Die OpenCode-Transformation ist verfügbar, sobald VibeHex mit OpenCode verbunden ist.", "La transformación de OpenCode estará disponible cuando VibeHex se conecte a OpenCode.", "Перетворення за допомогою OpenCode стане доступним після підключення VibeHex до OpenCode."]),
    ("{bluetooth} is Bluetooth, so VibeHex listens to {input}", ["{bluetooth} to Bluetooth, więc VibeHex słucha przez {input}", "{bluetooth} 是蓝牙设备，因此 VibeHex 使用 {input}", "{bluetooth} は Bluetooth のため、VibeHex は {input} を使います", "{bluetooth} ist Bluetooth, daher nutzt VibeHex {input}", "{bluetooth} es Bluetooth, así que VibeHex usa {input}", "{bluetooth} — це Bluetooth, тож VibeHex слухає {input}"]),
    ("VibeHex signature", ["Sygnatura VibeHex", "VibeHex 标志音", "VibeHex シグネチャー", "VibeHex-Signatur", "Firma de VibeHex", "Фірмовий VibeHex"]),
    ("The tones VibeHex first shipped with", ["Dźwięki, z którymi VibeHex zadebiutował", "VibeHex 最初自带的提示音", "VibeHex が最初に搭載していた音", "Die Töne, mit denen VibeHex zuerst erschien", "Los tonos con los que VibeHex se lanzó", "Звуки, з якими VibeHex вийшов уперше"]),
    ("VibeHex cannot recognize the dictation shortcut or paste into other apps.", ["VibeHex nie rozpozna skrótu dyktowania ani nie wklei tekstu w innych aplikacjach.", "VibeHex 无法在其他应用中识别听写快捷键或粘贴文字。", "VibeHex はほかのアプリで音声入力ショートカットを認識できず、貼り付けもできません。", "VibeHex erkennt das Diktierkürzel in anderen Apps nicht und kann dort nichts einfügen.", "VibeHex no puede reconocer el atajo de dictado ni pegar en otras apps.", "VibeHex не може розпізнати сполучення клавіш для диктування або вставити текст в інші програми."]),
    ("VibeHex cannot record dictation until microphone access is restored.", ["VibeHex nie może nagrywać dyktowania, dopóki dostęp do mikrofonu nie zostanie przywrócony.", "在恢复麦克风访问权限之前，VibeHex 无法录制听写。", "マイクへのアクセスが戻るまで、VibeHex は音声入力を録音できません。", "VibeHex kann keine Diktate aufnehmen, bis der Mikrofonzugriff wiederhergestellt ist.", "VibeHex no puede grabar dictados hasta que se restablezca el acceso al micrófono.", "VibeHex не може записувати диктування, доки доступ до мікрофона не буде відновлено."]),
    ("VibeHex is still recording.", ["VibeHex nadal nagrywa.", "VibeHex 仍在录制。", "VibeHex はまだ録音中です。", "VibeHex nimmt weiter auf.", "VibeHex sigue grabando.", "VibeHex досі записує."]),
    ("Enable VibeHex to dictate into any app with {}", ["Włącz VibeHex, aby dyktować w dowolnej aplikacji za pomocą {}", "启用 VibeHex 后，可用 {} 在任意应用中听写", "VibeHex を有効にすると、{} でどのアプリにも音声入力できます", "Aktiviere VibeHex, um mit {} in jede App zu diktieren", "Activa VibeHex para dictar en cualquier app con {}", "Увімкніть VibeHex, щоб диктувати в будь-якій програмі за допомогою {}"]),
    ("Could not restart VibeHex: {}", ["Nie udało się ponownie uruchomić VibeHex: {}", "无法重启 VibeHex：{}", "VibeHex を再起動できませんでした: {}", "VibeHex konnte nicht neu gestartet werden: {}", "No se pudo reiniciar VibeHex: {}", "Не вдалося перезапустити VibeHex: {}"]),
    ("VibeHex starts listening once the speech model has downloaded.", ["VibeHex zacznie słuchać, gdy model mowy się pobierze.", "语音模型下载完成后，VibeHex 就会开始聆听。", "音声モデルのダウンロードが終わると、VibeHex が聞き取りを始めます。", "VibeHex hört zu, sobald das Sprachmodell geladen ist.", "VibeHex empezará a escuchar cuando termine de descargarse el modelo de voz.", "VibeHex почне слухати після завантаження моделі розпізнавання мовлення."]),
    ("Let VibeHex paste for you", ["Pozwól VibeHex wklejać za ciebie", "让 VibeHex 替你粘贴", "VibeHex に貼り付けを任せる", "Lass VibeHex für dich einfügen", "Deja que VibeHex pegue por ti", "Дозвольте VibeHex вставляти за вас"]),
    ("VibeHex needs this to type your words into your apps. Your desktop asks once, and may call it remote desktop.", ["VibeHex potrzebuje tego, by wpisywać twoje słowa w aplikacjach. Pulpit zapyta raz i może nazwać to zdalnym pulpitem.", "VibeHex 需要此权限才能把你的话输入到应用中。桌面只会询问一次，可能会称之为“远程桌面”。", "VibeHex がアプリに言葉を入力するために必要です。デスクトップが一度だけ確認し、「リモートデスクトップ」と表示することがあります。", "VibeHex braucht das, um deine Worte in deine Apps zu schreiben. Dein Desktop fragt einmal und nennt es vielleicht Fernsteuerung.", "VibeHex lo necesita para escribir tus palabras en tus apps. Tu escritorio lo pregunta una vez y puede llamarlo escritorio remoto.", "VibeHex потрібен цей дозвіл, щоб вводити ваші слова в застосунки. Стільниця запитає один раз і може назвати це віддаленою стільницею."]),
    ("VibeHex only sends the paste keystroke. It never sees your screen.", ["VibeHex wysyła tylko skrót wklejania. Nigdy nie widzi twojego ekranu.", "VibeHex 只发送粘贴按键，从不查看你的屏幕。", "VibeHex は貼り付けのキー操作を送るだけで、画面を見ることはありません。", "VibeHex sendet nur den Einfüge-Tastendruck. Deinen Bildschirm sieht es nie.", "VibeHex solo envía la tecla de pegar. Nunca ve tu pantalla.", "VibeHex лише надсилає комбінацію вставлення. Він ніколи не бачить ваш екран."]),
    ("Without it, VibeHex copies your words and you press Ctrl+V.", ["Bez tego VibeHex kopiuje twoje słowa, a ty naciskasz Ctrl+V.", "没有此权限时，VibeHex 会复制你的话，由你按 Ctrl+V。", "許可しない場合、VibeHex が言葉をコピーし、あなたが Ctrl+V を押します。", "Ohne das kopiert VibeHex deine Worte und du drückst Strg+V.", "Sin esto, VibeHex copia tus palabras y tú pulsas Ctrl+V.", "Без цього VibeHex копіює ваші слова, а ви натискаєте Ctrl+V."]),
    ("Ready the moment you press the shortcut. macOS shows the mic in use while VibeHex runs; you can change that in Settings", ["Gotowy, gdy tylko naciśniesz skrót. macOS pokazuje, że mikrofon jest używany, dopóki działa VibeHex; możesz to zmienić w Ustawieniach", "按下快捷键即可开始。VibeHex 运行时，macOS 会显示麦克风正在使用；你可以在设置中更改", "ショートカットを押した瞬間から使えます。VibeHex の実行中、macOS はマイクを使用中と表示します。設定で変更できます", "Bereit, sobald du das Kürzel drückst. macOS zeigt das Mikrofon als aktiv, solange VibeHex läuft; das kannst du in den Einstellungen ändern", "Listo en cuanto pulsas el atajo. macOS muestra el micrófono en uso mientras VibeHex está abierto; puedes cambiarlo en Ajustes", "Готовий, щойно ви натиснете комбінацію клавіш. Поки VibeHex працює, macOS показує, що мікрофон використовується; це можна змінити в Налаштуваннях"]),
    ("Let VibeHex hear you and type for you", ["Pozwól VibeHex słyszeć cię i pisać za ciebie", "让 VibeHex 听见你并替你输入", "VibeHex が聞き取り、入力できるようにする", "Lass VibeHex zuhören und für dich tippen", "Deja que VibeHex te escuche y escriba por ti", "Дозвольте VibeHex слухати вас і друкувати за вас"]),
    ("Quit and reopen VibeHex to finish setup. macOS applies some permissions only after a restart.", ["Zamknij i ponownie otwórz VibeHex, aby dokończyć konfigurację. macOS stosuje niektóre uprawnienia dopiero po ponownym uruchomieniu.", "请退出并重新打开 VibeHex 以完成设置。macOS 的部分权限需要重启后才会生效。", "設定を完了するには VibeHex を終了して開き直してください。macOS の一部の許可は再起動後に有効になります。", "Beende VibeHex und öffne es erneut, um die Einrichtung abzuschließen. macOS übernimmt manche Berechtigungen erst nach einem Neustart.", "Cierra y vuelve a abrir VibeHex para terminar la configuración. macOS aplica algunos permisos solo tras reiniciar.", "Завершіть роботу VibeHex і відкрийте його знову, щоб завершити налаштування. macOS застосовує деякі дозволи лише після перезапуску."]),
    ("Reopen VibeHex", ["Otwórz VibeHex ponownie", "重新打开 VibeHex", "VibeHex を開き直す", "VibeHex neu öffnen", "Reabrir VibeHex", "Відкрити VibeHex знову"]),
    ("Restart VibeHex", ["Uruchom VibeHex ponownie", "重新启动 VibeHex", "VibeHex を再起動", "VibeHex neu starten", "Reiniciar VibeHex", "Перезапустити VibeHex"]),
    ("Turns on when VibeHex restarts", ["Włączy się po ponownym uruchomieniu VibeHex", "VibeHex 重新启动后生效", "VibeHex の再起動後に有効になります", "Wird nach einem Neustart von VibeHex aktiv", "Se activa cuando VibeHex se reinicia", "Увімкнеться після перезапуску VibeHex"]),
    ("VibeHex can't dictate until a speech model is installed", ["VibeHex nie może dyktować, dopóki nie zainstalujesz modelu mowy", "安装语音模型后 VibeHex 才能听写", "音声モデルをインストールするまで VibeHex は音声入力できません", "VibeHex kann erst diktieren, wenn ein Sprachmodell installiert ist", "VibeHex no puede dictar hasta que se instale un modelo de voz", "VibeHex не може диктувати, доки не встановлено модель мовлення"]),
    ("Start using VibeHex", ["Zacznij używać VibeHex", "开始使用 VibeHex", "VibeHex を使い始める", "VibeHex verwenden", "Empezar a usar VibeHex", "Почати користуватися VibeHex"]),
    ("Voice Filter isn't in this version of VibeHex. Uninstall its driver to remove Hex Microphone from your inputs.", ["Filtra głosu nie ma w tej wersji VibeHex. Odinstaluj jego sterownik, aby usunąć Hex Microphone z listy wejść.", "此版本的 VibeHex 没有声音滤镜。卸载它的驱动，即可从输入设备中移除 Hex Microphone。", "このバージョンの VibeHex にはボイスフィルターがありません。ドライバをアンインストールすると、入力から Hex Microphone が消えます。", "Diese VibeHex-Version hat keinen Stimmfilter. Deinstalliere seinen Treiber, um Hex Microphone aus deinen Eingängen zu entfernen.", "Esta versión de VibeHex no incluye el filtro de voz. Desinstala su controlador para quitar Hex Microphone de tus entradas.", "У цій версії VibeHex немає голосового фільтра. Видаліть його драйвер, щоб прибрати Hex Microphone зі списку входів."]),
    ("Finishing the speech model so VibeHex can hear you.", ["Kończę model mowy, aby VibeHex mógł cię usłyszeć.", "正在完成语音模型，好让 VibeHex 能听到你。", "VibeHex が聞き取れるよう、音声モデルを仕上げています。", "Das Sprachmodell wird fertiggestellt, damit VibeHex dich hören kann.", "Terminando el modelo de voz para que VibeHex pueda oírte.", "Завершуємо мовну модель, щоб VibeHex міг вас чути."]),
    ("VibeHex didn't hear any words. Try again a little louder.", ["VibeHex nie usłyszał żadnych słów. Spróbuj ponownie trochę głośniej.", "VibeHex 没听到任何词语。请大声一点再试一次。", "VibeHex は言葉を聞き取れませんでした。もう少し大きな声でもう一度どうぞ。", "VibeHex hat keine Wörter gehört. Versuch es etwas lauter noch einmal.", "VibeHex no oyó ninguna palabra. Inténtalo otra vez un poco más alto.", "VibeHex не почув жодного слова. Спробуйте ще раз трохи голосніше."]),
    ("VibeHex found speech models already on this computer.", ["VibeHex znalazł modele mowy, które już są na tym komputerze.", "VibeHex 在这台电脑上找到了已有的语音模型。", "VibeHex はこのコンピューターにある音声モデルを見つけました。", "VibeHex hat Sprachmodelle gefunden, die schon auf diesem Computer sind.", "VibeHex encontró modelos de voz que ya están en este ordenador.", "VibeHex знайшов мовні моделі, які вже є на цьому комп'ютері."]),
    ("VibeHex is ready", ["VibeHex jest gotowy", "VibeHex 已就绪", "VibeHex の準備ができました", "VibeHex ist bereit", "VibeHex está listo", "VibeHex готовий"]),
    ("Getting VibeHex ready to listen…", ["VibeHex przygotowuje się do słuchania…", "VibeHex 正在准备聆听…", "VibeHex が聞き取れるよう準備しています…", "VibeHex macht sich bereit zum Zuhören …", "VibeHex se está preparando para escuchar…", "VibeHex готується слухати…"]),
    ("The VibeHex shortcut didn't start.", ["Skrót VibeHex się nie uruchomił.", "VibeHex 的快捷键没有启动。", "VibeHex のショートカットが開始しませんでした。", "Das Kürzel von VibeHex ist nicht gestartet.", "El atajo de VibeHex no se inició.", "Комбінація клавіш VibeHex не запустилася."]),
    ("Hold a key and talk. VibeHex types it wherever your cursor is.", ["Przytrzymaj klawisz i mów. VibeHex wpisze to tam, gdzie jest kursor.", "按住一个键说话，VibeHex 会把它打在光标所在的位置。", "キーを押したまま話すと、VibeHex がカーソルの位置に入力します。", "Halte eine Taste und sprich. VibeHex tippt es dort, wo dein Cursor ist.", "Mantén una tecla y habla. VibeHex lo escribe donde esté tu cursor.", "Утримуйте клавішу й говоріть. VibeHex надрукує це там, де курсор."]),
    ("I hereby have completed VibeHex onboarding.", ["Niniejszym ukończyłem wprowadzenie do VibeHex.", "我在此确认已完成 VibeHex 入门。", "私は VibeHex のオンボーディングを修了したことをここに証します。", "Hiermit habe ich die Einführung in VibeHex abgeschlossen.", "Por la presente he completado la introducción a VibeHex.", "Цим я завершив знайомство з VibeHex."]),
    ("Setting up VibeHex", ["Konfigurowanie VibeHex", "正在设置 VibeHex", "VibeHex を準備しています", "VibeHex wird eingerichtet", "Configurando VibeHex", "Налаштування VibeHex"]),
    ("Switch on VibeHex in System Settings", ["Włącz VibeHex w Ustawieniach systemowych", "在“系统设置”中打开 VibeHex", "システム設定で VibeHex をオンにしてください", "Schalte VibeHex in den Systemeinstellungen ein", "Activa VibeHex en Ajustes del Sistema", "Увімкніть VibeHex у Системних параметрах"]),
    ("That's VibeHex", ["To jest VibeHex", "这就是 VibeHex", "これが VibeHex です", "Das ist VibeHex", "Eso es VibeHex", "Це VibeHex"]),
    ("With VibeHex", ["Z VibeHex", "用 VibeHex", "VibeHex あり", "Mit VibeHex", "Con VibeHex", "З VibeHex"]),
    ("Without VibeHex", ["Bez VibeHex", "不用 VibeHex", "VibeHex なし", "Ohne VibeHex", "Sin VibeHex", "Без VibeHex"]),
    ("turn VibeHex on in System Settings", ["włącz VibeHex w Ustawieniach systemowych", "在系统设置中开启 VibeHex", "システム設定で VibeHex をオンに", "VibeHex in den Systemeinstellungen einschalten", "activa VibeHex en Ajustes del Sistema", "увімкніть VibeHex у Системних параметрах"]),
    ("turns on when VibeHex restarts", ["włączy się po ponownym uruchomieniu VibeHex", "VibeHex 重启后开启", "VibeHex の再起動後にオン", "aktiv nach dem Neustart von VibeHex", "se activa al reiniciar VibeHex", "увімкнеться після перезапуску VibeHex"]),
    ("VibeHex needs {}", ["VibeHex potrzebuje: {}", "VibeHex 需要{}", "VibeHex には{}が必要です", "VibeHex braucht {}", "VibeHex necesita {}", "VibeHex потребує: {}"]),
    ("Press Caps Lock to start dictation and again to finish. VibeHex keeps Caps Lock off and uses its light while it records and transcribes", ["Naciśnij Caps Lock, aby rozpocząć dyktowanie, i ponownie, aby zakończyć. VibeHex wyłącza Caps Lock i używa jego lampki podczas nagrywania i transkrypcji", "按 Caps Lock 开始听写，再按一次结束。VibeHex 会让 Caps Lock 保持关闭，并在录音和转写时使用它的指示灯", "Caps Lock を押すと音声入力を開始し、もう一度押すと終了します。VibeHex は Caps Lock をオフに保ち、録音と文字起こしの間そのライトを使います", "Caps Lock startet das Diktat, noch einmal beendet es. VibeHex hält Caps Lock aus und nutzt seine Leuchte während Aufnahme und Transkription", "Pulsa Bloq Mayús para empezar a dictar y otra vez para terminar. VibeHex mantiene Bloq Mayús apagado y usa su luz mientras graba y transcribe", "Натисніть Caps Lock, щоб почати диктування, і ще раз, щоб завершити. VibeHex тримає Caps Lock вимкненим і використовує його індикатор під час запису й розпізнавання"]),
    ("An older VibeHex entry can keep this off", ["Starszy wpis VibeHex może to blokować", "旧版 VibeHex 条目可能会导致此项无法开启", "古い VibeHex の項目が妨げになっている場合があります", "Ein älterer VibeHex-Eintrag kann dies blockieren", "Una entrada anterior de VibeHex puede impedirlo", "Старий запис VibeHex може блокувати це"]),
    ("The VibeHex local API can return transcripts with Global mode's processing applied.", ["Aplikacje mogą prosić lokalne API VibeHex o transkrypcje z przetwarzaniem trybu Globalnego", "应用可以向 VibeHex 本地 API 请求应用了全局模式处理的转写", "アプリは VibeHex のローカル API にグローバルモードの処理を適用した文字起こしを要求できます", "Apps können die lokale API von VibeHex um Transkripte mit der Verarbeitung des Global-Modus bitten", "Las apps pueden pedir a la API local de VibeHex transcripciones con el procesamiento del modo Global aplicado", "Програми можуть запитувати в локального API VibeHex транскрипти із застосованою обробкою Глобального режиму"]),
    ("A model VibeHex downloads and runs itself, offline", ["Model, który VibeHex pobiera i uruchamia sam, offline", "VibeHex 自行下载并离线运行的模型", "VibeHex が自分でダウンロードしてオフラインで動かすモデル", "Ein Modell, das VibeHex selbst lädt und offline ausführt", "Un modelo que VibeHex descarga y ejecuta por sí mismo, sin conexión", "Модель, яку VibeHex сам завантажує й запускає офлайн"]),
    ("Your own rules, added to the VibeHex prompt along with your vocabulary", ["Twoje zasady, dodawane do promptu VibeHex razem z Twoim słownictwem", "你自己的规则，会和你的词汇一起加入 VibeHex 的提示词", "あなた独自のルール。語彙と一緒に VibeHex のプロンプトに追加されます", "Deine eigenen Regeln, zusammen mit deinem Vokabular zum Prompt von VibeHex hinzugefügt", "Tus propias reglas, añadidas al prompt de VibeHex junto con tu vocabulario", "Ваші власні правила, що додаються до промпту VibeHex разом зі словником"]),
    ("VibeHex {version} is ready", ["VibeHex {version} jest gotowy", "VibeHex {version} 已就绪", "VibeHex {version} の準備ができました", "VibeHex {version} ist bereit", "VibeHex {version} está listo", "VibeHex {version} готовий"]),
    ("You have {running}. Restart VibeHex to install it now, or it installs when you quit", ["Masz {running}. Uruchom VibeHex ponownie, aby zainstalować teraz, albo zainstaluje się po zamknięciu", "当前版本为 {running}。立即重启 VibeHex 进行安装，或在退出时自动安装", "現在のバージョンは {running} です。今すぐ VibeHex を再起動してインストールするか、終了時にインストールされます", "Du hast {running}. Starte VibeHex neu, um es jetzt zu installieren, oder es wird beim Beenden installiert", "Tienes {running}. Reinicia VibeHex para instalarla ahora, o se instalará al salir", "У вас {running}. Перезапустіть VibeHex, щоб установити зараз, або оновлення встановиться під час виходу"]),
    ("VibeHex can't read this file", ["VibeHex nie odczyta tego pliku", "VibeHex 无法读取此文件", "VibeHex はこのファイルを読めません", "VibeHex kann diese Datei nicht lesen", "VibeHex no puede leer este archivo", "VibeHex не може прочитати цей файл"]),
    ("Use the VibeHex prompt", ["Użyj instrukcji VibeHex", "使用 VibeHex 的提示词", "VibeHex のプロンプトを使う", "VibeHex-Prompt verwenden", "Usar la instrucción de VibeHex", "Інструкція VibeHex"]),
    ("{} isn't an audio or video file VibeHex can read.", ["{} nie jest plikiem audio ani wideo, który VibeHex odczyta.", "{} 不是 VibeHex 能读取的音频或视频文件。", "{} は VibeHex が読める音声・動画ファイルではありません。", "{} ist keine Audio- oder Videodatei, die VibeHex lesen kann.", "{} no es un archivo de audio o vídeo que VibeHex pueda leer.", "{} — не аудіо- чи відеофайл, який VibeHex може прочитати."]),
    ("Your speech model, {}, isn't set to one language VibeHex writes prompts in, so they follow the app's language", ["Twój model mowy, {}, nie ma ustawionego języka, w którym VibeHex pisze prompty, więc prompty są w języku aplikacji", "你的语音模型 {} 未设为 VibeHex 提供提示词的某一种语言，因此提示词跟随应用语言", "音声モデル {} が VibeHex のプロンプトがある言語に設定されていないため、プロンプトはアプリの言語に従います", "Dein Sprachmodell, {}, ist auf keine Sprache eingestellt, in der VibeHex Prompts schreibt, daher folgen sie der Sprache der App", "Tu modelo de voz, {}, no está en un idioma para el que VibeHex tenga prompts, así que siguen el idioma de la app", "Ваша модель мовлення, {}, не налаштована на мову, якою VibeHex пише промпти, тож вони мовою застосунку"]),
    ("Not found. VibeHex uses the Codex CLI on this computer: install it with {}, then sign in once", ["Nie znaleziono. VibeHex używa Codex CLI na tym komputerze: zainstaluj go poleceniem {}, a potem zaloguj się raz", "未找到。VibeHex 使用这台电脑上的 Codex CLI：用 {} 安装，然后登录一次", "見つかりません。VibeHex はこのコンピュータの Codex CLI を使います。{} でインストールし、一度サインインしてください", "Nicht gefunden. VibeHex nutzt die Codex CLI auf diesem Computer: Installiere sie mit {} und melde dich einmal an", "No encontrado. VibeHex usa Codex CLI en este ordenador: instálalo con {} e inicia sesión una vez", "Не знайдено. VibeHex використовує Codex CLI на цьому комп’ютері: встановіть його командою {} і один раз увійдіть"]),
    ("Not signed in. Signing in opens your browser; VibeHex never sees your password", ["Nie zalogowano. Logowanie otwiera przeglądarkę; VibeHex nigdy nie widzi Twojego hasła", "未登录。登录会打开浏览器；VibeHex 永远看不到你的密码", "サインインしていません。サインインするとブラウザが開きます。VibeHex がパスワードを見ることはありません", "Nicht angemeldet. Die Anmeldung öffnet deinen Browser; VibeHex sieht dein Passwort nie", "Sin sesión iniciada. Iniciar sesión abre tu navegador; VibeHex nunca ve tu contraseña", "Ви не ввійшли. Вхід відкриває браузер; VibeHex ніколи не бачить ваш пароль"]),
];

#[rustfmt::skip]
pub(super) const FRENCH: &[(&str, &str)] = &[
    ("VibeHex picks a fast model", "VibeHex choisit un modèle rapide"),
    ("Summary models show once VibeHex connects to OpenCode.", "Les modèles de résumé s'affichent dès que VibeHex se connecte à OpenCode."),
    ("Start OpenCode to choose a summary model. VibeHex keeps the saved one.", "Lancez OpenCode pour choisir un modèle de résumé. VibeHex garde celui enregistré."),
    ("Hide the VibeHex icon from the menu bar. Shortcuts keep working; open VibeHex again from Finder or Spotlight for Settings", "Masque l'icône de VibeHex dans la barre des menus. Les raccourcis continuent de fonctionner ; rouvrez VibeHex depuis le Finder ou Spotlight pour accéder aux Réglages"),
    ("⌘Q closes the Settings window instead of quitting VibeHex. Dictation keeps running; quit from the menu bar icon", "⌘Q ferme la fenêtre des Réglages au lieu de quitter VibeHex. La dictée continue de fonctionner ; pour quitter, utilisez l'icône de la barre des menus"),
    ("Tell apps on this computer when a meeting's transcript is ready. They get only the meeting's ID, through the VibeHex local API", "Prévient les apps de cet ordinateur quand la transcription d'une réunion est prête. Elles ne reçoivent que l'identifiant de la réunion, via l'API locale de VibeHex"),
    ("Write it as provider/model, such as openai/gpt-5.4-mini. Until then, VibeHex tries its list of fast models", "Écrivez-le sous la forme fournisseur/modèle, par exemple openai/gpt-5.4-mini. En attendant, VibeHex essaie sa liste de modèles rapides"),
    ("Use the § key, left of 1 on ISO keyboards, as a shortcut without a modifier. VibeHex takes § from every app while it's bound", "Utilise la touche §, à gauche du 1 sur les claviers ISO, comme raccourci sans modificateur. Tant qu'elle est attribuée, VibeHex prend la touche § à toutes les apps"),
    ("This desktop doesn't let VibeHex put your clipboard back after a paste. Turn on Labs › Restore clipboard on GNOME to try it.", "Ce bureau ne permet pas à VibeHex de remettre votre presse-papiers après un collage. Activez Labs › Restaurer le presse-papiers sous GNOME pour essayer."),
    ("VibeHex can't listen until the speech model has downloaded", "VibeHex ne peut pas écouter tant que le modèle vocal n'est pas téléchargé"),
    ("Your desktop assigns this key itself, so VibeHex can't confirm it", "Votre bureau attribue cette touche lui-même, VibeHex ne peut donc pas la confirmer"),
    ("Stop a recording VibeHex started on its own 30 s after the call app leaves the microphone and goes quiet", "Arrête un enregistrement lancé automatiquement par VibeHex 30 s après que l'app d'appel a libéré le micro et s'est tue"),
    ("VibeHex stops recording in 30 s unless you keep recording.", "VibeHex arrête l'enregistrement dans 30 s, sauf si vous continuez."),
    ("While dictating with Pause media: YouTube and other tabs, resumed only if VibeHex paused them", "Pendant la dictée avec Mettre en pause les médias : YouTube et les autres onglets, relancés seulement si VibeHex les a mis en pause"),
    ("{model} is missing, so VibeHex uses {fallback} until you import it again or choose another model", "{model} est introuvable, VibeHex utilise donc {fallback} jusqu'à ce que vous le réimportiez ou choisissiez un autre modèle"),
    ("{model} changed since it was added, so VibeHex uses {fallback} until you check it again in the speech model picker or choose another model", "{model} a changé depuis son ajout, VibeHex utilise donc {fallback} jusqu'à ce que vous le vérifiiez à nouveau dans le choix du modèle vocal ou choisissiez un autre modèle"),
    ("Not tested by VibeHex", "Non testé par VibeHex"),
    ("On this computer, not tested by VibeHex", "Sur cet ordinateur, non testé par VibeHex"),
    ("Experimental: VibeHex has run it only on Linux (Vulkan) so far.", "Expérimental : VibeHex ne l'a exécuté jusqu'ici que sous Linux (Vulkan)."),
    ("Published conversion benchmarks; not measured in VibeHex", "Tests de conversion publiés ; non mesurés dans VibeHex"),
    ("{model} needs Labs › Parakeet Ultra, so VibeHex uses {fallback} until you turn it on or choose another model", "{model} nécessite Labs › Parakeet Ultra, VibeHex utilise donc {fallback} jusqu'à ce que vous l'activiez ou choisissiez un autre modèle"),
    ("Off. VibeHex records nothing and keeps no shortcut until you turn this on.", "Désactivé. VibeHex n'enregistre rien et ne réserve aucun raccourci tant que vous ne l'activez pas."),
    ("The model VibeHex dictates with, with a time for every word", "Le modèle de dictée de VibeHex, avec un horodatage pour chaque mot"),
    ("VibeHex leaves the Annotate shortcut to Scrawl until you switch", "VibeHex laisse le raccourci Annotate à Scrawl jusqu'à ce que vous changiez"),
    ("Use VibeHex for annotations", "Utiliser VibeHex pour les annotations"),
    ("Lets recordings, annotations included, show the VibeHex capsule, ink, and HUD, for demos", "Permet aux enregistrements, annotations comprises, de montrer la capsule, l'encre et le HUD de VibeHex, pour les démos"),
    ("Change the keys your desktop gave VibeHex", "Modifiez les touches que votre bureau a attribuées à VibeHex"),
    ("Added to GNOME Settings › Keyboard › Custom Shortcuts while VibeHex is enabled. GNOME runs them on the key press, so your dictation shortcut toggles dictation. Change the keys there.", "Ajoutés dans Paramètres GNOME › Clavier › Raccourcis personnalisés tant que VibeHex est activé. GNOME les exécute à l'appui sur la touche, votre raccourci de dictée fonctionne donc en bascule. Modifiez les touches à cet endroit."),
    ("GNOME uses {} itself, so it never reaches VibeHex. Record another shortcut, or change it in GNOME Settings › Keyboard › Keyboard Shortcuts.", "GNOME utilise {} lui-même, ce raccourci n'atteint donc jamais VibeHex. Enregistrez-en un autre ou modifiez-le dans Paramètres GNOME › Clavier › Raccourcis clavier."),
    ("VibeHex uses the CPU because the graphics driver's Vulkan is missing or too old. Update the GPU driver to use the GPU.", "VibeHex utilise le CPU, car le pilote graphique n'a pas Vulkan ou en a une version trop ancienne. Mettez à jour le pilote GPU pour utiliser le GPU."),
    ("Tell your agent how VibeHex should write", "Dites à votre agent comment VibeHex doit écrire"),
    ("VibeHex asks again whenever the config changes. Until you allow it, they paste your words unchanged", "VibeHex redemande à chaque modification de la configuration. Tant que vous ne l'autorisez pas, elles collent vos mots sans les modifier"),
    ("No steps yet. VibeHex pastes your words as it heard them", "Aucune étape pour l'instant. VibeHex colle vos mots tels qu'il les a entendus"),
    ("What VibeHex pastes", "Ce que VibeHex colle"),
    ("Pastes your words as VibeHex heard them", "Colle vos mots tels que VibeHex les a entendus"),
    ("Paste your words as VibeHex heard them, skipping every step", "Colle vos mots tels que VibeHex les a entendus, sans aucune étape"),
    ("Lets VibeHex hear whether a tab is playing", "Permet à VibeHex d'entendre si un onglet joue du son"),
    ("Hex Microphone is the system default, so VibeHex listens to {}", "Hex Microphone est l'entrée par défaut du système, VibeHex écoute donc {}"),
    ("{built_in} is off while the lid is closed, so VibeHex listens to {input}", "{built_in} est éteint quand l'écran est rabattu, VibeHex écoute donc {input}"),
    ("VibeHex could not apply this change.", "VibeHex n'a pas pu appliquer cette modification."),
    ("Let VibeHex hear you", "Laissez VibeHex vous entendre"),
    ("VibeHex could not load the OpenCode model catalog.", "VibeHex n'a pas pu charger le catalogue de modèles d'OpenCode."),
    ("Allow VibeHex under Privacy & Security › Calendars to get meeting reminders", "Autorisez VibeHex dans Confidentialité et sécurité › Calendriers pour recevoir des rappels de réunion"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash", "L'audio, les transcriptions, les notes et la copie Markdown de VibeHex vont à la Corbeille"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash. Meetings from before this setting stay until you choose", "L'audio, les transcriptions, les notes et la copie Markdown de VibeHex vont à la Corbeille. Les réunions antérieures à ce réglage restent jusqu'à ce que vous choisissiez"),
    ("OpenCode transformation will be available after VibeHex connects to OpenCode.", "La transformation OpenCode sera disponible une fois VibeHex connecté à OpenCode."),
    ("{bluetooth} is Bluetooth, so VibeHex listens to {input}", "{bluetooth} est en Bluetooth, VibeHex écoute donc {input}"),
    ("VibeHex signature", "Signature VibeHex"),
    ("The tones VibeHex first shipped with", "Les sons d'origine de VibeHex"),
    ("VibeHex cannot recognize the dictation shortcut or paste into other apps.", "VibeHex ne peut ni reconnaître le raccourci de dictée ni coller dans d'autres apps."),
    ("VibeHex cannot record dictation until microphone access is restored.", "VibeHex ne peut pas enregistrer de dictée tant que l'accès au micro n'est pas rétabli."),
    ("VibeHex is still recording.", "VibeHex enregistre toujours."),
    ("Enable VibeHex to dictate into any app with {}", "Activez VibeHex pour dicter dans n'importe quelle app avec {}"),
    ("Could not restart VibeHex: {}", "Impossible de redémarrer VibeHex : {}"),
    ("VibeHex starts listening once the speech model has downloaded.", "VibeHex commence à écouter dès que le modèle vocal est téléchargé."),
    ("Let VibeHex paste for you", "Laissez VibeHex coller pour vous"),
    ("VibeHex needs this to type your words into your apps. Your desktop asks once, and may call it remote desktop.", "VibeHex en a besoin pour saisir vos mots dans vos apps. Votre bureau le demande une seule fois et peut appeler cela bureau à distance."),
    ("VibeHex only sends the paste keystroke. It never sees your screen.", "VibeHex envoie seulement le raccourci de collage. Il ne voit jamais votre écran."),
    ("Without it, VibeHex copies your words and you press Ctrl+V.", "Sans cela, VibeHex copie vos mots et vous appuyez sur Ctrl+V."),
    ("Ready the moment you press the shortcut. macOS shows the mic in use while VibeHex runs; you can change that in Settings", "Prêt dès que vous appuyez sur le raccourci. macOS indique que le micro est utilisé tant que VibeHex est ouvert ; vous pouvez changer cela dans les Réglages"),
    ("Let VibeHex hear you and type for you", "Laissez VibeHex vous entendre et écrire pour vous"),
    ("Quit and reopen VibeHex to finish setup. macOS applies some permissions only after a restart.", "Quittez et rouvrez VibeHex pour terminer la configuration. macOS n'applique certaines autorisations qu'après un redémarrage."),
    ("Reopen VibeHex", "Rouvrir VibeHex"),
    ("Restart VibeHex", "Redémarrer VibeHex"),
    ("Turns on when VibeHex restarts", "S'active au redémarrage de VibeHex"),
    ("VibeHex can't dictate until a speech model is installed", "VibeHex ne peut pas dicter tant qu'aucun modèle vocal n'est installé"),
    ("Start using VibeHex", "Commencer à utiliser VibeHex"),
    ("Voice Filter isn't in this version of VibeHex. Uninstall its driver to remove Hex Microphone from your inputs.", "Le filtre vocal n’est pas dans cette version de VibeHex. Désinstallez son pilote pour retirer Hex Microphone de vos entrées."),
    ("Finishing the speech model so VibeHex can hear you.", "Finalisation du modèle vocal pour que VibeHex puisse vous entendre."),
    ("VibeHex didn't hear any words. Try again a little louder.", "VibeHex n'a entendu aucun mot. Réessayez un peu plus fort."),
    ("VibeHex found speech models already on this computer.", "VibeHex a trouvé des modèles vocaux déjà présents sur cet ordinateur."),
    ("VibeHex is ready", "VibeHex est prêt"),
    ("Getting VibeHex ready to listen…", "VibeHex se prépare à écouter…"),
    ("The VibeHex shortcut didn't start.", "Le raccourci de VibeHex n'a pas démarré."),
    ("Hold a key and talk. VibeHex types it wherever your cursor is.", "Maintenez une touche et parlez. VibeHex l'écrit là où se trouve votre curseur."),
    ("I hereby have completed VibeHex onboarding.", "Je certifie avoir terminé la prise en main de VibeHex."),
    ("Setting up VibeHex", "Configuration de VibeHex"),
    ("Switch on VibeHex in System Settings", "Activez VibeHex dans les Réglages Système"),
    ("That's VibeHex", "Voilà VibeHex"),
    ("With VibeHex", "Avec VibeHex"),
    ("Without VibeHex", "Sans VibeHex"),
    ("turn VibeHex on in System Settings", "activez VibeHex dans Réglages Système"),
    ("turns on when VibeHex restarts", "s'active au redémarrage de VibeHex"),
    ("VibeHex needs {}", "VibeHex a besoin de {}"),
    ("Press Caps Lock to start dictation and again to finish. VibeHex keeps Caps Lock off and uses its light while it records and transcribes", "Appuyez sur Verr. Maj pour commencer la dictée et de nouveau pour terminer. VibeHex garde Verr. Maj désactivé et utilise son voyant pendant l'enregistrement et la transcription"),
    ("An older VibeHex entry can keep this off", "Une ancienne entrée VibeHex peut bloquer cette option"),
    ("The VibeHex local API can return transcripts with Global mode's processing applied.", "Les apps peuvent demander à l'API locale de VibeHex des transcriptions avec le traitement du mode Global appliqué"),
    ("A model VibeHex downloads and runs itself, offline", "Un modèle que VibeHex télécharge et exécute lui-même, hors ligne"),
    ("Your own rules, added to the VibeHex prompt along with your vocabulary", "Vos propres règles, ajoutées au prompt de VibeHex avec votre vocabulaire"),
    ("VibeHex {version} is ready", "VibeHex {version} est prêt"),
    ("You have {running}. Restart VibeHex to install it now, or it installs when you quit", "Vous avez {running}. Redémarrez VibeHex pour l'installer maintenant, ou elle s'installera à la fermeture"),
    ("VibeHex can't read this file", "VibeHex ne peut pas lire ce fichier"),
    ("Use the VibeHex prompt", "Utiliser la consigne de VibeHex"),
    ("{} isn't an audio or video file VibeHex can read.", "{} n'est pas un fichier audio ou vidéo que VibeHex peut lire."),
    ("Your speech model, {}, isn't set to one language VibeHex writes prompts in, so they follow the app's language", "Ton modèle vocal, {}, n'est pas réglé sur une langue dans laquelle VibeHex a des prompts, ils suivent donc la langue de l'app"),
    ("Not found. VibeHex uses the Codex CLI on this computer: install it with {}, then sign in once", "Introuvable. VibeHex utilise la CLI Codex de cet ordinateur : installez-la avec {}, puis connectez-vous une fois"),
    ("Not signed in. Signing in opens your browser; VibeHex never sees your password", "Non connecté. La connexion ouvre votre navigateur ; VibeHex ne voit jamais votre mot de passe"),
];

#[rustfmt::skip]
pub(super) const PORTUGUESE: &[(&str, &str)] = &[
    ("VibeHex picks a fast model", "O VibeHex escolhe um modelo rápido"),
    ("Summary models show once VibeHex connects to OpenCode.", "Os modelos de resumo aparecem quando o VibeHex se conecta ao OpenCode."),
    ("Start OpenCode to choose a summary model. VibeHex keeps the saved one.", "Inicie o OpenCode para escolher um modelo de resumo. O VibeHex mantém o salvo."),
    ("Hide the VibeHex icon from the menu bar. Shortcuts keep working; open VibeHex again from Finder or Spotlight for Settings", "Oculta o ícone do VibeHex na barra de menus. Os atalhos continuam funcionando; abra o VibeHex de novo pelo Finder ou Spotlight para ver os Ajustes"),
    ("⌘Q closes the Settings window instead of quitting VibeHex. Dictation keeps running; quit from the menu bar icon", "⌘Q fecha a janela de Ajustes em vez de encerrar o VibeHex. O ditado continua funcionando; para encerrar, use o ícone da barra de menus"),
    ("Tell apps on this computer when a meeting's transcript is ready. They get only the meeting's ID, through the VibeHex local API", "Avisa os apps deste computador quando a transcrição de uma reunião fica pronta. Eles recebem só o ID da reunião, pela API local do VibeHex"),
    ("Write it as provider/model, such as openai/gpt-5.4-mini. Until then, VibeHex tries its list of fast models", "Escreva como provedor/modelo, por exemplo openai/gpt-5.4-mini. Até lá, o VibeHex tenta sua lista de modelos rápidos"),
    ("Use the § key, left of 1 on ISO keyboards, as a shortcut without a modifier. VibeHex takes § from every app while it's bound", "Usa a tecla §, à esquerda do 1 em teclados ISO, como atalho sem modificador. Enquanto estiver atribuída, o VibeHex toma a tecla § de todos os apps"),
    ("This desktop doesn't let VibeHex put your clipboard back after a paste. Turn on Labs › Restore clipboard on GNOME to try it.", "Este desktop não deixa o VibeHex devolver sua área de transferência depois de colar. Ative Labs › Restaurar a área de transferência no GNOME para testar."),
    ("VibeHex can't listen until the speech model has downloaded", "O VibeHex não pode ouvir até o modelo de voz terminar de baixar"),
    ("Your desktop assigns this key itself, so VibeHex can't confirm it", "Seu ambiente de trabalho atribui essa tecla por conta própria, então o VibeHex não pode confirmá-la"),
    ("Stop a recording VibeHex started on its own 30 s after the call app leaves the microphone and goes quiet", "Para uma gravação que o VibeHex iniciou sozinho 30 s depois que o app de chamada solta o microfone e fica em silêncio"),
    ("VibeHex stops recording in 30 s unless you keep recording.", "O VibeHex para de gravar em 30 s, a menos que você continue gravando."),
    ("While dictating with Pause media: YouTube and other tabs, resumed only if VibeHex paused them", "Ao ditar com Pausar mídia: YouTube e outras abas, retomadas só se o VibeHex as tiver pausado"),
    ("{model} is missing, so VibeHex uses {fallback} until you import it again or choose another model", "{model} sumiu, então o VibeHex usa {fallback} até você importá-lo de novo ou escolher outro modelo"),
    ("{model} changed since it was added, so VibeHex uses {fallback} until you check it again in the speech model picker or choose another model", "{model} mudou desde que foi adicionado, então o VibeHex usa {fallback} até você verificá-lo de novo na escolha do modelo de voz ou escolher outro modelo"),
    ("Not tested by VibeHex", "Não testado pelo VibeHex"),
    ("On this computer, not tested by VibeHex", "Neste computador, não testado pelo VibeHex"),
    ("Experimental: VibeHex has run it only on Linux (Vulkan) so far.", "Experimental: até agora o VibeHex só o executou no Linux (Vulkan)."),
    ("Published conversion benchmarks; not measured in VibeHex", "Testes de conversão publicados; não medidos no VibeHex"),
    ("{model} needs Labs › Parakeet Ultra, so VibeHex uses {fallback} until you turn it on or choose another model", "{model} precisa de Labs › Parakeet Ultra, então o VibeHex usa {fallback} até você ativá-lo ou escolher outro modelo"),
    ("Off. VibeHex records nothing and keeps no shortcut until you turn this on.", "Desativado. O VibeHex não grava nada nem ocupa nenhum atalho até você ativar isto."),
    ("The model VibeHex dictates with, with a time for every word", "O modelo com que o VibeHex dita, com o tempo de cada palavra"),
    ("VibeHex leaves the Annotate shortcut to Scrawl until you switch", "O VibeHex deixa o atalho do Annotate para o Scrawl até você trocar"),
    ("Use VibeHex for annotations", "Usar o VibeHex para anotações"),
    ("Lets recordings, annotations included, show the VibeHex capsule, ink, and HUD, for demos", "Permite que as gravações, anotações incluídas, mostrem a cápsula, a tinta e o HUD do VibeHex, para demonstrações"),
    ("Change the keys your desktop gave VibeHex", "Altere as teclas que o ambiente de trabalho deu ao VibeHex"),
    ("Added to GNOME Settings › Keyboard › Custom Shortcuts while VibeHex is enabled. GNOME runs them on the key press, so your dictation shortcut toggles dictation. Change the keys there.", "Adicionados a Configurações do GNOME › Teclado › Atalhos personalizados enquanto o VibeHex está ativado. O GNOME os executa ao pressionar a tecla, então seu atalho de ditado alterna o ditado. Altere as teclas lá."),
    ("GNOME uses {} itself, so it never reaches VibeHex. Record another shortcut, or change it in GNOME Settings › Keyboard › Keyboard Shortcuts.", "O GNOME usa {} para si, então ele nunca chega ao VibeHex. Grave outro atalho ou altere-o em Configurações do GNOME › Teclado › Atalhos de teclado."),
    ("VibeHex uses the CPU because the graphics driver's Vulkan is missing or too old. Update the GPU driver to use the GPU.", "O VibeHex usa a CPU porque o driver gráfico não tem Vulkan ou tem uma versão antiga demais. Atualize o driver da GPU para usar a GPU."),
    ("Tell your agent how VibeHex should write", "Diga ao seu agente como o VibeHex deve escrever"),
    ("VibeHex asks again whenever the config changes. Until you allow it, they paste your words unchanged", "O VibeHex pergunta de novo sempre que a configuração muda. Até você permitir, elas colam suas palavras sem alterações"),
    ("No steps yet. VibeHex pastes your words as it heard them", "Nenhuma etapa ainda. O VibeHex cola suas palavras como as ouviu"),
    ("What VibeHex pastes", "O que o VibeHex cola"),
    ("Pastes your words as VibeHex heard them", "Cola suas palavras como o VibeHex as ouviu"),
    ("Paste your words as VibeHex heard them, skipping every step", "Cola suas palavras como o VibeHex as ouviu, pulando todas as etapas"),
    ("Lets VibeHex hear whether a tab is playing", "Permite que o VibeHex ouça se uma aba está tocando"),
    ("Hex Microphone is the system default, so VibeHex listens to {}", "O Hex Microphone é o padrão do sistema, então o VibeHex ouve {}"),
    ("{built_in} is off while the lid is closed, so VibeHex listens to {input}", "{built_in} fica desligado com a tampa fechada, então o VibeHex ouve {input}"),
    ("VibeHex could not apply this change.", "O VibeHex não conseguiu aplicar esta alteração."),
    ("Let VibeHex hear you", "Deixe o VibeHex ouvir você"),
    ("VibeHex could not load the OpenCode model catalog.", "O VibeHex não conseguiu carregar o catálogo de modelos do OpenCode."),
    ("Allow VibeHex under Privacy & Security › Calendars to get meeting reminders", "Permita o VibeHex em Privacidade e Segurança › Calendários para receber lembretes de reuniões"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash", "Áudio, transcrições, notas e a cópia em Markdown do VibeHex vão para a Lixeira"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash. Meetings from before this setting stay until you choose", "Áudio, transcrições, notas e a cópia em Markdown do VibeHex vão para a Lixeira. Reuniões anteriores a este ajuste ficam até você escolher"),
    ("OpenCode transformation will be available after VibeHex connects to OpenCode.", "A transformação do OpenCode estará disponível depois que o VibeHex se conectar ao OpenCode."),
    ("{bluetooth} is Bluetooth, so VibeHex listens to {input}", "{bluetooth} é Bluetooth, então o VibeHex ouve {input}"),
    ("VibeHex signature", "Assinatura do VibeHex"),
    ("The tones VibeHex first shipped with", "Os sons com que o VibeHex foi lançado"),
    ("VibeHex cannot recognize the dictation shortcut or paste into other apps.", "O VibeHex não consegue reconhecer o atalho de ditado nem colar em outros apps."),
    ("VibeHex cannot record dictation until microphone access is restored.", "O VibeHex não pode gravar ditados até o acesso ao microfone ser restaurado."),
    ("VibeHex is still recording.", "O VibeHex ainda está gravando."),
    ("Enable VibeHex to dictate into any app with {}", "Ative o VibeHex para ditar em qualquer app com {}"),
    ("Could not restart VibeHex: {}", "Não foi possível reiniciar o VibeHex: {}"),
    ("VibeHex starts listening once the speech model has downloaded.", "O VibeHex começa a ouvir assim que o modelo de voz terminar de baixar."),
    ("Let VibeHex paste for you", "Deixe o VibeHex colar para você"),
    ("VibeHex needs this to type your words into your apps. Your desktop asks once, and may call it remote desktop.", "O VibeHex precisa disso para digitar suas palavras nos seus apps. Sua área de trabalho pergunta uma vez e pode chamar isso de área de trabalho remota."),
    ("VibeHex only sends the paste keystroke. It never sees your screen.", "O VibeHex só envia o atalho de colar. Ele nunca vê sua tela."),
    ("Without it, VibeHex copies your words and you press Ctrl+V.", "Sem isso, o VibeHex copia suas palavras e você pressiona Ctrl+V."),
    ("Ready the moment you press the shortcut. macOS shows the mic in use while VibeHex runs; you can change that in Settings", "Pronto assim que você pressiona o atalho. O macOS mostra o microfone em uso enquanto o VibeHex está aberto; você pode mudar isso nos Ajustes"),
    ("Let VibeHex hear you and type for you", "Deixe o VibeHex ouvir você e digitar por você"),
    ("Quit and reopen VibeHex to finish setup. macOS applies some permissions only after a restart.", "Saia e abra o VibeHex de novo para concluir a configuração. O macOS só aplica algumas permissões após reiniciar."),
    ("Reopen VibeHex", "Reabrir o VibeHex"),
    ("Restart VibeHex", "Reiniciar o VibeHex"),
    ("Turns on when VibeHex restarts", "É ativado quando o VibeHex reinicia"),
    ("VibeHex can't dictate until a speech model is installed", "O VibeHex não pode ditar até que um modelo de fala seja instalado"),
    ("Start using VibeHex", "Começar a usar o VibeHex"),
    ("Voice Filter isn't in this version of VibeHex. Uninstall its driver to remove Hex Microphone from your inputs.", "O filtro de voz não está nesta versão do VibeHex. Desinstale o driver dele para remover o Hex Microphone das suas entradas."),
    ("Finishing the speech model so VibeHex can hear you.", "Terminando o modelo de voz para o VibeHex poder ouvir você."),
    ("VibeHex didn't hear any words. Try again a little louder.", "O VibeHex não ouviu nenhuma palavra. Tente de novo um pouco mais alto."),
    ("VibeHex found speech models already on this computer.", "O VibeHex encontrou modelos de voz que já estão neste computador."),
    ("VibeHex is ready", "O VibeHex está pronto"),
    ("Getting VibeHex ready to listen…", "O VibeHex está se preparando para ouvir…"),
    ("The VibeHex shortcut didn't start.", "O atalho do VibeHex não iniciou."),
    ("Hold a key and talk. VibeHex types it wherever your cursor is.", "Segure uma tecla e fale. O VibeHex digita onde estiver o cursor."),
    ("I hereby have completed VibeHex onboarding.", "Declaro que concluí a introdução ao VibeHex."),
    ("Setting up VibeHex", "Configurando o VibeHex"),
    ("Switch on VibeHex in System Settings", "Ative o VibeHex nos Ajustes do Sistema"),
    ("That's VibeHex", "Esse é o VibeHex"),
    ("With VibeHex", "Com o VibeHex"),
    ("Without VibeHex", "Sem o VibeHex"),
    ("turn VibeHex on in System Settings", "ative o VibeHex nos Ajustes do Sistema"),
    ("turns on when VibeHex restarts", "ativa quando o VibeHex reiniciar"),
    ("VibeHex needs {}", "O VibeHex precisa de {}"),
    ("Press Caps Lock to start dictation and again to finish. VibeHex keeps Caps Lock off and uses its light while it records and transcribes", "Pressione Caps Lock para começar a ditar e de novo para terminar. O VibeHex mantém o Caps Lock desligado e usa a luz dele enquanto grava e transcreve"),
    ("An older VibeHex entry can keep this off", "Um item antigo do VibeHex pode manter isto desativado"),
    ("The VibeHex local API can return transcripts with Global mode's processing applied.", "Apps podem pedir à API local do VibeHex transcrições com o processamento do modo Global aplicado"),
    ("A model VibeHex downloads and runs itself, offline", "Um modelo que o VibeHex baixa e executa sozinho, offline"),
    ("Your own rules, added to the VibeHex prompt along with your vocabulary", "Suas próprias regras, adicionadas ao prompt do VibeHex junto com seu vocabulário"),
    ("VibeHex {version} is ready", "VibeHex {version} está pronto"),
    ("You have {running}. Restart VibeHex to install it now, or it installs when you quit", "Você tem {running}. Reinicie o VibeHex para instalar agora, ou ela será instalada ao sair"),
    ("VibeHex can't read this file", "O VibeHex não consegue ler este arquivo"),
    ("Use the VibeHex prompt", "Usar a instrução do VibeHex"),
    ("{} isn't an audio or video file VibeHex can read.", "{} não é um arquivo de áudio ou vídeo que o VibeHex consiga ler."),
    ("Your speech model, {}, isn't set to one language VibeHex writes prompts in, so they follow the app's language", "Seu modelo de fala, {}, não está em um idioma para o qual o VibeHex tem prompts, então eles seguem o idioma do app"),
    ("Not found. VibeHex uses the Codex CLI on this computer: install it with {}, then sign in once", "Não encontrado. O VibeHex usa o Codex CLI neste computador: instale com {} e entre uma vez"),
    ("Not signed in. Signing in opens your browser; VibeHex never sees your password", "Sem login. Entrar abre o navegador; o VibeHex nunca vê sua senha"),
];

#[rustfmt::skip]
pub(super) const RUSSIAN: &[(&str, &str)] = &[
    ("VibeHex picks a fast model", "VibeHex выбирает быструю модель"),
    ("Summary models show once VibeHex connects to OpenCode.", "Модели для сводок появятся, когда VibeHex подключится к OpenCode."),
    ("Start OpenCode to choose a summary model. VibeHex keeps the saved one.", "Запустите OpenCode, чтобы выбрать модель для сводок. VibeHex сохранит выбранную ранее."),
    ("Hide the VibeHex icon from the menu bar. Shortcuts keep working; open VibeHex again from Finder or Spotlight for Settings", "Скрывает значок VibeHex в строке меню. Сочетания клавиш продолжают работать; чтобы открыть Настройки, снова откройте VibeHex из Finder или Spotlight"),
    ("⌘Q closes the Settings window instead of quitting VibeHex. Dictation keeps running; quit from the menu bar icon", "⌘Q закрывает окно настроек, а не завершает VibeHex. Диктовка продолжает работать; чтобы выйти, используйте значок в строке меню"),
    ("Tell apps on this computer when a meeting's transcript is ready. They get only the meeting's ID, through the VibeHex local API", "Сообщает приложениям на этом компьютере, когда расшифровка встречи готова. Они получают только ID встречи через локальный API VibeHex"),
    ("Write it as provider/model, such as openai/gpt-5.4-mini. Until then, VibeHex tries its list of fast models", "Укажите в виде провайдер/модель, например openai/gpt-5.4-mini. До тех пор VibeHex пробует свой список быстрых моделей"),
    ("Use the § key, left of 1 on ISO keyboards, as a shortcut without a modifier. VibeHex takes § from every app while it's bound", "Использовать клавишу § (слева от 1 на клавиатурах ISO) как сочетание без модификатора. Пока она назначена, VibeHex забирает § у всех приложений"),
    ("This desktop doesn't let VibeHex put your clipboard back after a paste. Turn on Labs › Restore clipboard on GNOME to try it.", "Это окружение не позволяет VibeHex восстановить буфер обмена после вставки. Чтобы попробовать, включите Лаборатория › Восстановление буфера обмена в GNOME."),
    ("VibeHex can't listen until the speech model has downloaded", "VibeHex не может слушать, пока модель речи не загрузится"),
    ("Your desktop assigns this key itself, so VibeHex can't confirm it", "Ваше окружение рабочего стола само назначает эту клавишу, поэтому VibeHex не может это подтвердить"),
    ("Stop a recording VibeHex started on its own 30 s after the call app leaves the microphone and goes quiet", "Останавливает запись, которую VibeHex начал сам, через 30 с после того, как приложение звонка освободит микрофон и затихнет"),
    ("VibeHex stops recording in 30 s unless you keep recording.", "VibeHex остановит запись через 30 с, если вы не продолжите запись."),
    ("While dictating with Pause media: YouTube and other tabs, resumed only if VibeHex paused them", "Во время диктовки с «Приостанавливать медиа»: YouTube и другие вкладки, возобновляются, только если их приостановил VibeHex"),
    ("{model} is missing, so VibeHex uses {fallback} until you import it again or choose another model", "{model} не найдена, поэтому VibeHex использует {fallback}, пока вы не импортируете её снова или не выберете другую модель"),
    ("{model} changed since it was added, so VibeHex uses {fallback} until you check it again in the speech model picker or choose another model", "{model} изменилась после добавления, поэтому VibeHex использует {fallback}, пока вы не проверите её снова в списке моделей речи или не выберете другую модель"),
    ("Not tested by VibeHex", "Не проверена VibeHex"),
    ("On this computer, not tested by VibeHex", "На этом компьютере, не проверена VibeHex"),
    ("Experimental: VibeHex has run it only on Linux (Vulkan) so far.", "Экспериментально: пока VibeHex запускал её только в Linux (Vulkan)."),
    ("Published conversion benchmarks; not measured in VibeHex", "Опубликованные тесты конвертации; в VibeHex не измерялись"),
    ("{model} needs Labs › Parakeet Ultra, so VibeHex uses {fallback} until you turn it on or choose another model", "{model} требует Лаборатория › Parakeet Ultra, поэтому VibeHex использует {fallback}, пока вы её не включите или не выберете другую модель"),
    ("Off. VibeHex records nothing and keeps no shortcut until you turn this on.", "Выключено. VibeHex ничего не записывает и не занимает сочетание клавиш, пока вы это не включите."),
    ("The model VibeHex dictates with, with a time for every word", "Модель, которой диктует VibeHex, с временем для каждого слова"),
    ("VibeHex leaves the Annotate shortcut to Scrawl until you switch", "VibeHex оставляет сочетание Annotate за Scrawl, пока вы не переключитесь"),
    ("Use VibeHex for annotations", "Использовать VibeHex для аннотаций"),
    ("Lets recordings, annotations included, show the VibeHex capsule, ink, and HUD, for demos", "Позволяет записям, включая аннотации, показывать капсулу, рисунки и HUD VibeHex — для демонстраций"),
    ("Change the keys your desktop gave VibeHex", "Измените клавиши, которые окружение рабочего стола назначило VibeHex"),
    ("Added to GNOME Settings › Keyboard › Custom Shortcuts while VibeHex is enabled. GNOME runs them on the key press, so your dictation shortcut toggles dictation. Change the keys there.", "Добавлены в Настройки GNOME › Клавиатура › Особые комбинации клавиш, пока VibeHex включён. GNOME запускает их при нажатии клавиши, поэтому сочетание для диктовки переключает диктовку. Клавиши можно изменить там."),
    ("GNOME uses {} itself, so it never reaches VibeHex. Record another shortcut, or change it in GNOME Settings › Keyboard › Keyboard Shortcuts.", "GNOME сам использует {}, поэтому это сочетание не доходит до VibeHex. Запишите другое сочетание или измените его в Настройки GNOME › Клавиатура › Комбинации клавиш."),
    ("VibeHex uses the CPU because the graphics driver's Vulkan is missing or too old. Update the GPU driver to use the GPU.", "VibeHex использует CPU, потому что в графическом драйвере нет Vulkan или его версия слишком старая. Обновите драйвер видеокарты, чтобы использовать GPU."),
    ("Tell your agent how VibeHex should write", "Расскажите агенту, как VibeHex должен писать"),
    ("VibeHex asks again whenever the config changes. Until you allow it, they paste your words unchanged", "VibeHex спрашивает снова при каждом изменении конфигурации. Пока вы не разрешите, они вставляют ваши слова без изменений"),
    ("No steps yet. VibeHex pastes your words as it heard them", "Шагов пока нет. VibeHex вставляет ваши слова так, как услышал"),
    ("What VibeHex pastes", "Что вставит VibeHex"),
    ("Pastes your words as VibeHex heard them", "Вставляет ваши слова так, как их услышал VibeHex"),
    ("Paste your words as VibeHex heard them, skipping every step", "Вставляет ваши слова так, как их услышал VibeHex, пропуская все шаги"),
    ("Lets VibeHex hear whether a tab is playing", "Позволяет VibeHex слышать, воспроизводит ли вкладка звук"),
    ("Hex Microphone is the system default, so VibeHex listens to {}", "Hex Microphone выбран в системе по умолчанию, поэтому VibeHex слушает {}"),
    ("{built_in} is off while the lid is closed, so VibeHex listens to {input}", "{built_in} выключен, пока крышка закрыта, поэтому VibeHex слушает {input}"),
    ("VibeHex could not apply this change.", "VibeHex не удалось применить это изменение."),
    ("Let VibeHex hear you", "Позвольте VibeHex вас слышать"),
    ("VibeHex could not load the OpenCode model catalog.", "VibeHex не удалось загрузить каталог моделей OpenCode."),
    ("Allow VibeHex under Privacy & Security › Calendars to get meeting reminders", "Разрешите VibeHex в разделе Конфиденциальность и безопасность › Календари, чтобы получать напоминания о встречах"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash", "Звук, расшифровки, заметки и копия Markdown от VibeHex перемещаются в Корзину"),
    ("Audio, transcripts, notes, and the Markdown copy VibeHex wrote move to the Trash. Meetings from before this setting stay until you choose", "Звук, расшифровки, заметки и копия Markdown от VibeHex перемещаются в Корзину. Встречи, записанные до этой настройки, остаются, пока вы не сделаете выбор"),
    ("OpenCode transformation will be available after VibeHex connects to OpenCode.", "Преобразование OpenCode станет доступно, когда VibeHex подключится к OpenCode."),
    ("{bluetooth} is Bluetooth, so VibeHex listens to {input}", "{bluetooth} подключён по Bluetooth, поэтому VibeHex слушает {input}"),
    ("VibeHex signature", "Фирменные VibeHex"),
    ("The tones VibeHex first shipped with", "Звуки, с которыми VibeHex вышел впервые"),
    ("VibeHex cannot recognize the dictation shortcut or paste into other apps.", "VibeHex не может распознать сочетание для диктовки или вставлять текст в другие приложения."),
    ("VibeHex cannot record dictation until microphone access is restored.", "VibeHex не может записывать диктовку, пока доступ к микрофону не восстановлен."),
    ("VibeHex is still recording.", "VibeHex всё ещё записывает."),
    ("Enable VibeHex to dictate into any app with {}", "Включите VibeHex, чтобы диктовать в любое приложение с помощью {}"),
    ("Could not restart VibeHex: {}", "Не удалось перезапустить VibeHex: {}"),
    ("VibeHex starts listening once the speech model has downloaded.", "VibeHex начнёт слушать, как только загрузится модель речи."),
    ("Let VibeHex paste for you", "Позвольте VibeHex вставлять за вас"),
    ("VibeHex needs this to type your words into your apps. Your desktop asks once, and may call it remote desktop.", "VibeHex нужно это, чтобы вводить ваши слова в приложения. Окружение рабочего стола спросит один раз и может назвать это удалённым рабочим столом."),
    ("VibeHex only sends the paste keystroke. It never sees your screen.", "VibeHex только отправляет сочетание вставки. Он никогда не видит ваш экран."),
    ("Without it, VibeHex copies your words and you press Ctrl+V.", "Без этого VibeHex копирует ваши слова, а вы нажимаете Ctrl+V."),
    ("Ready the moment you press the shortcut. macOS shows the mic in use while VibeHex runs; you can change that in Settings", "Готов, как только вы нажмёте сочетание клавиш. Пока VibeHex работает, macOS показывает, что микрофон используется; это можно изменить в Настройках"),
    ("Let VibeHex hear you and type for you", "Позвольте VibeHex слышать вас и печатать за вас"),
    ("Quit and reopen VibeHex to finish setup. macOS applies some permissions only after a restart.", "Закройте и снова откройте VibeHex, чтобы завершить настройку. macOS применяет некоторые разрешения только после перезапуска."),
    ("Reopen VibeHex", "Открыть VibeHex снова"),
    ("Restart VibeHex", "Перезапустить VibeHex"),
    ("Turns on when VibeHex restarts", "Включится после перезапуска VibeHex"),
    ("VibeHex can't dictate until a speech model is installed", "VibeHex не может диктовать, пока не установлена модель речи"),
    ("Start using VibeHex", "Начать пользоваться VibeHex"),
    ("Voice Filter isn't in this version of VibeHex. Uninstall its driver to remove Hex Microphone from your inputs.", "В этой версии VibeHex нет голосового фильтра. Удалите его драйвер, чтобы убрать Hex Microphone из списка входов."),
    ("Finishing the speech model so VibeHex can hear you.", "Завершаем модель речи, чтобы VibeHex мог вас слышать."),
    ("VibeHex didn't hear any words. Try again a little louder.", "VibeHex не услышал ни слова. Попробуйте ещё раз чуть громче."),
    ("VibeHex found speech models already on this computer.", "VibeHex нашёл модели речи, которые уже есть на этом компьютере."),
    ("VibeHex is ready", "VibeHex готов"),
    ("Getting VibeHex ready to listen…", "VibeHex готовится слушать…"),
    ("The VibeHex shortcut didn't start.", "Сочетание клавиш VibeHex не запустилось."),
    ("Hold a key and talk. VibeHex types it wherever your cursor is.", "Удерживайте клавишу и говорите. VibeHex напечатает это там, где курсор."),
    ("I hereby have completed VibeHex onboarding.", "Настоящим подтверждаю, что прошёл знакомство с VibeHex."),
    ("Setting up VibeHex", "Настройка VibeHex"),
    ("Switch on VibeHex in System Settings", "Включите VibeHex в Системных настройках"),
    ("That's VibeHex", "Это VibeHex"),
    ("With VibeHex", "С VibeHex"),
    ("Without VibeHex", "Без VibeHex"),
    ("turn VibeHex on in System Settings", "включите VibeHex в Системных настройках"),
    ("turns on when VibeHex restarts", "включится после перезапуска VibeHex"),
    ("VibeHex needs {}", "VibeHex нужно: {}"),
    ("Press Caps Lock to start dictation and again to finish. VibeHex keeps Caps Lock off and uses its light while it records and transcribes", "Нажмите Caps Lock, чтобы начать диктовку, и ещё раз, чтобы закончить. VibeHex держит Caps Lock выключенным и использует его индикатор во время записи и расшифровки"),
    ("An older VibeHex entry can keep this off", "Старая запись VibeHex может мешать включению"),
    ("The VibeHex local API can return transcripts with Global mode's processing applied.", "Приложения могут запрашивать у локального API VibeHex транскрипты с обработкой глобального режима"),
    ("A model VibeHex downloads and runs itself, offline", "Модель, которую VibeHex сам скачивает и запускает без интернета"),
    ("Your own rules, added to the VibeHex prompt along with your vocabulary", "Ваши правила, которые добавляются к промпту VibeHex вместе со словарём"),
    ("VibeHex {version} is ready", "VibeHex {version} готов"),
    ("You have {running}. Restart VibeHex to install it now, or it installs when you quit", "У вас {running}. Перезапустите VibeHex, чтобы установить обновление сейчас, или оно установится при выходе"),
    ("VibeHex can't read this file", "VibeHex не может прочитать этот файл"),
    ("Use the VibeHex prompt", "Использовать промпт VibeHex"),
    ("{} isn't an audio or video file VibeHex can read.", "VibeHex не может прочитать {}: это не аудио- или видеофайл."),
    ("Your speech model, {}, isn't set to one language VibeHex writes prompts in, so they follow the app's language", "Ваша модель речи, {}, не настроена на язык, для которого у VibeHex есть промпты, поэтому они следуют языку приложения"),
    ("Not found. VibeHex uses the Codex CLI on this computer: install it with {}, then sign in once", "Не найден. VibeHex использует Codex CLI на этом компьютере: установите его командой {} и один раз войдите"),
    ("Not signed in. Signing in opens your browser; VibeHex never sees your password", "Вход не выполнен. Для входа откроется браузер; VibeHex не видит ваш пароль"),
];

#[cfg(test)]
pub(super) mod tests {
    use std::path::{Path, PathBuf};

    /// Names that keep "Hex" inside a string that otherwise says VibeHex:
    /// the audio device, the meetings folder, and saved captures.
    const KEPT_NAMES: &[&str] = &[
        "VibeHex",
        "Hex Microphone",
        "Hex Meetings",
        "Hex Screenshot",
        "Hex Recording",
    ];

    /// Whole strings where "Hex" is Hex itself: the credit in About, and
    /// the Windows messages that send to Hex's installer, which VibeHex has
    /// none of its own to name instead.
    const KEPT_STRINGS: &[&str] = &[
        "VibeHex is a fork of Hex, the open-source dictation app. It adds many useful features of its own, and its code is open too.",
        "This copy of Hex can't update itself because the Hex installer didn't install it. Install Hex from anoma.ly/hex to get updates in the app.",
        "This copy of Hex can't update itself because it's installed outside your user folder. Reinstall Hex from anoma.ly/hex to get updates in the app.",
        "This copy of Hex can't update itself because its install record is from another installer. Reinstall Hex from anoma.ly/hex to get updates in the app.",
    ];

    /// Whether `text` calls the app "Hex" rather than VibeHex.
    fn names_hex(text: &str) -> bool {
        if KEPT_STRINGS.contains(&text) {
            return false;
        }
        let mut rest = text.to_owned();
        for name in KEPT_NAMES {
            rest = rest.replace(name, "");
        }
        rest.match_indices("Hex").any(|(start, _)| {
            let before = rest[..start].chars().next_back();
            let after = rest[start + 3..].chars().next();
            !before.is_some_and(char::is_alphabetic) && !after.is_some_and(char::is_lowercase)
        })
    }

    /// Every Rust source outside the translation tables, without its comments.
    pub(in super::super) fn sources() -> Vec<(PathBuf, String)> {
        fn walk(folder: &Path, found: &mut Vec<(PathBuf, String)>) {
            for entry in std::fs::read_dir(folder)
                .expect("src is readable")
                .flatten()
            {
                let path = entry.path();
                if path.is_dir() {
                    if !path.ends_with("desktop/i18n") {
                        walk(&path, found);
                    }
                } else if path.extension().is_some_and(|extension| extension == "rs")
                    && !path.ends_with("desktop/i18n.rs")
                {
                    let source = std::fs::read_to_string(&path).expect("a source is UTF-8");
                    let code: Vec<&str> = source
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("//"))
                        .collect();
                    found.push((path, code.join("\n")));
                }
            }
        }
        let mut found = Vec::new();
        walk(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut found,
        );
        assert!(found.len() > 300, "only {} sources found", found.len());
        found
    }

    /// The string literals passed to `tr`, `tr_fill`, and `tr_named`.
    pub(in super::super) fn translated_literals(source: &str) -> Vec<String> {
        let mut literals = Vec::new();
        for call in ["tr(", "tr_fill(", "tr_named("] {
            for (start, _) in source.match_indices(call) {
                let in_a_longer_name = source[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|character| character.is_alphanumeric() || character == '_');
                let rest = source[start + call.len()..].trim_start();
                let Some(rest) = rest.strip_prefix('"').filter(|_| !in_a_longer_name) else {
                    continue;
                };
                let mut literal = String::new();
                let mut characters = rest.chars();
                while let Some(character) = characters.next() {
                    match character {
                        '"' => break,
                        '\\' => literal.extend(characters.next()),
                        other => literal.push(other),
                    }
                }
                literals.push(literal);
            }
        }
        literals
    }

    #[test]
    fn the_check_tells_hex_from_vibehex() {
        assert!(names_hex("Restart Hex"));
        assert!(names_hex("Hex's prompt"));
        assert!(names_hex("Hex Microphone is the default, so Hex listens"));
        assert!(!names_hex("Restart VibeHex"));
        assert!(!names_hex("Choose Hex Microphone as your input"));
        assert!(!names_hex("Hexadecimal"));
        assert!(!names_hex(KEPT_STRINGS[0]));
    }

    /// A string the window translates calls the app VibeHex. A new string
    /// from Hex that says "Hex" fails here until it has a row above.
    #[test]
    fn no_translated_string_calls_the_app_hex() {
        let mut checked = 0;
        for (path, source) in sources() {
            for literal in translated_literals(&source) {
                assert!(
                    !names_hex(&literal),
                    "{} shows {literal:?}: say VibeHex and add its rows to vibehex/naming.rs",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} translated strings found");
    }

    /// The same for a string translated away from where it is written (a
    /// constant, a sound set's name): Hex's rows that say "Hex" stay in the
    /// tables unused, so none of them is written out in a source.
    #[test]
    fn no_source_writes_out_a_string_that_calls_the_app_hex() {
        let sources = sources();
        for (english, _) in super::super::super::all_translations() {
            if !names_hex(english) {
                continue;
            }
            let written = format!("\"{}\"", english.replace('\\', "\\\\").replace('"', "\\\""));
            for (path, source) in &sources {
                assert!(
                    !source.contains(&written),
                    "{} writes {english:?}: say VibeHex and add its rows to vibehex/naming.rs",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn every_renamed_string_says_vibehex_in_every_language() {
        let french: std::collections::HashMap<_, _> = super::FRENCH.iter().copied().collect();
        let portuguese: std::collections::HashMap<_, _> =
            super::PORTUGUESE.iter().copied().collect();
        let russian: std::collections::HashMap<_, _> = super::RUSSIAN.iter().copied().collect();
        for (english, translated) in super::TRANSLATIONS {
            assert!(!names_hex(english), "{english:?}");
            let others = [french[english], portuguese[english], russian[english]];
            for translation in translated.iter().chain(&others) {
                assert!(
                    !names_hex(translation),
                    "{english:?} became {translation:?}"
                );
            }
        }
    }
}
