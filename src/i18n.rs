use serde::{Deserialize, Serialize};

/// Interface language. `Settings::lang` stores an explicit user choice
/// (`Some`), or `None` to mean "follow the system" — see
/// [`Lang::detect_system`], which is what actually picks a `Lang` in that
/// case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    /// The editor only ships two languages, so this is a simple rule
    /// rather than real locale negotiation: Russian if (and only if) the
    /// OS-reported locale is Russian, English for absolutely everything
    /// else — including a locale we fail to read at all, per the user's
    /// request ("если стоит английский или любой другой, кроме русского,
    /// выбираем английский").
    pub fn detect_system() -> Self {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_lowercase().starts_with("ru") => Lang::Ru,
            _ => Lang::En,
        }
    }

    /// Label shown for this language in the language picker itself —
    /// always in that language's own name, not translated.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
        }
    }
}

/// All UI strings, looked up by a short dotted key. Kept as one match
/// instead of pulling in a full localization framework (fluent, gettext,
/// ...) since the app only has two languages and a few dozen strings.
///
/// Falls back to the key itself for anything unmapped, so a missing
/// translation shows up as an obviously-wrong string in the UI rather than
/// silently rendering blank.
pub fn t(lang: Lang, key: &'static str) -> &'static str {
    use Lang::*;
    match (lang, key) {
        // ---- settings pages ----
        (Ru, "settings.appearance") => "Внешний вид",
        (En, "settings.appearance") => "Appearance",
        (Ru, "settings.font") => "Шрифт",
        (En, "settings.font") => "Font",
        (Ru, "settings.editor") => "Редактор",
        (En, "settings.editor") => "Editor",
        (Ru, "settings.advanced") => "Дополнительно",
        (En, "settings.advanced") => "Advanced",

        // ---- file dialogs ----
        (Ru, "dialog.open_file") => "Открыть файл",
        (En, "dialog.open_file") => "Open file",
        (Ru, "dialog.open_folder") => "Открыть папку",
        (En, "dialog.open_folder") => "Open folder",
        (Ru, "dialog.save_as") => "Сохранить как",
        (En, "dialog.save_as") => "Save as",

        // ---- status messages ----
        (Ru, "status.saved") => "Сохранено",
        (En, "status.saved") => "Saved",

        // ---- toolbar ----
        (Ru, "toolbar.open") => "Открыть",
        (En, "toolbar.open") => "Open",
        (Ru, "toolbar.folder") => "Папка",
        (En, "toolbar.folder") => "Folder",
        (Ru, "toolbar.save") => "Сохранить",
        (En, "toolbar.save") => "Save",
        (Ru, "toolbar.search") => "Поиск",
        (En, "toolbar.search") => "Search",
        (Ru, "toolbar.tree") => "Дерево",
        (En, "toolbar.tree") => "Tree",
        (Ru, "toolbar.new_tab_tooltip") => "Новая вкладка (Ctrl+N)",
        (En, "toolbar.new_tab_tooltip") => "New tab (Ctrl+N)",
        (Ru, "toolbar.settings") => "Настройки",
        (En, "toolbar.settings") => "Settings",

        // ---- title bar "File" menu / activity bar ----
        (Ru, "menu.file") => "Файл",
        (En, "menu.file") => "File",
        (Ru, "menu.new_tab") => "Новая вкладка",
        (En, "menu.new_tab") => "New Tab",
        (Ru, "menu.exit") => "Выход",
        (En, "menu.exit") => "Exit",
        (Ru, "activity.explorer") => "Проводник",
        (En, "activity.explorer") => "Explorer",
        (Ru, "file_tree.empty_hint") => "Вы ещё не открыли папку.",
        (En, "file_tree.empty_hint") => "You have not yet opened a folder.",

        // ---- sidebar / tabs ----
        (Ru, "sidebar.files_header") => "ФАЙЛЫ",
        (En, "sidebar.files_header") => "FILES",
        (Ru, "editor.no_open_files") => "Нет открытых файлов",
        (En, "editor.no_open_files") => "No open files",
        (Ru, "editor.loading_file") => "Загрузка файла…",
        (En, "editor.loading_file") => "Loading file…",
        (Ru, "file_tree.no_folder") => "Папка не выбрана",
        (En, "file_tree.no_folder") => "No folder selected",

        // ---- search window ----
        (Ru, "search.case_sensitive") => "Учитывать регистр",
        (En, "search.case_sensitive") => "Case sensitive",
        (Ru, "search.no_matches") => "Нет совпадений",
        (En, "search.no_matches") => "No matches",

        // ---- common ----
        (Ru, "common.close") => "Закрыть",
        (En, "common.close") => "Close",
        (Ru, "common.cancel") => "Отмена",
        (En, "common.cancel") => "Cancel",

        // ---- settings: appearance / font / editor / advanced ----
        (Ru, "settings.theme") => "Тема",
        (En, "settings.theme") => "Theme",
        (Ru, "settings.builtin_mono") => "Встроенный (моно)",
        (En, "settings.builtin_mono") => "Built-in (mono)",
        (Ru, "settings.font_family") => "Семейство шрифта",
        (En, "settings.font_family") => "Font family",
        (Ru, "settings.scanning_fonts") => "Сканирование системных шрифтов…",
        (En, "settings.scanning_fonts") => "Scanning system fonts…",
        (Ru, "settings.font_size") => "Размер шрифта",
        (En, "settings.font_size") => "Font size",
        (Ru, "settings.line_numbers") => "Нумерация строк",
        (En, "settings.line_numbers") => "Line numbers",
        (Ru, "settings.syntax_highlight") => "Подсветка синтаксиса",
        (En, "settings.syntax_highlight") => "Syntax highlighting",
        (Ru, "settings.autoclose") => "Автозакрытие скобок и кавычек",
        (En, "settings.autoclose") => "Auto-close brackets and quotes",
        (Ru, "settings.word_wrap") => "Перенос строк",
        (En, "settings.word_wrap") => "Word wrap",
        (Ru, "settings.show_tree") => "Показывать дерево файлов",
        (En, "settings.show_tree") => "Show file tree",
        (Ru, "settings.tab_width") => "Ширина табуляции",
        (En, "settings.tab_width") => "Tab width",
        (Ru, "settings.css_theme") => "CSS-тема",
        (En, "settings.css_theme") => "CSS theme",
        (Ru, "settings.css_theme_desc") => {
            "Свой файл темы (переменные --bg, --fg, --accent, --font-family, ...)."
        }
        (En, "settings.css_theme_desc") => {
            "Custom theme file (variables --bg, --fg, --accent, --font-family, ...)."
        }
        (Ru, "settings.browse") => "Обзор…",
        (En, "settings.browse") => "Browse…",
        (Ru, "settings.create_example_css") => "Создать пример theme.css",
        (En, "settings.create_example_css") => "Create example theme.css",
        (Ru, "settings.apply_as_theme") => "Применить как тему",
        (En, "settings.apply_as_theme") => "Apply as theme",
        (Ru, "settings.reread_css") => "Перечитать CSS",
        (En, "settings.reread_css") => "Re-read CSS",
        (Ru, "settings.language") => "Язык",
        (En, "settings.language") => "Language",
        (Ru, "settings.language_system") => "Как в системе",
        (En, "settings.language_system") => "Follow system",

        // ---- close-confirm modal ----
        (Ru, "close_confirm.title") => "Несохранённые изменения",
        (En, "close_confirm.title") => "Unsaved changes",
        (Ru, "close_confirm.dont_save") => "Не сохранять",
        (En, "close_confirm.dont_save") => "Don't save",

        // ---- theme names ----
        (Ru, "theme.light") => "Светлая",
        (En, "theme.light") => "Light",
        (Ru, "theme.dark") => "Тёмная",
        (En, "theme.dark") => "Dark",
        (Ru, "theme.custom_css") => "Своя (CSS)",
        (En, "theme.custom_css") => "Custom (CSS)",

        // ---- text-selection context menu ----
        (Ru, "context_menu.cut") => "Вырезать",
        (En, "context_menu.cut") => "Cut",
        (Ru, "context_menu.copy") => "Копировать",
        (En, "context_menu.copy") => "Copy",
        (Ru, "context_menu.paste") => "Вставить",
        (En, "context_menu.paste") => "Paste",
        (Ru, "context_menu.select_all") => "Выделить всё",
        (En, "context_menu.select_all") => "Select all",

        // ---- Settings editor (VS Code style) ----
        (Ru, "set.search_hint") => "Поиск параметров",
        (En, "set.search_hint") => "Search settings",
        (Ru, "set.user") => "Пользователь",
        (En, "set.user") => "User",
        (Ru, "set.toc.common") => "Часто используемые",
        (En, "set.toc.common") => "Commonly Used",
        (Ru, "set.toc.editor") => "Текстовый редактор",
        (En, "set.toc.editor") => "Text Editor",
        (Ru, "set.toc.workbench") => "Рабочая область",
        (En, "set.toc.workbench") => "Workbench",
        (Ru, "set.pre.editor") => "Редактор",
        (En, "set.pre.editor") => "Editor",
        (Ru, "set.pre.workbench") => "Рабочая область",
        (En, "set.pre.workbench") => "Workbench",
        (Ru, "set.reset") => "Сбросить параметр",
        (En, "set.reset") => "Reset Setting",
        (Ru, "set.colorTheme.name") => "Цветовая тема",
        (En, "set.colorTheme.name") => "Color Theme",
        (Ru, "set.colorTheme.desc") => "Определяет цветовую тему, используемую в рабочей области.",
        (En, "set.colorTheme.desc") => "Specifies the color theme used in the workbench.",
        (Ru, "set.customTheme.name") => "Файл своей темы",
        (En, "set.customTheme.name") => "Custom Theme File",
        (Ru, "set.customTheme.desc") => "Путь к CSS-файлу с переменными темы (--bg, --fg, --accent, --font-family, ...) для темы «Своя (CSS)».",
        (En, "set.customTheme.desc") => "Path to a CSS file with theme variables (--bg, --fg, --accent, --font-family, ...) used by the Custom (CSS) theme.",
        (Ru, "set.sideBar.name") => "Боковая панель",
        (En, "set.sideBar.name") => "Side Bar Visible",
        (Ru, "set.sideBar.desc") => "Управляет видимостью боковой панели «Проводник».",
        (En, "set.sideBar.desc") => "Controls the visibility of the Explorer side bar.",
        (Ru, "set.language.name") => "Язык интерфейса",
        (En, "set.language.name") => "Display Language",
        (Ru, "set.language.desc") => "Определяет язык интерфейса. «Как в системе» выбирает русский, если язык системы русский, иначе английский.",
        (En, "set.language.desc") => "Controls the language of the user interface. Follow system picks Russian if the OS language is Russian, English otherwise.",
        (Ru, "set.fontFamily.name") => "Семейство шрифтов",
        (En, "set.fontFamily.name") => "Font Family",
        (Ru, "set.fontFamily.desc") => "Определяет семейство шрифтов.",
        (En, "set.fontFamily.desc") => "Controls the font family.",
        (Ru, "set.fontSize.name") => "Размер шрифта",
        (En, "set.fontSize.name") => "Font Size",
        (Ru, "set.fontSize.desc") => "Определяет размер шрифта в пикселях.",
        (En, "set.fontSize.desc") => "Controls the font size in pixels.",
        (Ru, "set.lineNumbers.name") => "Номера строк",
        (En, "set.lineNumbers.name") => "Line Numbers",
        (Ru, "set.lineNumbers.desc") => "Управляет отображением номеров строк.",
        (En, "set.lineNumbers.desc") => "Controls the display of line numbers.",
        (Ru, "set.wordWrap.name") => "Перенос по словам",
        (En, "set.wordWrap.name") => "Word Wrap",
        (Ru, "set.wordWrap.desc") => "Управляет переносом длинных строк.",
        (En, "set.wordWrap.desc") => "Controls how lines should wrap.",
        (Ru, "set.tabSize.name") => "Размер табуляции",
        (En, "set.tabSize.name") => "Tab Size",
        (Ru, "set.tabSize.desc") => "Количество пробелов, равное одной табуляции.",
        (En, "set.tabSize.desc") => "The number of spaces a tab is equal to.",
        (Ru, "set.autoClose.name") => "Автозакрытие скобок",
        (En, "set.autoClose.name") => "Auto Closing Brackets",
        (Ru, "set.autoClose.desc") => "Определяет, должен ли редактор автоматически закрывать скобки и кавычки.",
        (En, "set.autoClose.desc") => "Controls whether the editor should automatically close brackets and quotes.",
        (Ru, "set.syntax.name") => "Подсветка синтаксиса",
        (En, "set.syntax.name") => "Syntax Highlighting",
        (Ru, "set.syntax.desc") => "Определяет, раскрашивается ли код в соответствии с его языком.",
        (En, "set.syntax.desc") => "Controls whether code is colored according to its language.",

        (_, other) => other,
    }
}

// ---- strings that need runtime-formatted arguments (line/column, error
// details, file names, ...) — kept as small functions rather than crammed
// into `t`, since that only returns `&'static str`.

pub fn open_file_error(lang: Lang, e: &str) -> String {
    match lang {
        Lang::Ru => format!("Не удалось открыть файл: {e}"),
        Lang::En => format!("Failed to open file: {e}"),
    }
}

pub fn save_error(lang: Lang, e: &str) -> String {
    match lang {
        Lang::Ru => format!("Ошибка сохранения: {e}"),
        Lang::En => format!("Failed to save: {e}"),
    }
}

pub fn could_not_read_file(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "не удалось прочитать файл",
        Lang::En => "could not read file",
    }
}

pub fn theme_file_not_selected(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Файл темы не выбран — используется тёмная тема по умолчанию.",
        Lang::En => "No theme file selected — using the default dark theme.",
    }
}

pub fn css_read_failed(lang: Lang, path: &str, e: &str) -> String {
    match lang {
        Lang::Ru => format!("Не удалось прочитать {path}: {e}"),
        Lang::En => format!("Failed to read {path}: {e}"),
    }
}

pub fn font_not_found(lang: Lang, family: &str) -> String {
    match lang {
        Lang::Ru => format!("Шрифт «{family}» не найден"),
        Lang::En => format!("Font \"{family}\" not found"),
    }
}

pub fn line_col(lang: Lang, line: usize, col: usize) -> String {
    match lang {
        Lang::Ru => format!("Строка {line}, Столбец {col}"),
        Lang::En => format!("Line {line}, Column {col}"),
    }
}

pub fn save_changes_prompt(lang: Lang, title: &str) -> String {
    match lang {
        Lang::Ru => format!("Сохранить изменения в «{title}»?"),
        Lang::En => format!("Save changes to \"{title}\"?"),
    }
}

pub fn untitled(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Ru => format!("Без имени {n}"),
        Lang::En => format!("Untitled {n}"),
    }
}

pub fn untitled_plain(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Без имени",
        Lang::En => "Untitled",
    }
}

pub fn loading_title(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Ru => format!("{name} (загрузка…)"),
        Lang::En => format!("{name} (loading…)"),
    }
}

pub fn settings_found(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Ru => format!("Найдено параметров: {n}"),
        Lang::En if n == 1 => "1 Setting Found".to_string(),
        Lang::En => format!("{n} Settings Found"),
    }
}
