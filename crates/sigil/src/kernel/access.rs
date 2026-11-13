mod role_map;

/// What a node is to a screen reader.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Role {
    /// A node with no known role.
    Unknown,
    /// A run of text on one line, the child of a text node.
    TextRun,
    /// A cell of a table.
    Cell,
    /// A text label for another control.
    Label,
    /// An image.
    Image,
    /// A link to another place.
    Link,
    /// A row of a table, a grid or a tree grid.
    Row,
    /// An item of a list.
    ListItem,
    /// The bullet or number in front of a list item.
    ListMarker,
    /// An item of a tree.
    TreeItem,
    /// An option of a list box.
    ListBoxOption,
    /// A command in a menu.
    MenuItem,
    /// An option in the popup of a drop-down list.
    MenuListOption,
    /// A paragraph of text.
    Paragraph,
    /// A container with no meaning of its own, the default role of a node.
    GenericContainer,
    /// A check box.
    CheckBox,
    /// A radio button, one choice of a group.
    RadioButton,
    /// A single-line text field.
    TextInput,
    /// A push button.
    Button,
    /// A push button that is the default action of its dialog.
    DefaultButton,
    /// A pane of a window.
    Pane,
    /// A cell that is the header of a table row.
    RowHeader,
    /// A cell that is the header of a table column.
    ColumnHeader,
    /// A group of rows in a table, such as its head or its body.
    RowGroup,
    /// A list of items.
    List,
    /// A table of rows and columns.
    Table,
    /// An on/off switch.
    Switch,
    /// A menu of commands.
    Menu,
    /// A text field with more than one line.
    MultilineTextInput,
    /// A text field for a search query.
    SearchInput,
    /// A field for a date.
    DateInput,
    /// A field for a date and a time.
    DateTimeInput,
    /// A field for a week of a year.
    WeekInput,
    /// A field for a month of a year.
    MonthInput,
    /// A field for a time of day.
    TimeInput,
    /// A text field for an email address.
    EmailInput,
    /// A text field for a number.
    NumberInput,
    /// A text field that hides what is typed.
    PasswordInput,
    /// A text field for a phone number.
    PhoneNumberInput,
    /// A text field for a web address.
    UrlInput,
    /// An abbreviation or acronym.
    Abbr,
    /// An urgent message, announced at once unless `live` says otherwise.
    Alert,
    /// A dialog with an urgent message that needs an answer.
    AlertDialog,
    /// A region that handles its own keyboard input as an application.
    Application,
    /// A self-contained piece of content, such as a post.
    Article,
    /// An audio player.
    Audio,
    /// The header of the whole page, with content that is the same on each screen.
    Banner,
    /// A quotation from another source.
    Blockquote,
    /// A surface that the app draws on.
    Canvas,
    /// The caption of a table or a figure.
    Caption,
    /// A fragment of computer code.
    Code,
    /// A control that picks a color.
    ColorWell,
    /// A drop-down list of options.
    ComboBox,
    /// A drop-down list with a text field for typing.
    EditableComboBox,
    /// Content that supports the main content, such as a sidebar.
    Complementary,
    /// A comment on a part of the content.
    Comment,
    /// Content marked as deleted.
    ContentDeletion,
    /// Content marked as inserted.
    ContentInsertion,
    /// Information about the whole page, such as a copyright footer.
    ContentInfo,
    /// The definition of a term.
    Definition,
    /// A list of terms and their descriptions.
    DescriptionList,
    /// A section that the user expands to show more.
    Details,
    /// A dialog box.
    Dialog,
    /// A control that expands and collapses a section.
    DisclosureTriangle,
    /// Content that the user reads as a document.
    Document,
    /// Text with stress emphasis.
    Emphasis,
    /// A scrolling list of articles that grows as the user reads.
    Feed,
    /// The caption of a figure.
    FigureCaption,
    /// A figure, such as an image or a chart, with an optional caption.
    Figure,
    /// The footer of a page or a section.
    Footer,
    /// A group of controls that make up one form.
    Form,
    /// A table whose cells take keyboard focus.
    Grid,
    /// A cell of a grid.
    GridCell,
    /// A set of related controls.
    Group,
    /// The header of a page or a section.
    Header,
    /// A section heading; `level` holds its rank.
    Heading,
    /// An on-screen keyboard.
    Keyboard,
    /// The caption of a group of form controls.
    Legend,
    /// A forced line break in text.
    LineBreak,
    /// A list of options to select from.
    ListBox,
    /// A region where new entries are added in order, such as a chat history.
    Log,
    /// The main content of the window.
    Main,
    /// Text highlighted for reference.
    Mark,
    /// A region of text that scrolls or changes on its own.
    Marquee,
    /// A mathematical expression.
    Math,
    /// A bar of menus.
    MenuBar,
    /// A menu item that can be checked.
    MenuItemCheckBox,
    /// A menu item that is one choice of a group.
    MenuItemRadio,
    /// The popup that lists the options of a drop-down list.
    MenuListPopup,
    /// A gauge that shows a value in a known range.
    Meter,
    /// A group of links for moving around the app.
    Navigation,
    /// A note that adds to the main content.
    Note,
    /// A progress bar.
    ProgressIndicator,
    /// A group of radio buttons.
    RadioGroup,
    /// A named section of the content.
    Region,
    /// Text with a pronunciation annotation.
    Ruby,
    /// The pronunciation annotation of ruby text.
    RubyAnnotation,
    /// A scroll bar.
    ScrollBar,
    /// A container that scrolls its content.
    ScrollView,
    /// A region that holds search controls.
    Search,
    /// A section of the content with no name.
    Section,
    /// The footer of a section.
    SectionFooter,
    /// The header of a section.
    SectionHeader,
    /// A slider that picks a value in a range.
    Slider,
    /// A number field with buttons that step the value.
    SpinButton,
    /// A bar that the user drags to resize two panes.
    Splitter,
    /// A status message, announced when the screen reader is idle unless `live` says otherwise.
    Status,
    /// Text with strong importance.
    Strong,
    /// A suggested change to the content.
    Suggestion,
    /// A tab that selects a tab panel.
    Tab,
    /// A row of tabs.
    TabList,
    /// The content that a tab shows.
    TabPanel,
    /// A term that has a definition.
    Term,
    /// A date or a time in text.
    Time,
    /// A counter of elapsed or remaining time.
    Timer,
    /// The title bar of a window.
    TitleBar,
    /// A bar of buttons and controls.
    Toolbar,
    /// A popup that describes a control.
    Tooltip,
    /// A tree of items that expand and collapse.
    Tree,
    /// A grid whose rows expand and collapse.
    TreeGrid,
    /// A video player.
    Video,
    /// A window.
    Window,
    /// A terminal emulator.
    Terminal,
}

/// What a screen reader can ask a node to do.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Action {
    /// Activates the node, as a click does.
    Click,
    /// Moves keyboard focus to the node.
    Focus,
    /// Removes keyboard focus from the node.
    Blur,
    /// Collapses the node.
    Collapse,
    /// Expands the node.
    Expand,
    /// Lowers the value by one step.
    Decrement,
    /// Raises the value by one step.
    Increment,
    /// Scrolls the node into view.
    ScrollIntoView,
    /// Sets the caret and the selection of the text.
    SetTextSelection,
    /// Sets the text or the numeric value of the node.
    SetValue,
    /// Opens the context menu of the node.
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
    /// Changes are not announced.
    Off,
    /// Changes are announced when the screen reader is idle.
    Polite,
    /// Changes are announced at once and interrupt speech.
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
    /// Sorted from the smallest value to the largest.
    Ascending,
    /// Sorted from the largest value to the smallest.
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
