# Module `sigil::prelude`

| Item | Kind | Description |
| --- | --- | --- |
| [`AppConfig`](#appconfig) | struct |  |
| [`Combobox`](#combobox) | struct |  |
| [`Date`](#date) | struct |  |
| [`DragState`](#dragstate) | struct |  |
| [`FocusRing`](#focusring) | struct |  |
| [`ImageSource`](#imagesource) | struct |  |
| [`IntervalId`](#intervalid) | struct |  |
| [`KeyEvent`](#keyevent) | struct |  |
| [`LayoutStyle`](#layoutstyle) | struct |  |
| [`ListRowSlots`](#listrowslots) | struct |  |
| [`MenuItem`](#menuitem) | struct |  |
| [`ModalBuilder`](#modalbuilder) | struct |  |
| [`MouseButtons`](#mousebuttons) | struct |  |
| [`NodeBuilder`](#nodebuilder) | struct |  |
| [`PathNode`](#pathnode) | struct |  |
| [`Point`](#point) | struct |  |
| [`PointerEvent`](#pointerevent) | struct |  |
| [`Rgba8`](#rgba8) | struct |  |
| [`Shortcut`](#shortcut) | struct |  |
| [`Signal`](#signal) | struct |  |
| [`SignalWriter`](#signalwriter) | struct |  |
| [`Size`](#size) | struct |  |
| [`SortKey`](#sortkey) | struct |  |
| [`TableColumn`](#tablecolumn) | struct |  |
| [`TableState`](#tablestate) | struct |  |
| [`Task`](#task) | struct |  |
| [`Theme`](#theme) | struct |  |
| [`ThemeWatcher`](#themewatcher) | struct |  |
| [`ToastQueue`](#toastqueue) | struct |  |
| [`AlignItems`](#alignitems) | enum |  |
| [`ColWidth`](#colwidth) | enum |  |
| [`ColorAtom`](#coloratom) | enum |  |
| [`ControlSize`](#controlsize) | enum |  |
| [`Density`](#density) | enum |  |
| [`DiffView`](#diffview) | enum |  |
| [`Disclosure`](#disclosure) | enum |  |
| [`Display`](#display) | enum |  |
| [`Elevation`](#elevation) | enum |  |
| [`FlexWrap`](#flexwrap) | enum |  |
| [`FontFamily`](#fontfamily) | enum |  |
| [`Icon`](#icon) | enum |  |
| [`JustifyContent`](#justifycontent) | enum |  |
| [`KeyPhase`](#keyphase) | enum |  |
| [`LogicalKey`](#logicalkey) | enum |  |
| [`MouseButton`](#mousebutton) | enum |  |
| [`Orientation`](#orientation) | enum |  |
| [`PointerKind`](#pointerkind) | enum |  |
| [`PointerPhase`](#pointerphase) | enum |  |
| [`Radius`](#radius) | enum |  |
| [`ScheduleError`](#scheduleerror) | enum |  |
| [`SelectionMode`](#selectionmode) | enum |  |
| [`Side`](#side) | enum |  |
| [`SortDir`](#sortdir) | enum |  |
| [`Status`](#status) | enum |  |
| [`SubmitKey`](#submitkey) | enum |  |
| [`TextContent`](#textcontent) | enum |  |
| [`Token`](#token) | enum |  |
| [`Tone`](#tone) | enum |  |
| [`DragDropExt`](#dragdropext) | trait |  |
| [`Identifiable`](#identifiable) | trait |  |
| [`IntoNode`](#intonode) | trait |  |
| [`IntoPx`](#intopx) | trait |  |
| [`LayoutBuilderExt`](#layoutbuilderext) | trait |  |
| [`LayoutReactiveExt`](#layoutreactiveext) | trait |  |
| [`NodeEventsExt`](#nodeeventsext) | trait |  |
| [`NodeStyleExt`](#nodestyleext) | trait |  |
| [`NumericKind`](#numerickind) | trait |  |
| [`Paint`](#paint) | trait |  |
| [`Reactive`](#reactive) | trait |  |
| [`StateVariants`](#statevariants) | trait |  |
| [`WidgetLayoutExt`](#widgetlayoutext) | trait |  |
| [`app_shell`](#app_shell) | fn |  |
| [`avatar`](#avatar) | fn |  |
| [`axis_drag`](#axis_drag) | fn |  |
| [`button`](#button) | fn |  |
| [`card`](#card) | fn |  |
| [`chart`](#chart) | fn |  |
| [`checkbox`](#checkbox) | fn |  |
| [`chip`](#chip) | fn |  |
| [`code_edit`](#code_edit) | fn |  |
| [`color_field`](#color_field) | fn |  |
| [`cols`](#cols) | fn |  |
| [`column`](#column) | fn |  |
| [`combobox`](#combobox-1) | fn |  |
| [`command_menu`](#command_menu) | fn |  |
| [`container`](#container) | fn |  |
| [`context_menu`](#context_menu) | fn |  |
| [`date_field`](#date_field) | fn |  |
| [`derived`](#derived) | fn |  |
| [`diff`](#diff) | fn |  |
| [`divider_horizontal`](#divider_horizontal) | fn |  |
| [`divider_vertical`](#divider_vertical) | fn |  |
| [`dot`](#dot) | fn |  |
| [`drag_ghost`](#drag_ghost) | fn |  |
| [`dropdown`](#dropdown) | fn |  |
| [`effect`](#effect) | fn |  |
| [`em`](#em) | fn |  |
| [`empty_state`](#empty_state) | fn |  |
| [`field`](#field) | fn |  |
| [`for_each`](#for_each) | fn |  |
| [`fragment`](#fragment) | fn |  |
| [`fuzzy_score`](#fuzzy_score) | fn |  |
| [`gauge`](#gauge) | fn |  |
| [`heatmap`](#heatmap) | fn |  |
| [`highlight`](#highlight) | fn |  |
| [`hint_bar`](#hint_bar) | fn |  |
| [`hint_overlay`](#hint_overlay) | fn |  |
| [`hl_spans`](#hl_spans) | fn |  |
| [`html`](#html) | fn |  |
| [`icon`](#icon-1) | fn |  |
| [`icon_button`](#icon_button) | fn |  |
| [`image`](#image) | fn |  |
| [`image_bytes`](#image_bytes) | fn |  |
| [`image_rgba`](#image_rgba) | fn |  |
| [`interactive`](#interactive) | fn |  |
| [`interval`](#interval) | fn |  |
| [`kv_grid`](#kv_grid) | fn |  |
| [`kv_row`](#kv_row) | fn |  |
| [`list_row`](#list_row) | fn |  |
| [`list_row_with`](#list_row_with) | fn |  |
| [`markdown`](#markdown) | fn |  |
| [`markdown_edit`](#markdown_edit) | fn |  |
| [`markdown_inline`](#markdown_inline) | fn |  |
| [`match_status`](#match_status) | fn |  |
| [`menu`](#menu) | fn |  |
| [`menu_bar`](#menu_bar) | fn |  |
| [`mermaid`](#mermaid) | fn |  |
| [`modal`](#modal) | fn |  |
| [`node`](#node) | fn |  |
| [`notice_band`](#notice_band) | fn |  |
| [`number_field`](#number_field) | fn |  |
| [`open`](#open) | fn |  |
| [`panel`](#panel) | fn |  |
| [`path`](#path) | fn |  |
| [`progress`](#progress) | fn |  |
| [`radio_group`](#radio_group) | fn |  |
| [`request_focus`](#request_focus) | fn |  |
| [`row`](#row) | fn |  |
| [`rows`](#rows) | fn |  |
| [`run`](#run) | fn |  |
| [`scroll`](#scroll) | fn |  |
| [`section_label`](#section_label) | fn |  |
| [`segmented`](#segmented) | fn |  |
| [`set_theme`](#set_theme) | fn |  |
| [`sheet`](#sheet) | fn |  |
| [`show`](#show) | fn |  |
| [`sidebar`](#sidebar) | fn |  |
| [`signal`](#signal-1) | fn |  |
| [`signal_writer`](#signal_writer) | fn |  |
| [`slider`](#slider) | fn |  |
| [`spacer`](#spacer) | fn |  |
| [`sparkline`](#sparkline) | fn |  |
| [`spawn`](#spawn) | fn |  |
| [`spinner`](#spinner) | fn |  |
| [`splitter`](#splitter) | fn |  |
| [`stat`](#stat) | fn |  |
| [`status_bar`](#status_bar) | fn |  |
| [`surface`](#surface) | fn |  |
| [`switch`](#switch) | fn |  |
| [`table`](#table) | fn |  |
| [`tabs`](#tabs) | fn |  |
| [`task`](#task-1) | fn |  |
| [`task_with`](#task_with) | fn |  |
| [`terminal`](#terminal) | fn |  |
| [`text`](#text) | fn |  |
| [`text_edit`](#text_edit) | fn |  |
| [`text_field`](#text_field) | fn |  |
| [`toast`](#toast) | fn |  |
| [`toast_queue`](#toast_queue) | fn |  |
| [`toggle`](#toggle) | fn |  |
| [`tooltip`](#tooltip) | fn |  |
| [`tree`](#tree) | fn |  |
| [`virtual_list`](#virtual_list) | fn |  |
| [`watch_theme`](#watch_theme) | fn |  |
| [`AppResult`](#appresult) | type |  |
| [`Action`](#action) | use |  |
| [`Key`](#key) | use |  |
| [`ModifiersState`](#modifiersstate) | use |  |
| [`Role`](#role) | use |  |

## Structs

### `AppConfig`


### `Combobox`


### `Date`

**Fields**

| Field | Description |
| --- | --- |
| <code>day: u8</code> |  |
| <code>month: u8</code> |  |
| <code>year: i32</code> |  |


### `DragState`


### `FocusRing`


### `ImageSource`


### `IntervalId`


### `KeyEvent`

**Fields**

| Field | Description |
| --- | --- |
| <code>key: <a href="sigil-events.html#logicalkey">LogicalKey</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="sigil-events.html#keyphase">KeyPhase</a></code> |  |
| <code>physical: Code</code> |  |
| <code>repeat: bool</code> |  |
| <code>text: Option&lt;SmolStr&gt;</code> |  |


### `LayoutStyle`


### `ListRowSlots`

**Fields**

| Field | Description |
| --- | --- |
| <code>activate: Option&lt;Rc&lt;dyn Fn()&gt;&gt;</code> |  |
| <code>badge: Option&lt;(<a href="sigil-kernel.html#textcontent">TextContent</a>, <a href="sigil-style.html#tone">Tone</a>)&gt;</code> |  |
| <code>columns: Vec&lt;<a href="sigil-widgets.html#listcolumn">ListColumn</a>&gt;</code> |  |
| <code>depth: u16</code> |  |
| <code>detail: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>disclosure: <a href="sigil-widgets.html#disclosure">Disclosure</a></code> |  |
| <code>indent: Option&lt;f32&gt;</code> |  |
| <code>leading: Option&lt;Box&lt;dyn <a href="sigil-kernel.html#intonode">IntoNode</a>&gt;&gt;</code> |  |
| <code>marker: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>marker_tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |
| <code>shrink: bool</code> |  |
| <code>status: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>status_tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |
| <code>title_width: Option&lt;f32&gt;</code> |  |
| <code>tone: <a href="sigil-widgets.html#rowtone">RowTone</a></code> |  |


### `MenuItem`

**Fields**

| Field | Description |
| --- | --- |
| <code>disabled: bool</code> |  |
| <code>label: String</code> |  |
| <code>shortcut: Option&lt;String&gt;</code> |  |
| <code>submenu_indicator: bool</code> |  |


### `ModalBuilder`


### `MouseButtons`


### `NodeBuilder`


### `PathNode`


### `Point`

**Fields**

| Field | Description |
| --- | --- |
| <code>x: f32</code> |  |
| <code>y: f32</code> |  |


### `PointerEvent`

**Fields**

| Field | Description |
| --- | --- |
| <code>button: Option&lt;<a href="sigil-events.html#mousebutton">MouseButton</a>&gt;</code> |  |
| <code>buttons: <a href="sigil-events.html#mousebuttons">MouseButtons</a></code> |  |
| <code>delta: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>kind: <a href="sigil-events.html#pointerkind">PointerKind</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="sigil-events.html#pointerphase">PointerPhase</a></code> |  |
| <code>pointer_id: u64</code> |  |
| <code>position: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>pressure: Option&lt;f32&gt;</code> |  |
| <code>target_size: <a href="sigil-geometry.html#logicalsize">LogicalSize</a></code> |  |
| <code>tilt: Option&lt;<a href="sigil-events.html#pointertilt">PointerTilt</a>&gt;</code> |  |
| <code>window_position: <a href="sigil-geometry.html#point">Point</a></code> |  |


### `Rgba8`

**Fields**

| Field | Description |
| --- | --- |
| <code>a: u8</code> |  |
| <code>b: u8</code> |  |
| <code>g: u8</code> |  |
| <code>r: u8</code> |  |


### `Shortcut`

**Fields**

| Field | Description |
| --- | --- |
| <code>key: <a href="sigil-events.html#logicalkey">LogicalKey</a></code> |  |
| <code>modifiers: Modifiers</code> |  |


### `Signal`


### `SignalWriter`


### `Size`

**Fields**

| Field | Description |
| --- | --- |
| <code>height: <a href="sigil-geometry.html#dim">Dim</a></code> |  |
| <code>width: <a href="sigil-geometry.html#dim">Dim</a></code> |  |


### `SortKey`

**Fields**

| Field | Description |
| --- | --- |
| <code>column: &amp;'static str</code> |  |
| <code>dir: <a href="sigil-widgets.html#sortdir">SortDir</a></code> |  |


### `TableColumn`


### `TableState`


### `Task`


### `Theme`

**Fields**

| Field | Description |
| --- | --- |
| <code>accent: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>accent_hover: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>accent_scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>border: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>direction: <a href="sigil-style.html#direction">Direction</a></code> |  |
| <code>error: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>gray: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>info: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>locale: <a href="sigil-style.html#locale">Locale</a></code> |  |
| <code>metrics: <a href="sigil-style.html#metrics">Metrics</a></code> |  |
| <code>missing_token_fallback: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>on_accent: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>on_solid: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>on_surface: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>strings: <a href="sigil-style.html#strings">Strings</a></code> |  |
| <code>success: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>surface: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>surface_hover: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>surface_pressed: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>surface_pressed_strong: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>tokens: <a href="sigil-style.html#tokentable">TokenTable</a></code> |  |
| <code>warn: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |


### `ThemeWatcher`


### `ToastQueue`


## Enums

### `AlignItems`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Baseline</code> |  |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |


### `ColWidth`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Fixed(f32)</code> |  |
| <code>Fraction(f32)</code> |  |


### `ColorAtom`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Rgba8(<a href="sigil-style.html#rgba8">Rgba8</a>)</code> |  |
| <code>ThemeRef(<a href="sigil-style.html#colorkey">ColorKey</a>)</code> |  |


### `ControlSize`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compact</code> |  |
| <code>Default</code> |  |
| <code>Grow</code> |  |
| <code>Wide</code> |  |


### `Density`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compact</code> |  |
| <code>Default</code> |  |
| <code>Grid</code> |  |


### `DiffView`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Split</code> |  |
| <code>Unified</code> |  |


### `Disclosure`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Collapsed</code> |  |
| <code>Expanded</code> |  |
| <code>Leaf</code> |  |
| <code>None</code> |  |


### `Display`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Block</code> |  |
| <code>Flex</code> |  |
| <code>Grid</code> |  |
| <code>None</code> |  |


### `Elevation`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Base</code> |  |
| <code>Lg</code> |  |
| <code>Md</code> |  |
| <code>Sm</code> |  |
| <code>Xl</code> |  |


### `FlexWrap`

**Variants**

| Variant | Description |
| --- | --- |
| <code>NoWrap</code> |  |
| <code>Wrap</code> |  |
| <code>WrapReverse</code> |  |


### `FontFamily`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Display</code> |  |
| <code>Icon</code> |  |
| <code>Mono</code> |  |
| <code>Sans</code> |  |
| <code>Serif</code> |  |


### `Icon`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Activity</code> |  |
| <code>ArrowDown</code> |  |
| <code>ArrowLeft</code> |  |
| <code>ArrowRight</code> |  |
| <code>ArrowUp</code> |  |
| <code>AtSign</code> |  |
| <code>Bell</code> |  |
| <code>Box</code> |  |
| <code>Braces</code> |  |
| <code>Bug</code> |  |
| <code>Calendar</code> |  |
| <code>ChartColumn</code> |  |
| <code>Check</code> |  |
| <code>ChevronDown</code> |  |
| <code>ChevronLeft</code> |  |
| <code>ChevronRight</code> |  |
| <code>ChevronUp</code> |  |
| <code>ChevronsUpDown</code> |  |
| <code>Circle</code> |  |
| <code>CircleAlert</code> |  |
| <code>CircleCheck</code> |  |
| <code>CircleX</code> |  |
| <code>Clock</code> |  |
| <code>Cloud</code> |  |
| <code>Code</code> |  |
| <code>Command</code> |  |
| <code>Copy</code> |  |
| <code>Cpu</code> |  |
| <code>Database</code> |  |
| <code>Download</code> |  |
| <code>Ellipsis</code> |  |
| <code>EllipsisVertical</code> |  |
| <code>ExternalLink</code> |  |
| <code>Eye</code> |  |
| <code>EyeOff</code> |  |
| <code>File</code> |  |
| <code>FileCode</code> |  |
| <code>FileText</code> |  |
| <code>Folder</code> |  |
| <code>FolderOpen</code> |  |
| <code>Funnel</code> |  |
| <code>Gauge</code> |  |
| <code>GitBranch</code> |  |
| <code>GitCommitHorizontal</code> |  |
| <code>GitPullRequest</code> |  |
| <code>Globe</code> |  |
| <code>GripVertical</code> |  |
| <code>Hash</code> |  |
| <code>Heart</code> |  |
| <code>History</code> |  |
| <code>House</code> |  |
| <code>Inbox</code> |  |
| <code>Info</code> |  |
| <code>Key</code> |  |
| <code>Layers</code> |  |
| <code>LayoutDashboard</code> |  |
| <code>Link</code> |  |
| <code>List</code> |  |
| <code>ListTree</code> |  |
| <code>Lock</code> |  |
| <code>LockOpen</code> |  |
| <code>LogIn</code> |  |
| <code>LogOut</code> |  |
| <code>Mail</code> |  |
| <code>Menu</code> |  |
| <code>Minus</code> |  |
| <code>Monitor</code> |  |
| <code>Moon</code> |  |
| <code>Package</code> |  |
| <code>PanelBottom</code> |  |
| <code>PanelLeft</code> |  |
| <code>Pause</code> |  |
| <code>Pencil</code> |  |
| <code>Play</code> |  |
| <code>Plus</code> |  |
| <code>RefreshCw</code> |  |
| <code>Repeat</code> |  |
| <code>Repeat1</code> |  |
| <code>Rocket</code> |  |
| <code>RotateCcw</code> |  |
| <code>ScrollText</code> |  |
| <code>Search</code> |  |
| <code>Server</code> |  |
| <code>Settings</code> |  |
| <code>Shield</code> |  |
| <code>Shuffle</code> |  |
| <code>SkipBack</code> |  |
| <code>SkipForward</code> |  |
| <code>SlidersHorizontal</code> |  |
| <code>Square</code> |  |
| <code>Star</code> |  |
| <code>Sun</code> |  |
| <code>Terminal</code> |  |
| <code>Trash2</code> |  |
| <code>TrendingDown</code> |  |
| <code>TrendingUp</code> |  |
| <code>Triangle</code> |  |
| <code>TriangleAlert</code> |  |
| <code>Upload</code> |  |
| <code>User</code> |  |
| <code>Users</code> |  |
| <code>Volume2</code> |  |
| <code>VolumeX</code> |  |
| <code>X</code> |  |
| <code>Zap</code> |  |


### `JustifyContent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>SpaceAround</code> |  |
| <code>SpaceBetween</code> |  |
| <code>SpaceEvenly</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |


### `KeyPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Down</code> |  |
| <code>Up</code> |  |


### `LogicalKey`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Character(SmolStr)</code> |  |
| <code>Named(NamedKey)</code> |  |


### `MouseButton`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Back</code> |  |
| <code>Forward</code> |  |
| <code>Left</code> |  |
| <code>Middle</code> |  |
| <code>Other(u16)</code> |  |
| <code>Right</code> |  |


### `Orientation`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Horizontal</code> |  |
| <code>Vertical</code> |  |


### `PointerKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Mouse</code> |  |
| <code>Pen</code> |  |
| <code>Touch</code> |  |


### `PointerPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cancel</code> |  |
| <code>Down</code> |  |
| <code>Enter</code> |  |
| <code>Leave</code> |  |
| <code>Move</code> |  |
| <code>Up</code> |  |


### `Radius`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Base</code> |  |
| <code>Full</code> |  |
| <code>Lg</code> |  |
| <code>Md</code> |  |
| <code>None</code> |  |
| <code>Sm</code> |  |


### `ScheduleError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Closed</code> |  |
| <code>Full</code> |  |


### `SelectionMode`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Extend</code> |  |
| <code>Multi</code> |  |
| <code>None</code> |  |
| <code>Single</code> |  |


### `Side`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bottom</code> |  |
| <code>Left</code> |  |
| <code>Right</code> |  |


### `SortDir`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ascending</code> |  |
| <code>Descending</code> |  |


### `Status`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Err(E)</code> |  |
| <code>Idle</code> |  |
| <code>Ok(T)</code> |  |
| <code>Panicked(String)</code> |  |
| <code>Pending</code> |  |


### `SubmitKey`

**Variants**

| Variant | Description |
| --- | --- |
| <code>CtrlEnter</code> |  |
| <code>Enter</code> |  |


### `TextContent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compute(Rc&lt;dyn Fn() -&gt; String&gt;)</code> |  |
| <code>Reactive(<a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;)</code> |  |
| <code>Static(String)</code> |  |


### `Token`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Accent</code> |  |
| <code>Accent1</code> |  |
| <code>Accent10</code> |  |
| <code>Accent11</code> |  |
| <code>Accent12</code> |  |
| <code>Accent2</code> |  |
| <code>Accent3</code> |  |
| <code>Accent4</code> |  |
| <code>Accent5</code> |  |
| <code>Accent6</code> |  |
| <code>Accent7</code> |  |
| <code>Accent8</code> |  |
| <code>Accent9</code> |  |
| <code>AccentHover</code> |  |
| <code>Border</code> |  |
| <code>BorderStrong</code> |  |
| <code>BorderSubtle</code> |  |
| <code>Error1</code> |  |
| <code>Error10</code> |  |
| <code>Error11</code> |  |
| <code>Error12</code> |  |
| <code>Error2</code> |  |
| <code>Error3</code> |  |
| <code>Error4</code> |  |
| <code>Error5</code> |  |
| <code>Error6</code> |  |
| <code>Error7</code> |  |
| <code>Error8</code> |  |
| <code>Error9</code> |  |
| <code>Field</code> |  |
| <code>Gray1</code> |  |
| <code>Gray10</code> |  |
| <code>Gray11</code> |  |
| <code>Gray12</code> |  |
| <code>Gray2</code> |  |
| <code>Gray3</code> |  |
| <code>Gray4</code> |  |
| <code>Gray5</code> |  |
| <code>Gray6</code> |  |
| <code>Gray7</code> |  |
| <code>Gray8</code> |  |
| <code>Gray9</code> |  |
| <code>Info1</code> |  |
| <code>Info10</code> |  |
| <code>Info11</code> |  |
| <code>Info12</code> |  |
| <code>Info2</code> |  |
| <code>Info3</code> |  |
| <code>Info4</code> |  |
| <code>Info5</code> |  |
| <code>Info6</code> |  |
| <code>Info7</code> |  |
| <code>Info8</code> |  |
| <code>Info9</code> |  |
| <code>OnAccent</code> |  |
| <code>OnSolid</code> |  |
| <code>OnSurface</code> |  |
| <code>OnSurfaceMuted</code> |  |
| <code>Overlay</code> |  |
| <code>OverlayBorder</code> |  |
| <code>Ring</code> |  |
| <code>Scrim</code> |  |
| <code>Success1</code> |  |
| <code>Success10</code> |  |
| <code>Success11</code> |  |
| <code>Success12</code> |  |
| <code>Success2</code> |  |
| <code>Success3</code> |  |
| <code>Success4</code> |  |
| <code>Success5</code> |  |
| <code>Success6</code> |  |
| <code>Success7</code> |  |
| <code>Success8</code> |  |
| <code>Success9</code> |  |
| <code>Surface</code> |  |
| <code>SurfaceHover</code> |  |
| <code>SurfaceInset</code> |  |
| <code>SurfacePressed</code> |  |
| <code>SurfacePressedStrong</code> |  |
| <code>SurfaceSubtle</code> |  |
| <code>TableHeader</code> |  |
| <code>Warn1</code> |  |
| <code>Warn10</code> |  |
| <code>Warn11</code> |  |
| <code>Warn12</code> |  |
| <code>Warn2</code> |  |
| <code>Warn3</code> |  |
| <code>Warn4</code> |  |
| <code>Warn5</code> |  |
| <code>Warn6</code> |  |
| <code>Warn7</code> |  |
| <code>Warn8</code> |  |
| <code>Warn9</code> |  |


### `Tone`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Error</code> |  |
| <code>Info</code> |  |
| <code>Neutral</code> |  |
| <code>Primary</code> |  |
| <code>Success</code> |  |
| <code>Warn</code> |  |


## Traits

### `DragDropExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>drag_source(self, state: <a href="sigil-widgets.html#dragstate">DragState</a>&lt;T&gt;, payload: T) -&gt; Self</code> |  |
| <code>drop_target(self, state: <a href="sigil-widgets.html#dragstate">DragState</a>&lt;T&gt;, accepts: impl Fn(&amp;T) -&gt; bool + 'static, on_drop: impl Fn(T) + 'static) -&gt; Self</code> |  |


### `Identifiable`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Id: Clone + Eq + Hash</code> |  |
| <code>id(&amp;self) -&gt; Self::Id</code> |  |


### `IntoNode`

**Methods**

| Method | Description |
| --- | --- |
| <code>build(self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>) -&gt; <a href="sigil-kernel.html#nodeid">NodeId</a></code> |  |


### `IntoPx`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_px(self) -&gt; f32</code> |  |


### `LayoutBuilderExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>layout(self, style: <a href="sigil-layout.html#layoutstyle">LayoutStyle</a>) -&gt; Self</code> |  |


### `LayoutReactiveExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>bounds_reactive(self, bounds: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32, f32, f32)&gt;) -&gt; Self</code> |  |
| <code>position_reactive(self, pos: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32)&gt;) -&gt; Self</code> |  |
| <code>size_reactive(self, size: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32)&gt;) -&gt; Self</code> |  |


### `NodeEventsExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>autofocus(self) -&gt; Self</code> |  |
| <code>file_drop&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;<a href="sigil-events.html#filedropevent">FileDropEvent</a>) + 'static</code> |  |
| <code>focus_in&lt;H&gt;(self, h: H) -&gt; Self where H: Fn() + 'static</code> |  |
| <code>focus_out&lt;H&gt;(self, h: H) -&gt; Self where H: Fn() + 'static</code> |  |
| <code>focus_when(self, trigger: <a href="sigil-reactive.html#signal">Signal</a>&lt;u64&gt;) -&gt; Self</code> |  |
| <code>ime_caret(self, anchor: Rc&lt;Cell&lt;Option&lt;<a href="sigil-layout.html#caretgeometry">CaretGeometry</a>&gt;&gt;&gt;) -&gt; Self</code> |  |
| <code>ime_eligible&lt;F&gt;(self, f: F) -&gt; Self where F: Fn() -&gt; bool + 'static</code> |  |
| <code>key&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_any&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_capture&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_capture_any&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>link&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;str) + 'static</code> |  |
| <code>on_gesture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#gestureevent">GestureEvent</a>) + 'static</code> |  |
| <code>on_ime&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;<a href="sigil-events.html#imeevent">ImeEvent</a>) + 'static</code> |  |
| <code>on_paste&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pasteevent">PasteEvent</a>) + 'static</code> |  |
| <code>on_scroll&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#scrollevent">ScrollEvent</a>) + 'static</code> |  |
| <code>on_selection_copy&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;str, &amp;<a href="sigil.html#appresult">AppResult</a>&lt;<a href="sigil-clipboard.html#delivery">Delivery</a>&gt;) + 'static</code> |  |
| <code>pointer_down&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_down_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_enter&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_leave&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_move&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_move_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_up&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_up_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="sigil-events.html#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>selectable_text(self, yes: bool) -&gt; Self</code> |  |
| <code>shortcut&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static</code> |  |


### `NodeStyleExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>animate(self, kind: <a href="sigil-style.html#numericatomkind">NumericAtomKind</a>, value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>focus_ring(self, ring: <a href="sigil-style.html#focusring">FocusRing</a>) -&gt; Self</code> |  |
| <code>paint_when&lt;F&gt;(self, active: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;, paint: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>presence(self, shown: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;, config: <a href="sigil-animation_runtime.html#animationconfig">AnimationConfig</a>) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |
| <code>tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>visible_when(self, shown: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |


### `NumericKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>clamp_to(self, range: &amp;RangeInclusive&lt;Self&gt;) -&gt; Self</code> |  |
| <code>parse(input: &amp;str, locale: &amp;<a href="sigil-style.html#locale">Locale</a>) -&gt; Option&lt;Self&gt;</code> |  |
| <code>render(self, locale: &amp;<a href="sigil-style.html#locale">Locale</a>) -&gt; String</code> |  |


### `Paint`

**Methods**

| Method | Description |
| --- | --- |
| <code>background_gradient(self, gradient: <a href="sigil-style.html#bggradient">BgGradient</a>) -&gt; Self</code> |  |
| <code>bg(self, color: impl Into&lt;<a href="sigil-style.html#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>border_bottom(self, px: i16) -&gt; Self</code> |  |
| <code>border_color(self, color: impl Into&lt;<a href="sigil-style.html#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>border_left(self, px: i16) -&gt; Self</code> |  |
| <code>border_right(self, px: i16) -&gt; Self</code> |  |
| <code>border_top(self, px: i16) -&gt; Self</code> |  |
| <code>border_w(self, px: i16) -&gt; Self</code> |  |
| <code>duration(self, duration: <a href="sigil-style.html#millisu16">MillisU16</a>) -&gt; Self</code> |  |
| <code>font_family(self, family: <a href="sigil-style.html#fontfamily">FontFamily</a>) -&gt; Self</code> |  |
| <code>font_italic(self, italic: bool) -&gt; Self</code> |  |
| <code>font_size(self, px: f32) -&gt; Self</code> |  |
| <code>font_weight(self, weight: u16) -&gt; Self</code> |  |
| <code>letter_spacing_em(self, em: f32) -&gt; Self</code> |  |
| <code>line_height_px(self, px: f32) -&gt; Self</code> |  |
| <code>line_through(self) -&gt; Self</code> |  |
| <code>opacity(self, value: f32) -&gt; Self</code> |  |
| <code>rounded(self, radius: <a href="sigil-style.html#radius">Radius</a>) -&gt; Self</code> |  |
| <code>shadow(self, elevation: <a href="sigil-style.html#elevation">Elevation</a>) -&gt; Self</code> |  |
| <code>tabular_nums(self, on: bool) -&gt; Self</code> |  |
| <code>text_color(self, color: impl Into&lt;<a href="sigil-style.html#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>transform(self, transform: <a href="sigil-style.html#transform2d">Transform2D</a>) -&gt; Self</code> |  |
| <code>transition(self, id: <a href="sigil-style.html#transitionid">TransitionId</a>) -&gt; Self</code> |  |
| <code>underline(self) -&gt; Self</code> |  |


### `Reactive`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_signal(self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt; where Self: Sized, T: Clone + PartialEq + 'static</code> |  |
| <code>read(&amp;self) -&gt; T</code> |  |


### `StateVariants`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled_style&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>focus&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>hover&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>pressed&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>rest&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="sigil-style.html#statestylebuilder">StateStyleBuilder</a></code> |  |


### `WidgetLayoutExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>grow(self, factor: f32) -&gt; Self</code> |  |
| <code>height(self, height: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>layout(self, style: <a href="sigil-layout.html#layoutstyle">LayoutStyle</a>) -&gt; Self</code> |  |
| <code>layout_overrides_mut(&amp;mut self) -&gt; &amp;mut <a href="sigil-widgets-layout_ext.html#layoutoverrides">LayoutOverrides</a></code> |  |
| <code>map_node(self, f: impl FnOnce(<a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> + 'static) -&gt; Self</code> |  |
| <code>width(self, width: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |


## Functions

### `app_shell`

<pre>pub fn <a href="#app_shell">app_shell</a>(top: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, work: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, status: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, modal: Option&lt;impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `avatar`

<pre>pub fn <a href="#avatar">avatar</a>(initials: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `axis_drag`

<pre>pub fn <a href="#axis_drag">axis_drag</a>&lt;S&gt;(node: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>, axis: <a href="sigil-widgets.html#axis">Axis</a>, snapshot: impl Fn() -&gt; S + 'static, on_delta: impl Fn(S, f32) + 'static, on_end: impl Fn() + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where S: Clone + 'static</pre>


### `button`

<pre>pub fn <a href="#button">button</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-widgets.html#buttonbuilder">ButtonBuilder</a></pre>


### `card`

<pre>pub fn <a href="#card">card</a>(child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="sigil-widgets.html#cardbuilder">CardBuilder</a></pre>


### `chart`

<pre>pub fn <a href="#chart">chart</a>(values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; <a href="sigil-widgets.html#chartbuilder">ChartBuilder</a></pre>


### `checkbox`

<pre>pub fn <a href="#checkbox">checkbox</a>(checked: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="sigil-widgets.html#checkboxbuilder">CheckboxBuilder</a></pre>


### `chip`

<pre>pub fn <a href="#chip">chip</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="sigil-widgets.html#chipbuilder">ChipBuilder</a></pre>


### `code_edit`

<pre>pub fn <a href="#code_edit">code_edit</a>(lines: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;String&gt;&gt;, caret: <a href="sigil-reactive.html#signal">Signal</a>&lt;(usize, usize)&gt;) -&gt; <a href="sigil-widgets.html#codeeditbuilder">CodeEditBuilder</a></pre>


### `color_field`

<pre>pub const fn <a href="#color_field">color_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="sigil-style.html#rgba8">Rgba8</a>&gt;) -&gt; <a href="sigil-widgets.html#colorfieldbuilder">ColorFieldBuilder</a></pre>


### `cols`

<pre>pub const fn <a href="#cols">cols</a>(count: f32) -&gt; <a href="sigil-layout.html#textlength">TextLength</a></pre>


### `column`

<pre>pub fn <a href="#column">column</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `combobox`

<pre>pub fn <a href="#combobox-1">combobox</a>(state: <a href="sigil-widgets.html#combobox">Combobox</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `command_menu`

<pre>pub fn <a href="#command_menu">command_menu</a>() -&gt; <a href="sigil-widgets.html#commandmenubuilder">CommandMenuBuilder</a></pre>


### `container`

<pre>pub fn <a href="#container">container</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `context_menu`

<pre>pub fn <a href="#context_menu">context_menu</a>(target: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, items: impl Fn() -&gt; Vec&lt;<a href="sigil-widgets.html#menuitem">MenuItem</a>&gt; + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `date_field`

<pre>pub fn <a href="#date_field">date_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="sigil-widgets.html#date">Date</a>&gt;) -&gt; <a href="sigil-widgets.html#datefieldbuilder">DateFieldBuilder</a></pre>


### `derived`

<pre>pub fn <a href="#derived">derived</a>&lt;T, F&gt;(f: F) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt; where T: Clone + PartialEq + 'static, F: FnMut() -&gt; T + 'static</pre>


### `diff`

<pre>pub fn <a href="#diff">diff</a>(patch: impl <a href="sigil-widgets.html#diffsource">DiffSource</a> + 'static) -&gt; <a href="sigil-widgets.html#diffbuilder">DiffBuilder</a></pre>


### `divider_horizontal`

<pre>pub fn <a href="#divider_horizontal">divider_horizontal</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `divider_vertical`

<pre>pub const fn <a href="#divider_vertical">divider_vertical</a>() -&gt; <a href="sigil-widgets.html#verticaldivider">VerticalDivider</a></pre>


### `dot`

<pre>pub fn <a href="#dot">dot</a>(tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="sigil-widgets.html#dotbuilder">DotBuilder</a></pre>


### `drag_ghost`

<pre>pub fn <a href="#drag_ghost">drag_ghost</a>&lt;T, N&gt;(state: <a href="sigil-widgets.html#dragstate">DragState</a>&lt;T&gt;, render: impl Fn(&amp;T) -&gt; N + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where T: Clone + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `dropdown`

<pre>pub fn <a href="#dropdown">dropdown</a>(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, options: Vec&lt;String&gt;) -&gt; <a href="sigil-widgets.html#dropdownbuilder">DropdownBuilder</a></pre>


### `effect`

<pre>pub fn <a href="#effect">effect</a>&lt;F: FnMut() + 'static&gt;(f: F)</pre>


### `em`

<pre>pub const fn <a href="#em">em</a>(count: f32) -&gt; <a href="sigil-layout.html#textlength">TextLength</a></pre>


### `empty_state`

<pre>pub fn <a href="#empty_state">empty_state</a>(title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, body: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `field`

<pre>pub fn <a href="#field">field</a>&lt;C: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static&gt;(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, control: C) -&gt; <a href="sigil-widgets.html#fieldbuilder">FieldBuilder</a>&lt;C&gt;</pre>


### `for_each`

<pre>pub fn <a href="#for_each">for_each</a>&lt;T, K, N&gt;(items: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;T&gt;&gt;, key: impl Fn(&amp;T) -&gt; K + 'static, render: impl Fn(&amp;T) -&gt; N + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where T: 'static, K: Clone + Eq + Hash + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `fragment`

<pre>pub fn <a href="#fragment">fragment</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `fuzzy_score`

<pre>pub fn <a href="#fuzzy_score">fuzzy_score</a>(query: &amp;str, text: &amp;str) -&gt; Option&lt;<a href="sigil-widgets.html#fuzzymatch">FuzzyMatch</a>&gt;</pre>


### `gauge`

<pre>pub fn <a href="#gauge">gauge</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; <a href="sigil-widgets.html#gaugebuilder">GaugeBuilder</a></pre>


### `heatmap`

<pre>pub fn <a href="#heatmap">heatmap</a>&lt;F: Fn(usize, usize) -&gt; <a href="sigil-style.html#rgba8">Rgba8</a>&gt;(grid_w: usize, grid_h: usize, color: F) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `highlight`

<pre>pub fn <a href="#highlight">highlight</a>(language: &amp;str, source: &amp;str) -&gt; Vec&lt;Vec&lt;<a href="sigil-widgets.html#highlight">Highlight</a>&gt;&gt;</pre>


### `hint_bar`

<pre>pub fn <a href="#hint_bar">hint_bar</a>(keys: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, right: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `hint_overlay`

<pre>pub fn <a href="#hint_overlay">hint_overlay</a>(hints: &amp;[<a href="sigil-widgets.html#hintentry">HintEntry</a>&lt;'_&gt;]) -&gt; <a href="sigil-widgets.html#hintoverlaybuilder">HintOverlayBuilder</a></pre>


### `hl_spans`

<pre>pub fn <a href="#hl_spans">hl_spans</a>(language: &amp;str, source: &amp;str) -&gt; Vec&lt;Vec&lt;<a href="sigil-widgets.html#hlspan">HlSpan</a>&gt;&gt;</pre>


### `html`

<pre>pub fn <a href="#html">html</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `icon`

<pre>pub fn <a href="#icon-1">icon</a>(glyph: impl <a href="sigil-widgets.html#intoiconsource">IntoIconSource</a>) -&gt; <a href="sigil-widgets.html#iconbuilder">IconBuilder</a></pre>


### `icon_button`

<pre>pub fn <a href="#icon_button">icon_button</a>(glyph: impl Into&lt;<a href="sigil-widgets.html#buttonglyph">ButtonGlyph</a>&gt;, activate: impl Fn() + 'static) -&gt; <a href="sigil-widgets.html#buttonbuilder">ButtonBuilder</a></pre>


### `image`

<pre>pub fn <a href="#image">image</a>(source: <a href="sigil-kernel.html#imagesource">ImageSource</a>) -&gt; <a href="sigil-kernel.html#imagebuilder">ImageBuilder</a></pre>


### `image_bytes`

<pre>pub fn <a href="#image_bytes">image_bytes</a>(encoded: &amp;[u8]) -&gt; Option&lt;<a href="sigil-kernel.html#imagebuilder">ImageBuilder</a>&gt;</pre>


### `image_rgba`

<pre>pub fn <a href="#image_rgba">image_rgba</a>(width: u32, height: u32, rgba: Vec&lt;u8&gt;) -&gt; Option&lt;<a href="sigil-kernel.html#imagebuilder">ImageBuilder</a>&gt;</pre>


### `interactive`

<pre>pub fn <a href="#interactive">interactive</a>(inner: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>) -&gt; <a href="sigil-widgets.html#interactivebuilder">InteractiveBuilder</a></pre>


### `interval`

<pre>pub fn <a href="#interval">interval</a>(period: Duration, callback: impl Fn() + 'static) -&gt; <a href="sigil-reactive.html#intervalid">IntervalId</a></pre>


### `kv_grid`

<pre>pub fn <a href="#kv_grid">kv_grid</a>&lt;K, V&gt;(rows: impl IntoIterator&lt;Item = (K, V)&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where K: Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, V: Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</pre>


### `kv_row`

<pre>pub fn <a href="#kv_row">kv_row</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, value: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `list_row`

<pre>pub fn <a href="#list_row">list_row</a>(selected: bool, title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, detail: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `list_row_with`

<pre>pub fn <a href="#list_row_with">list_row_with</a>(selected: bool, title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, slots: <a href="sigil-widgets.html#listrowslots">ListRowSlots</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `markdown`

<pre>pub fn <a href="#markdown">markdown</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `markdown_edit`

<pre>pub const fn <a href="#markdown_edit">markdown_edit</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-widgets.html#markdowneditbuilder">MarkdownEditBuilder</a></pre>


### `markdown_inline`

<pre>pub fn <a href="#markdown_inline">markdown_inline</a>(source: &amp;str) -&gt; <a href="sigil-kernel.html#textbuilder">TextBuilder</a></pre>


### `match_status`

<pre>pub fn <a href="#match_status">match_status</a>&lt;T: 'static, E: 'static&gt;(status: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;<a href="sigil-tasks.html#status">Status</a>&lt;T, E&gt;&gt;) -&gt; <a href="sigil-kernel.html#statusview">StatusView</a>&lt;T, E&gt;</pre>


### `menu`

<pre>pub fn <a href="#menu">menu</a>(open: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="sigil-widgets.html#menubuilder">MenuBuilder</a></pre>


### `menu_bar`

<pre>pub fn <a href="#menu_bar">menu_bar</a>() -&gt; <a href="sigil-widgets.html#menubarbuilder">MenuBarBuilder</a></pre>


### `mermaid`

<pre>pub fn <a href="#mermaid">mermaid</a>(source: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `modal`

<pre>pub fn <a href="#modal">modal</a>(show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="sigil-widgets.html#modalbuilder">ModalBuilder</a></pre>


### `node`

<pre>pub fn <a href="#node">node</a>(child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Box&lt;dyn <a href="sigil-kernel.html#intonode">IntoNode</a>&gt;</pre>


### `notice_band`

<pre>pub fn <a href="#notice_band">notice_band</a>(body: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `number_field`

<pre>pub fn <a href="#number_field">number_field</a>&lt;T: <a href="sigil-widgets.html#numerickind">NumericKind</a>&gt;(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;, range: RangeInclusive&lt;T&gt;) -&gt; <a href="sigil-widgets.html#numberfieldbuilder">NumberFieldBuilder</a>&lt;T&gt;</pre>


### `open`

<pre>pub fn <a href="#open">open</a>(target: impl AsRef&lt;OsStr&gt;) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;()&gt;</pre>


### `panel`

<pre>pub fn <a href="#panel">panel</a>() -&gt; <a href="sigil-widgets.html#panelbuilder">PanelBuilder</a></pre>


### `path`

<pre>pub const fn <a href="#path">path</a>(width: u32, height: u32) -&gt; <a href="sigil-widgets.html#pathnode">PathNode</a></pre>


### `progress`

<pre>pub fn <a href="#progress">progress</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, max: f32) -&gt; <a href="sigil-widgets.html#progressbuilder">ProgressBuilder</a></pre>


### `radio_group`

<pre>pub fn <a href="#radio_group">radio_group</a>(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-widgets.html#radiogroupbuilder">RadioGroupBuilder</a></pre>


### `request_focus`

<pre>pub fn <a href="#request_focus">request_focus</a>(id: <a href="sigil-kernel.html#nodeid">NodeId</a>)</pre>


### `row`

<pre>pub fn <a href="#row">row</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `rows`

<pre>pub const fn <a href="#rows">rows</a>(count: f32) -&gt; <a href="sigil-layout.html#textlength">TextLength</a></pre>


### `run`

<pre>pub fn <a href="#run">run</a>&lt;C, F, N&gt;(config: C, app: F) -&gt; <a href="sigil.html#appresult">AppResult</a> where C: Into&lt;<a href="sigil.html#appconfig">AppConfig</a>&gt;, F: Fn() -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a></pre>


### `scroll`

<pre>pub fn <a href="#scroll">scroll</a>(content: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>) -&gt; <a href="sigil-widgets.html#scrollbuilder">ScrollBuilder</a></pre>


### `section_label`

<pre>pub fn <a href="#section_label">section_label</a>(title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `segmented`

<pre>pub fn <a href="#segmented">segmented</a>(active: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, labels: Vec&lt;String&gt;) -&gt; <a href="sigil-widgets.html#segmentedbuilder">SegmentedBuilder</a></pre>


### `set_theme`

<pre>pub fn <a href="#set_theme">set_theme</a>(theme: <a href="sigil-style.html#theme">Theme</a>)</pre>


### `sheet`

<pre>pub fn <a href="#sheet">sheet</a>(show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="sigil-widgets.html#modalbuilder">ModalBuilder</a></pre>


### `show`

<pre>pub fn <a href="#show">show</a>&lt;A, B&gt;(cond: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;, then: impl Fn() -&gt; A + 'static, otherwise: impl Fn() -&gt; B + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where A: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, B: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `sidebar`

<pre>pub fn <a href="#sidebar">sidebar</a>(child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="sigil-widgets.html#sidebarbuilder">SidebarBuilder</a></pre>


### `signal`

<pre>pub fn <a href="#signal-1">signal</a>&lt;T: 'static&gt;(initial: T) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;</pre>


### `signal_writer`

<pre>pub fn <a href="#signal_writer">signal_writer</a>&lt;T&gt;(signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;) -&gt; <a href="sigil-tasks.html#signalwriter">SignalWriter</a>&lt;T&gt; where T: Send + 'static</pre>


### `slider`

<pre>pub fn <a href="#slider">slider</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, range: RangeInclusive&lt;f32&gt;) -&gt; <a href="sigil-widgets.html#sliderbuilder">SliderBuilder</a></pre>


### `spacer`

<pre>pub fn <a href="#spacer">spacer</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `sparkline`

<pre>pub fn <a href="#sparkline">sparkline</a>(values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; <a href="sigil-widgets.html#sparklinebuilder">SparklineBuilder</a></pre>


### `spawn`

<pre>pub fn <a href="#spawn">spawn</a>&lt;T, E, F&gt;(future: F) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="sigil-tasks.html#status">Status</a>&lt;T, E&gt;&gt; where T: Send + 'static, E: Send + 'static, F: Future&lt;Output = Result&lt;T, E&gt;&gt; + Send + 'static</pre>


### `spinner`

<pre>pub fn <a href="#spinner">spinner</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `splitter`

<pre>pub fn <a href="#splitter">splitter</a>(orientation: <a href="sigil-widgets.html#orientation">Orientation</a>) -&gt; <a href="sigil-widgets.html#splitterbuilder">SplitterBuilder</a></pre>


### `stat`

<pre>pub fn <a href="#stat">stat</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, value: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-widgets.html#statbuilder">StatBuilder</a></pre>


### `status_bar`

<pre>pub fn <a href="#status_bar">status_bar</a>&lt;I&gt;(children: I) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where I: IntoIterator, &lt;I as IntoIterator&gt;::Item: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `surface`

<pre>pub fn <a href="#surface">surface</a>(key: u64) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `switch`

<pre>pub fn <a href="#switch">switch</a>&lt;K: Clone + PartialEq + 'static&gt;(key: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;K&gt;) -&gt; <a href="sigil-kernel.html#switchbuilder">SwitchBuilder</a>&lt;K&gt;</pre>


### `table`

<pre>pub fn <a href="#table">table</a>&lt;Row: 'static&gt;(state: <a href="sigil-widgets.html#tablestate">TableState</a>&lt;Row&gt;) -&gt; <a href="sigil-widgets.html#tablebuilder">TableBuilder</a>&lt;Row&gt;</pre>


### `tabs`

<pre>pub fn <a href="#tabs">tabs</a>() -&gt; <a href="sigil-widgets.html#tabsbuilder">TabsBuilder</a></pre>


### `task`

<pre>pub fn <a href="#task-1">task</a>&lt;T, E, F, Fut&gt;(make: F) -&gt; <a href="sigil-tasks.html#task">Task</a>&lt;T, E&gt; where T: Send + 'static, E: Send + 'static, F: Fn() -&gt; Fut + 'static, Fut: Future&lt;Output = Result&lt;T, E&gt;&gt; + Send + 'static</pre>


### `task_with`

<pre>pub fn <a href="#task_with">task_with</a>&lt;A, T, E, F, Fut&gt;(make: F) -&gt; <a href="sigil-tasks.html#task">Task</a>&lt;T, E, A&gt; where A: 'static, T: Send + 'static, E: Send + 'static, F: Fn(A) -&gt; Fut + 'static, Fut: Future&lt;Output = Result&lt;T, E&gt;&gt; + Send + 'static</pre>


### `terminal`

<pre>pub fn <a href="#terminal">terminal</a>(lines: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;String&gt;&gt;, on_input: impl Fn(String) + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `text`

<pre>pub fn <a href="#text">text</a>(source: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#textbuilder">TextBuilder</a></pre>


### `text_edit`

<pre>pub fn <a href="#text_edit">text_edit</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-widgets.html#texteditbuilder">TextEditBuilder</a></pre>


### `text_field`

<pre>pub fn <a href="#text_field">text_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-widgets.html#textfieldbuilder">TextFieldBuilder</a></pre>


### `toast`

<pre>pub fn <a href="#toast">toast</a>(message: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;String&gt;&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `toast_queue`

<pre>pub fn <a href="#toast_queue">toast_queue</a>(queue: <a href="sigil-widgets.html#toastqueue">ToastQueue</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `toggle`

<pre>pub fn <a href="#toggle">toggle</a>(on: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="sigil-widgets.html#togglebuilder">ToggleBuilder</a></pre>


### `tooltip`

<pre>pub fn <a href="#tooltip">tooltip</a>(label: impl Into&lt;String&gt;) -&gt; <a href="sigil-widgets.html#tooltipbuilder">TooltipBuilder</a></pre>


### `tree`

<pre>pub fn <a href="#tree">tree</a>&lt;K: <a href="sigil-widgets.html#treekey">TreeKey</a>&gt;(nodes: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;<a href="sigil-widgets.html#treenode">TreeNode</a>&lt;K&gt;&gt;&gt;) -&gt; <a href="sigil-widgets.html#treebuilder">TreeBuilder</a>&lt;K&gt;</pre>


### `virtual_list`

<pre>pub fn <a href="#virtual_list">virtual_list</a>&lt;T, R, N, I&gt;(items: I, render: R) -&gt; <a href="sigil-widgets.html#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt; where T: 'static, R: Fn(usize, &amp;T) -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, I: <a href="sigil-widgets.html#intolist">IntoList</a>&lt;T, R&gt;</pre>


### `watch_theme`

<pre>pub fn <a href="#watch_theme">watch_theme</a>(path: impl Into&lt;PathBuf&gt;) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;<a href="sigil-app.html#themewatcher">ThemeWatcher</a>&gt;</pre>


## Type aliases

### `AppResult`

<pre>pub type <a href="#appresult">AppResult</a>&lt;T&gt; = Result&lt;T, <a href="sigil.html#error">Error</a>&gt;</pre>


## Re-exports

### `Action`


### `Key`


### `ModifiersState`


### `Role`


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
