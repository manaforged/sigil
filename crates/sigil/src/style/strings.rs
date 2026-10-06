use std::borrow::Cow;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum StringKey {
    Close,
    Cancel,
    Ok,
    Search,
    Dialog,
    Loading,
    RequiredMark,
    Empty,
    Avatar,
    Chart,
    NoData,
    Series,
    ChartSummary,
    Sparkline,
    SparklineSummary,
    Heatmap,
    HeatmapGrid,
    CodeEditor,
    CommandMenu,
    StatusBar,
    MenuBar,
    ContextMenu,
    KeyboardShortcuts,
    ResizeSidebar,
    ResizePanes,
    TableMenu,
    SortAscending,
    SortDescending,
    HideColumn,
    Autosize,
    MoveLeft,
    MoveRight,
    ShowAllColumns,
    Select,
    SelectAllRows,
    SelectRow,
}

impl StringKey {
    pub const ALL: [Self; 36] = [
        Self::Close,
        Self::Cancel,
        Self::Ok,
        Self::Search,
        Self::Dialog,
        Self::Loading,
        Self::RequiredMark,
        Self::Empty,
        Self::Avatar,
        Self::Chart,
        Self::NoData,
        Self::Series,
        Self::ChartSummary,
        Self::Sparkline,
        Self::SparklineSummary,
        Self::Heatmap,
        Self::HeatmapGrid,
        Self::CodeEditor,
        Self::CommandMenu,
        Self::StatusBar,
        Self::MenuBar,
        Self::ContextMenu,
        Self::KeyboardShortcuts,
        Self::ResizeSidebar,
        Self::ResizePanes,
        Self::TableMenu,
        Self::SortAscending,
        Self::SortDescending,
        Self::HideColumn,
        Self::Autosize,
        Self::MoveLeft,
        Self::MoveRight,
        Self::ShowAllColumns,
        Self::Select,
        Self::SelectAllRows,
        Self::SelectRow,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, key)| *key)
    }
}

const NAMES: [(&str, StringKey); StringKey::ALL.len()] = [
    ("close", StringKey::Close),
    ("cancel", StringKey::Cancel),
    ("ok", StringKey::Ok),
    ("search", StringKey::Search),
    ("dialog", StringKey::Dialog),
    ("loading", StringKey::Loading),
    ("required_mark", StringKey::RequiredMark),
    ("empty", StringKey::Empty),
    ("avatar", StringKey::Avatar),
    ("chart", StringKey::Chart),
    ("no_data", StringKey::NoData),
    ("series", StringKey::Series),
    ("chart_summary", StringKey::ChartSummary),
    ("sparkline", StringKey::Sparkline),
    ("sparkline_summary", StringKey::SparklineSummary),
    ("heatmap", StringKey::Heatmap),
    ("heatmap_grid", StringKey::HeatmapGrid),
    ("code_editor", StringKey::CodeEditor),
    ("command_menu", StringKey::CommandMenu),
    ("status_bar", StringKey::StatusBar),
    ("menu_bar", StringKey::MenuBar),
    ("context_menu", StringKey::ContextMenu),
    ("keyboard_shortcuts", StringKey::KeyboardShortcuts),
    ("resize_sidebar", StringKey::ResizeSidebar),
    ("resize_panes", StringKey::ResizePanes),
    ("table_menu", StringKey::TableMenu),
    ("sort_ascending", StringKey::SortAscending),
    ("sort_descending", StringKey::SortDescending),
    ("hide_column", StringKey::HideColumn),
    ("autosize", StringKey::Autosize),
    ("move_left", StringKey::MoveLeft),
    ("move_right", StringKey::MoveRight),
    ("show_all_columns", StringKey::ShowAllColumns),
    ("select", StringKey::Select),
    ("select_all_rows", StringKey::SelectAllRows),
    ("select_row", StringKey::SelectRow),
];

const ENGLISH: [&str; StringKey::ALL.len()] = [
    "Close",
    "Cancel",
    "OK",
    "Search",
    "Dialog",
    "Loading",
    "*",
    "Empty",
    "avatar",
    "Chart",
    "No data",
    "Series {0}",
    "{0}: {1} values from {2} to {3}",
    "Sparkline",
    "{0} to {1}, last {2}",
    "Heatmap",
    "{0} by {1} grid",
    "Code editor",
    "Command Menu",
    "Status bar",
    "Menu bar",
    "Context menu",
    "Keyboard shortcuts",
    "Resize sidebar",
    "Resize panes",
    "Table menu",
    "Sort ascending",
    "Sort descending",
    "Hide column",
    "Autosize",
    "Move left",
    "Move right",
    "Show all columns",
    "Select",
    "Select all visible rows",
    "Select row {0}",
];

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
        if let Some(slot) = self.values.get_mut(key.index()) {
            *slot = value.into();
        }
    }

    #[must_use]
    pub fn get(&self, key: StringKey) -> &str {
        self.values.get(key.index()).map_or("", AsRef::as_ref)
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
