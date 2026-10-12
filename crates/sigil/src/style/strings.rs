use std::borrow::Cow;
use std::fmt::Display;

macro_rules! string_keys {
    ($($key:ident $name:literal $english:literal,)*) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        #[non_exhaustive]
        pub enum StringKey {
            $($key,)*
        }

        impl StringKey {
            pub const ALL: [Self; 37] = [$(Self::$key,)*];
        }

        const NAMES: [(&str, StringKey); StringKey::ALL.len()] = [$(($name, StringKey::$key),)*];

        const ENGLISH: [&str; StringKey::ALL.len()] = [$($english,)*];
    };
}

string_keys! {
    Close "close" "Close",
    Cancel "cancel" "Cancel",
    Ok "ok" "OK",
    Search "search" "Search",
    Dialog "dialog" "Dialog",
    Loading "loading" "Loading",
    RequiredMark "required_mark" "*",
    Empty "empty" "Empty",
    Avatar "avatar" "avatar",
    Chart "chart" "Chart",
    NoData "no_data" "No data",
    Series "series" "Series {0}",
    ChartSummary "chart_summary" "{0}: {1} values from {2} to {3}",
    Sparkline "sparkline" "Sparkline",
    SparklineSummary "sparkline_summary" "{0} to {1}, last {2}",
    Heatmap "heatmap" "Heatmap",
    HeatmapGrid "heatmap_grid" "{0} by {1} grid",
    CodeEditor "code_editor" "Code editor",
    CommandMenu "command_menu" "Command Menu",
    StatusBar "status_bar" "Status bar",
    MenuBar "menu_bar" "Menu bar",
    ContextMenu "context_menu" "Context menu",
    KeyboardShortcuts "keyboard_shortcuts" "Keyboard shortcuts",
    ResizeSidebar "resize_sidebar" "Resize sidebar",
    ResizePanes "resize_panes" "Resize panes",
    TableMenu "table_menu" "Table menu",
    SortAscending "sort_ascending" "Sort ascending",
    SortDescending "sort_descending" "Sort descending",
    HideColumn "hide_column" "Hide column",
    Autosize "autosize" "Autosize",
    MoveLeft "move_left" "Move left",
    MoveRight "move_right" "Move right",
    ShowAllColumns "show_all_columns" "Show all columns",
    Select "select" "Select",
    SelectAllRows "select_all_rows" "Select all visible rows",
    SelectRow "select_row" "Select row {0}",
    SavingOnExit "saving_on_exit" "Saving before closing",
}

impl StringKey {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, key)| *key)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Strings {
    values: [Cow<'static, str>; StringKey::ALL.len()],
}

impl Default for Strings {
    fn default() -> Self {
        Self::english()
    }
}

impl Strings {
    #[must_use]
    pub fn english() -> Self {
        Self {
            values: ENGLISH.map(Cow::Borrowed),
        }
    }

    #[must_use]
    pub fn from_table(table: &[(StringKey, &'static str)]) -> Self {
        table.iter().fold(Self::english(), |strings, (key, value)| {
            strings.with(*key, *value)
        })
    }

    #[must_use]
    pub fn with(mut self, key: StringKey, value: impl Into<Cow<'static, str>>) -> Self {
        self.set(key, value);
        self
    }

    pub fn set(&mut self, key: StringKey, value: impl Into<Cow<'static, str>>) {
        if let Some(slot) = self.values.get_mut(key as usize) {
            *slot = value.into();
        }
    }

    #[must_use]
    pub fn get(&self, key: StringKey) -> &str {
        self.values.get(key as usize).map_or("", AsRef::as_ref)
    }

    #[must_use]
    pub fn fill(&self, key: StringKey, args: &[&dyn Display]) -> String {
        args.iter()
            .enumerate()
            .fold(self.get(key).to_owned(), |text, (index, arg)| {
                text.replace(&format!("{{{index}}}"), &arg.to_string())
            })
    }
}
