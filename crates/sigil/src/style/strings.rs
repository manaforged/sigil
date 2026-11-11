use std::borrow::Cow;
use std::fmt::Display;

macro_rules! string_keys {
    ($($key:ident $name:literal $english:literal,)*) => {
        /// A text that widgets show and an app can translate.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        #[non_exhaustive]
        pub enum StringKey {
            $(#[doc = concat!("The `", $name, "` text.")] $key,)*
        }

        impl StringKey {
            /// Every key.
            pub const ALL: [Self; [$(stringify!($key)),*].len()] = [$(Self::$key,)*];
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
    Toggle "toggle" "Toggle",
    Checkbox "checkbox" "Checkbox",
    NoticeError "notice_error" "Error",
    NoticeWarning "notice_warning" "Warning",
    NoticeSuccess "notice_success" "Success",
    Notice "notice" "Notice",
    Gauge "gauge" "Gauge",
    Progress "progress" "Progress",
    Slider "slider" "Slider",
    Tabs "tabs" "Tabs",
    CloseTab "close_tab" "Close tab",
    Menu "menu" "Menu",
    Diff "diff" "Diff",
    Math "math" "Math",
    Row "row" "row {0}",
    ResizeColumn "resize_column" "Resize {0}",
    ExpandDiagram "expand_diagram" "Expand diagram",
    ShowDiagram "show_diagram" "Show diagram",
    ShowCode "show_code" "Show code",
    Copied "copied" "Copied",
    CopySource "copy_source" "Copy source",
    ZoomOut "zoom_out" "Zoom out",
    ZoomIn "zoom_in" "Zoom in",
    Pagination "pagination" "Pagination",
    PreviousPage "previous_page" "Previous page",
    NextPage "next_page" "Next page",
    Breadcrumbs "breadcrumbs" "Breadcrumbs",
    Clear "clear" "Clear",
    RemoveItem "remove_item" "Remove {0}",
    Minimum "minimum" "minimum",
    Maximum "maximum" "maximum",
    Am "am" "AM",
    Pm "pm" "PM",
}

impl StringKey {
    /// Returns the key with this table name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, key)| *key)
    }
}

/// The translated texts of an app.
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
    /// Creates the English table.
    #[must_use]
    pub fn english() -> Self {
        Self {
            values: ENGLISH.map(Cow::Borrowed),
        }
    }

    /// Creates a table from name and text pairs.
    #[must_use]
    pub fn from_table(table: &[(StringKey, &'static str)]) -> Self {
        table.iter().fold(Self::english(), |strings, (key, value)| {
            strings.with(*key, *value)
        })
    }

    /// Sets one text.
    #[must_use]
    pub fn with(mut self, key: StringKey, value: impl Into<Cow<'static, str>>) -> Self {
        self.set(key, value);
        self
    }

    /// Sets one text.
    pub fn set(&mut self, key: StringKey, value: impl Into<Cow<'static, str>>) {
        if let Some(slot) = self.values.get_mut(key as usize) {
            *slot = value.into();
        }
    }

    /// Returns one text.
    #[must_use]
    pub fn get(&self, key: StringKey) -> &str {
        self.values.get(key as usize).map_or("", AsRef::as_ref)
    }

    /// Returns one text with `{n}` markers replaced by arguments.
    #[must_use]
    pub fn fill(&self, key: StringKey, args: &[&dyn Display]) -> String {
        args.iter()
            .enumerate()
            .fold(self.get(key).to_owned(), |text, (index, arg)| {
                text.replace(&format!("{{{index}}}"), &arg.to_string())
            })
    }
}
