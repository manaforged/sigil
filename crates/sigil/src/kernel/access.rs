mod role_map;

/// What a node is to a screen reader.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Role {
    /// The unknown role.
    Unknown,
    /// The text run role.
    TextRun,
    /// The cell role.
    Cell,
    /// The label role.
    Label,
    /// The image role.
    Image,
    /// The link role.
    Link,
    /// The row role.
    Row,
    /// The list item role.
    ListItem,
    /// The list marker role.
    ListMarker,
    /// The tree item role.
    TreeItem,
    /// The list box option role.
    ListBoxOption,
    /// The menu item role.
    MenuItem,
    /// The menu list option role.
    MenuListOption,
    /// The paragraph role.
    Paragraph,
    /// The generic container role.
    GenericContainer,
    /// The check box role.
    CheckBox,
    /// The radio button role.
    RadioButton,
    /// The text input role.
    TextInput,
    /// The button role.
    Button,
    /// The default button role.
    DefaultButton,
    /// The pane role.
    Pane,
    /// The row header role.
    RowHeader,
    /// The column header role.
    ColumnHeader,
    /// The row group role.
    RowGroup,
    /// The list role.
    List,
    /// The table role.
    Table,
    /// The switch role.
    Switch,
    /// The menu role.
    Menu,
    /// The multiline text input role.
    MultilineTextInput,
    /// The search input role.
    SearchInput,
    /// The date input role.
    DateInput,
    /// The date time input role.
    DateTimeInput,
    /// The week input role.
    WeekInput,
    /// The month input role.
    MonthInput,
    /// The time input role.
    TimeInput,
    /// The email input role.
    EmailInput,
    /// The number input role.
    NumberInput,
    /// The password input role.
    PasswordInput,
    /// The phone number input role.
    PhoneNumberInput,
    /// The url input role.
    UrlInput,
    /// The abbr role.
    Abbr,
    /// The alert role.
    Alert,
    /// The alert dialog role.
    AlertDialog,
    /// The application role.
    Application,
    /// The article role.
    Article,
    /// The audio role.
    Audio,
    /// The banner role.
    Banner,
    /// The blockquote role.
    Blockquote,
    /// The canvas role.
    Canvas,
    /// The caption role.
    Caption,
    /// The code role.
    Code,
    /// The color well role.
    ColorWell,
    /// The combo box role.
    ComboBox,
    /// The editable combo box role.
    EditableComboBox,
    /// The complementary role.
    Complementary,
    /// The comment role.
    Comment,
    /// The content deletion role.
    ContentDeletion,
    /// The content insertion role.
    ContentInsertion,
    /// The content info role.
    ContentInfo,
    /// The definition role.
    Definition,
    /// The description list role.
    DescriptionList,
    /// The details role.
    Details,
    /// The dialog role.
    Dialog,
    /// The disclosure triangle role.
    DisclosureTriangle,
    /// The document role.
    Document,
    /// The emphasis role.
    Emphasis,
    /// The feed role.
    Feed,
    /// The figure caption role.
    FigureCaption,
    /// The figure role.
    Figure,
    /// The footer role.
    Footer,
    /// The form role.
    Form,
    /// The grid role.
    Grid,
    /// The grid cell role.
    GridCell,
    /// The group role.
    Group,
    /// The header role.
    Header,
    /// The heading role.
    Heading,
    /// The keyboard role.
    Keyboard,
    /// The legend role.
    Legend,
    /// The line break role.
    LineBreak,
    /// The list box role.
    ListBox,
    /// The log role.
    Log,
    /// The main role.
    Main,
    /// The mark role.
    Mark,
    /// The marquee role.
    Marquee,
    /// The math role.
    Math,
    /// The menu bar role.
    MenuBar,
    /// The menu item check box role.
    MenuItemCheckBox,
    /// The menu item radio role.
    MenuItemRadio,
    /// The menu list popup role.
    MenuListPopup,
    /// The meter role.
    Meter,
    /// The navigation role.
    Navigation,
    /// The note role.
    Note,
    /// The progress indicator role.
    ProgressIndicator,
    /// The radio group role.
    RadioGroup,
    /// The region role.
    Region,
    /// The ruby role.
    Ruby,
    /// The ruby annotation role.
    RubyAnnotation,
    /// The scroll bar role.
    ScrollBar,
    /// The scroll view role.
    ScrollView,
    /// The search role.
    Search,
    /// The section role.
    Section,
    /// The section footer role.
    SectionFooter,
    /// The section header role.
    SectionHeader,
    /// The slider role.
    Slider,
    /// The spin button role.
    SpinButton,
    /// The splitter role.
    Splitter,
    /// The status role.
    Status,
    /// The strong role.
    Strong,
    /// The suggestion role.
    Suggestion,
    /// The tab role.
    Tab,
    /// The tab list role.
    TabList,
    /// The tab panel role.
    TabPanel,
    /// The term role.
    Term,
    /// The time role.
    Time,
    /// The timer role.
    Timer,
    /// The title bar role.
    TitleBar,
    /// The toolbar role.
    Toolbar,
    /// The tooltip role.
    Tooltip,
    /// The tree role.
    Tree,
    /// The tree grid role.
    TreeGrid,
    /// The video role.
    Video,
    /// The window role.
    Window,
    /// The terminal role.
    Terminal,
}

/// What a screen reader can ask a node to do.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Action {
    /// The click action.
    Click,
    /// The focus action.
    Focus,
    /// The blur action.
    Blur,
    /// The collapse action.
    Collapse,
    /// The expand action.
    Expand,
    /// The decrement action.
    Decrement,
    /// The increment action.
    Increment,
    /// The scroll into view action.
    ScrollIntoView,
    /// The set text selection action.
    SetTextSelection,
    /// The set value action.
    SetValue,
    /// The show context menu action.
    ShowContextMenu,
}

impl Action {
    pub(crate) const fn native(self) -> accesskit::Action {
        match self {
            Self::Click => accesskit::Action::Click,
            Self::Focus => accesskit::Action::Focus,
            Self::Blur => accesskit::Action::Blur,
            Self::Collapse => accesskit::Action::Collapse,
            Self::Expand => accesskit::Action::Expand,
            Self::Decrement => accesskit::Action::Decrement,
            Self::Increment => accesskit::Action::Increment,
            Self::ScrollIntoView => accesskit::Action::ScrollIntoView,
            Self::SetTextSelection => accesskit::Action::SetTextSelection,
            Self::SetValue => accesskit::Action::SetValue,
            Self::ShowContextMenu => accesskit::Action::ShowContextMenu,
        }
    }
}

/// How a screen reader announces a change to a live region.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Live {
    /// The off live.
    Off,
    /// The polite live.
    Polite,
    /// The assertive live.
    Assertive,
}

impl Live {
    pub(crate) const fn native(self) -> accesskit::Live {
        match self {
            Self::Off => accesskit::Live::Off,
            Self::Polite => accesskit::Live::Polite,
            Self::Assertive => accesskit::Live::Assertive,
        }
    }
}

/// The direction of a column sort.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SortDirection {
    /// The ascending sort direction.
    Ascending,
    /// The descending sort direction.
    Descending,
}

impl SortDirection {
    pub(crate) const fn native(self) -> accesskit::SortDirection {
        match self {
            Self::Ascending => accesskit::SortDirection::Ascending,
            Self::Descending => accesskit::SortDirection::Descending,
        }
    }
}
