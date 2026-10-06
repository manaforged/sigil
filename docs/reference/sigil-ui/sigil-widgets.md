# Module `sigil::widgets`

| Item | Kind | Description |
| --- | --- | --- |
| [`ButtonBuilder`](#buttonbuilder) | struct |  |
| [`CardBuilder`](#cardbuilder) | struct |  |
| [`ChartBuilder`](#chartbuilder) | struct |  |
| [`CheckboxBuilder`](#checkboxbuilder) | struct |  |
| [`ChipBuilder`](#chipbuilder) | struct |  |
| [`CodeEditBuilder`](#codeeditbuilder) | struct |  |
| [`ColorFieldBuilder`](#colorfieldbuilder) | struct |  |
| [`CommandMenuBuilder`](#commandmenubuilder) | struct |  |
| [`DateFieldBuilder`](#datefieldbuilder) | struct |  |
| [`DiffBuilder`](#diffbuilder) | struct |  |
| [`DotBuilder`](#dotbuilder) | struct |  |
| [`DropdownBuilder`](#dropdownbuilder) | struct |  |
| [`GaugeBuilder`](#gaugebuilder) | struct |  |
| [`HintOverlayBuilder`](#hintoverlaybuilder) | struct |  |
| [`IconBuilder`](#iconbuilder) | struct |  |
| [`InteractiveBuilder`](#interactivebuilder) | struct |  |
| [`MarkdownEditBuilder`](#markdowneditbuilder) | struct |  |
| [`MenuBarBuilder`](#menubarbuilder) | struct |  |
| [`MenuBuilder`](#menubuilder) | struct |  |
| [`ModalBuilder`](#modalbuilder) | struct |  |
| [`PaneBuilder`](#panebuilder) | struct |  |
| [`PanelBuilder`](#panelbuilder) | struct |  |
| [`PathNode`](#pathnode) | struct |  |
| [`ProgressBuilder`](#progressbuilder) | struct |  |
| [`RadioGroupBuilder`](#radiogroupbuilder) | struct |  |
| [`ScrollBuilder`](#scrollbuilder) | struct |  |
| [`SegmentedBuilder`](#segmentedbuilder) | struct |  |
| [`SidebarBuilder`](#sidebarbuilder) | struct |  |
| [`SliderBuilder`](#sliderbuilder) | struct |  |
| [`SparklineBuilder`](#sparklinebuilder) | struct |  |
| [`SplitterBuilder`](#splitterbuilder) | struct |  |
| [`StatBuilder`](#statbuilder) | struct |  |
| [`TabsBuilder`](#tabsbuilder) | struct |  |
| [`TextEditBuilder`](#texteditbuilder) | struct |  |
| [`TextFieldBuilder`](#textfieldbuilder) | struct |  |
| [`ToggleBuilder`](#togglebuilder) | struct |  |
| [`TooltipBuilder`](#tooltipbuilder) | struct |  |
| [`VerticalDivider`](#verticaldivider) | struct |  |
| [`FieldBuilder`](#fieldbuilder) | struct |  |
| [`TreeBuilder`](#treebuilder) | struct |  |
| [`TableBuilder`](#tablebuilder) | struct |  |
| [`VirtualListBuilder`](#virtuallistbuilder) | struct |  |
| [`NumberFieldBuilder`](#numberfieldbuilder) | struct |  |
| [`Combobox`](#combobox) | struct |  |
| [`Date`](#date) | struct |  |
| [`DragState`](#dragstate) | struct |  |
| [`ListRowSlots`](#listrowslots) | struct |  |
| [`MenuItem`](#menuitem) | struct |  |
| [`SortKey`](#sortkey) | struct |  |
| [`TableColumn`](#tablecolumn) | struct |  |
| [`TableState`](#tablestate) | struct |  |
| [`ToastQueue`](#toastqueue) | struct |  |
| [`ColumnLayout`](#columnlayout) | struct |  |
| [`DiagSpan`](#diagspan) | struct |  |
| [`FuzzyMatch`](#fuzzymatch) | struct |  |
| [`Highlight`](#highlight) | struct |  |
| [`HlSpan`](#hlspan) | struct |  |
| [`Hunk`](#hunk) | struct |  |
| [`IconSource`](#iconsource) | struct |  |
| [`Line`](#line) | struct |  |
| [`LineId`](#lineid) | struct |  |
| [`ListColumn`](#listcolumn) | struct |  |
| [`Patch`](#patch) | struct |  |
| [`Selection`](#selection) | struct |  |
| [`SelectionState`](#selectionstate) | struct |  |
| [`SubmenuBuilder`](#submenubuilder) | struct |  |
| [`TextEditState`](#texteditstate) | struct |  |
| [`ToastEntry`](#toastentry) | struct |  |
| [`TreeNode`](#treenode) | struct |  |
| [`TreeRow`](#treerow) | struct |  |
| [`ButtonGlyph`](#buttonglyph) | enum |  |
| [`ColWidth`](#colwidth) | enum |  |
| [`Density`](#density) | enum |  |
| [`DiffView`](#diffview) | enum |  |
| [`Disclosure`](#disclosure) | enum |  |
| [`Orientation`](#orientation) | enum |  |
| [`SelectionMode`](#selectionmode) | enum |  |
| [`Side`](#side) | enum |  |
| [`SortDir`](#sortdir) | enum |  |
| [`SubmitKey`](#submitkey) | enum |  |
| [`PathColor`](#pathcolor) | enum |  |
| [`Axis`](#axis) | enum |  |
| [`ChangeKind`](#changekind) | enum |  |
| [`ChartMark`](#chartmark) | enum |  |
| [`ColAlign`](#colalign) | enum |  |
| [`DiagSeverity`](#diagseverity) | enum |  |
| [`FocusCaret`](#focuscaret) | enum |  |
| [`LineKind`](#linekind) | enum |  |
| [`PathCmd`](#pathcmd) | enum |  |
| [`PathFill`](#pathfill) | enum |  |
| [`RowTone`](#rowtone) | enum |  |
| [`ScrollAnchor`](#scrollanchor) | enum |  |
| [`SparkFill`](#sparkfill) | enum |  |
| [`SparkMark`](#sparkmark) | enum |  |
| [`ToastLevel`](#toastlevel) | enum |  |
| [`DiffSource`](#diffsource) | trait |  |
| [`DragDropExt`](#dragdropext) | trait |  |
| [`FieldControl`](#fieldcontrol) | trait |  |
| [`Identifiable`](#identifiable) | trait |  |
| [`IntoIconSource`](#intoiconsource) | trait |  |
| [`IntoList`](#intolist) | trait |  |
| [`NumericKind`](#numerickind) | trait |  |
| [`RowRender`](#rowrender) | trait |  |
| [`TreeKey`](#treekey) | trait |  |
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
| [`combobox`](#combobox-1) | fn |  |
| [`command_menu`](#command_menu) | fn |  |
| [`context_menu`](#context_menu) | fn |  |
| [`date_field`](#date_field) | fn |  |
| [`diff`](#diff) | fn |  |
| [`divider_horizontal`](#divider_horizontal) | fn |  |
| [`divider_vertical`](#divider_vertical) | fn |  |
| [`dot`](#dot) | fn |  |
| [`drag_ghost`](#drag_ghost) | fn |  |
| [`dropdown`](#dropdown) | fn |  |
| [`empty_state`](#empty_state) | fn |  |
| [`field`](#field) | fn |  |
| [`fuzzy_score`](#fuzzy_score) | fn |  |
| [`gauge`](#gauge) | fn |  |
| [`heatmap`](#heatmap) | fn |  |
| [`highlight`](#highlight-1) | fn |  |
| [`hint_bar`](#hint_bar) | fn |  |
| [`hint_overlay`](#hint_overlay) | fn |  |
| [`hl_spans`](#hl_spans) | fn |  |
| [`html`](#html) | fn |  |
| [`icon`](#icon) | fn |  |
| [`icon_button`](#icon_button) | fn |  |
| [`icon_svg`](#icon_svg) | fn |  |
| [`image_bytes`](#image_bytes) | fn |  |
| [`image_rgba`](#image_rgba) | fn |  |
| [`interactive`](#interactive) | fn |  |
| [`kv_grid`](#kv_grid) | fn |  |
| [`kv_row`](#kv_row) | fn |  |
| [`language_for_path`](#language_for_path) | fn |  |
| [`list_row`](#list_row) | fn |  |
| [`list_row_with`](#list_row_with) | fn |  |
| [`markdown`](#markdown) | fn |  |
| [`markdown_edit`](#markdown_edit) | fn |  |
| [`markdown_inline`](#markdown_inline) | fn |  |
| [`menu`](#menu) | fn |  |
| [`menu_bar`](#menu_bar) | fn |  |
| [`mermaid`](#mermaid) | fn |  |
| [`modal`](#modal) | fn |  |
| [`notice_band`](#notice_band) | fn |  |
| [`number_field`](#number_field) | fn |  |
| [`panel`](#panel) | fn |  |
| [`parse_hex`](#parse_hex) | fn |  |
| [`path`](#path) | fn |  |
| [`progress`](#progress) | fn |  |
| [`radio_group`](#radio_group) | fn |  |
| [`scroll`](#scroll) | fn |  |
| [`section_label`](#section_label) | fn |  |
| [`segmented`](#segmented) | fn |  |
| [`sheet`](#sheet) | fn |  |
| [`sidebar`](#sidebar) | fn |  |
| [`slider`](#slider) | fn |  |
| [`spacer`](#spacer) | fn |  |
| [`sparkline`](#sparkline) | fn |  |
| [`spinner`](#spinner) | fn |  |
| [`spinner_frame`](#spinner_frame) | fn |  |
| [`splitter`](#splitter) | fn |  |
| [`stat`](#stat) | fn |  |
| [`status_bar`](#status_bar) | fn |  |
| [`surface`](#surface) | fn |  |
| [`table`](#table) | fn |  |
| [`tabs`](#tabs) | fn |  |
| [`terminal`](#terminal) | fn |  |
| [`text_edit`](#text_edit) | fn |  |
| [`text_field`](#text_field) | fn |  |
| [`text_runs`](#text_runs) | fn |  |
| [`to_hex`](#to_hex) | fn |  |
| [`toast`](#toast) | fn |  |
| [`toast_queue`](#toast_queue) | fn |  |
| [`toggle`](#toggle) | fn |  |
| [`tooltip`](#tooltip) | fn |  |
| [`tree`](#tree) | fn |  |
| [`virtual_list`](#virtual_list) | fn |  |
| [`virtual_list_bind`](#virtual_list_bind) | fn |  |
| [`HintEntry`](#hintentry) | type |  |
| [`DRAG_THRESHOLD_PX`](#drag_threshold_px) | const |  |
| [`DROP_TARGET_CLASS`](#drop_target_class) | const |  |
| [`MAX_VISIBLE`](#max_visible) | const |  |
| [`MIN_COL_WIDTH`](#min_col_width) | const |  |
| [`TOAST_TTL`](#toast_ttl) | const |  |

## Structs

### `ButtonBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>accessible(self, name: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>activate(self, activate: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>const destructive(self) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const ghost(self) -&gt; Self</code> |  |
| <code>icon(self, icon: <a href="sigil-icons.html#icon">Icon</a>) -&gt; Self</code> |  |
| <code>icon_svg(self, svg: impl Into&lt;Rc&lt;str&gt;&gt;) -&gt; Self</code> |  |
| <code>const neutral(self) -&gt; Self</code> |  |
| <code>new(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>const outline(self) -&gt; Self</code> |  |
| <code>const pending(self, pending: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const primary(self) -&gt; Self</code> |  |
| <code>size(self, size: <a href="sigil-style.html#controlsize">ControlSize</a>) -&gt; Self</code> |  |
| <code>const soft(self) -&gt; Self</code> |  |
| <code>const solid(self) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `CardBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>activate(self, activate: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>into_node_builder(self) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |
| <code>selected(self, selected: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `ChartBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const bar(self) -&gt; Self</code> |  |
| <code>candles(rows: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;(f32, f32, f32, f32)&gt;&gt;) -&gt; Self</code> |  |
| <code>const crosshair(self) -&gt; Self</code> |  |
| <code>labels(self, labels: impl IntoIterator&lt;Item = impl Into&lt;String&gt;&gt;) -&gt; Self</code> |  |
| <code>const line(self) -&gt; Self</code> |  |
| <code>series(self, name: impl Into&lt;String&gt;, values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>const x_range(self, range: <a href="sigil-reactive.html#signal">Signal</a>&lt;Range&lt;f32&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `CheckboxBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>change(self, change: impl Fn(bool) + 'static) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const indeterminate(self, partial: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>new(checked: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `ChipBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>activate(self, activate: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>const dot(self) -&gt; Self</code> |  |
| <code>into_node_builder(self) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |
| <code>layout(self, layout: <a href="sigil-layout.html#layoutstyle">LayoutStyle</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |
| <code>leading(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>tone(self, tone: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;<a href="sigil-style.html#tone">Tone</a>&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `CodeEditBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const diagnostics(self, spans: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;<a href="#diagspan">DiagSpan</a>&gt;&gt;) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const gutter(self, marks: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;Option&lt;<a href="#changekind">ChangeKind</a>&gt;&gt;&gt;) -&gt; Self</code> |  |
| <code>const highlights(self, spans: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;Vec&lt;<a href="#hlspan">HlSpan</a>&gt;&gt;&gt;) -&gt; Self</code> |  |
| <code>key(self, f: impl Fn(&amp;mut <a href="sigil-events.html#keyevent">KeyEvent</a>) + 'static) -&gt; Self</code> |  |
| <code>const row_height(self, h: f32) -&gt; Self</code> |  |
| <code>const selection(self, anchor: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;(usize, usize)&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `ColorFieldBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>invalid_when(self, invalid: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="#fieldcontrol">FieldControl</a></code>


### `CommandMenuBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>cancel(self, cb: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>disabled(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>item(self, label: impl Into&lt;String&gt;, action: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>placeholder(self, text: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>submenu(self, label: impl Into&lt;String&gt;, build: impl FnOnce(<a href="#submenubuilder">SubmenuBuilder</a>) -&gt; <a href="#submenubuilder">SubmenuBuilder</a>) -&gt; Self</code> |  |
| <code>submit(self, cb: impl Fn(&amp;str) + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>, <code>Default</code>


### `DateFieldBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>invalid_when(self, invalid: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>text_field(self) -&gt; <a href="#textfieldbuilder">TextFieldBuilder</a></code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `DiffBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>change(self, on_change: impl Fn(<a href="#diffview">DiffView</a>) + 'static) -&gt; Self</code> |  |
| <code>const max_rows(self, rows: usize) -&gt; Self</code> |  |
| <code>path(self, path: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const select(self, selection: <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="#selection">Selection</a>&gt;) -&gt; Self</code> |  |
| <code>status(self, status: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const view(self, view: <a href="#diffview">DiffView</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `DotBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>const size(self, size: <a href="sigil-style.html#controlsize">ControlSize</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `DropdownBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>change(self, cb: impl Fn(usize) + 'static) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>new(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, options: Vec&lt;String&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `GaugeBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>label(self, label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>const max(self, max: f32) -&gt; Self</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>const warn(self, threshold: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `HintOverlayBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>entry(self, chord: impl Into&lt;String&gt;, description: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>new(hints: &amp;[<a href="#hintentry">HintEntry</a>&lt;'_&gt;]) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `IconBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>label(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>new(glyph: impl <a href="#intoiconsource">IntoIconSource</a>) -&gt; Self</code> |  |
| <code>const px(self, px: f32) -&gt; Self</code> |  |
| <code>const size(self, size: <a href="sigil-style.html#controlsize">ControlSize</a>) -&gt; Self</code> |  |
| <code>const strong(self) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `InteractiveBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const focusable(self, yes: bool) -&gt; Self</code> |  |
| <code>into_node_builder(self) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `MarkdownEditBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `MenuBarBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const fill(self) -&gt; Self</code> |  |
| <code>menu(self, label: impl Into&lt;String&gt;, items: impl IntoIterator&lt;Item = <a href="#menuitem">MenuItem</a>&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `MenuBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const anchor(self, x: f32, y: f32) -&gt; Self</code> |  |
| <code>const host(self, owner: <a href="sigil-reactive.html#ownerid">OwnerId</a>) -&gt; Self</code> |  |
| <code>item(self, item: <a href="#menuitem">MenuItem</a>) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>new(open: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>separator(self) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `ModalBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>actions(self, cancel: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, confirm: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>child(self, child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>const close_button(self, enabled: bool) -&gt; Self</code> |  |
| <code>const dismiss_on_backdrop(self, enabled: bool) -&gt; Self</code> |  |
| <code>const escape_closes(self, enabled: bool) -&gt; Self</code> |  |
| <code>footer(self, action: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>new(show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const show(self, show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const side(self, side: <a href="#side">Side</a>) -&gt; Self</code> |  |
| <code>title(self, title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>const trap(self, enabled: bool) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `PaneBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>commit(self) -&gt; <a href="#splitterbuilder">SplitterBuilder</a></code> |  |
| <code>const grow(self, factor: f32) -&gt; Self</code> |  |
| <code>const initial(self, px: f32) -&gt; Self</code> |  |
| <code>const max(self, px: f32) -&gt; Self</code> |  |
| <code>const min(self, px: f32) -&gt; Self</code> |  |
| <code>pane(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `PanelBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>child(self, child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>elevation(self, tier: u32) -&gt; Self</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>const padding(self, px: f32) -&gt; Self</code> |  |
| <code>title(self, title: impl Into&lt;String&gt;) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>elevation: u32</code> |  |
| <code>padding: f32</code> |  |
| <code>title: Option&lt;String&gt;</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>, <code>Default</code>


### `PathNode`

**Methods**

| Method | Description |
| --- | --- |
| <code>close(self) -&gt; Self</code> |  |
| <code>cubic_to(self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -&gt; Self</code> |  |
| <code>fill(self, color: impl Into&lt;<a href="#pathcolor">PathColor</a>&gt;) -&gt; Self</code> |  |
| <code>fill_linear(self, start: (f32, f32), end: (f32, f32), stops: Vec&lt;(f32, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;) -&gt; Self</code> |  |
| <code>fill_radial(self, center: (f32, f32), radius: f32, stops: Vec&lt;(f32, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;) -&gt; Self</code> |  |
| <code>fit(self) -&gt; Self</code> |  |
| <code>fit_size(self, logical: f32) -&gt; Self</code> |  |
| <code>layout(self, style: <a href="sigil-layout.html#layoutstyle">LayoutStyle</a>) -&gt; Self</code> |  |
| <code>line_to(self, x: f32, y: f32) -&gt; Self</code> |  |
| <code>move_to(self, x: f32, y: f32) -&gt; Self</code> |  |
| <code>const non_scaling_stroke(self) -&gt; Self</code> |  |
| <code>quad_to(self, cx: f32, cy: f32, x: f32, y: f32) -&gt; Self</code> |  |
| <code>stroke(self, color: impl Into&lt;<a href="#pathcolor">PathColor</a>&gt;, width: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `ProgressBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const indeterminate(self) -&gt; Self</code> |  |
| <code>label(self, name: impl Into&lt;<a href="sigil-kernel.html#accessiblename">AccessibleName</a>&gt;) -&gt; Self</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, max: f32) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>indeterminate: bool</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `RadioGroupBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>label(self, name: impl Into&lt;<a href="sigil-kernel.html#accessiblename">AccessibleName</a>&gt;) -&gt; Self</code> |  |
| <code>const labelled_by(self, node: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; Self</code> |  |
| <code>new(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; Self</code> |  |
| <code>option(self, key: impl Into&lt;String&gt;, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `ScrollBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>extent(self, extent: impl <a href="#scroll">IntoScrollExtent</a>) -&gt; Self</code> |  |
| <code>const fill(self) -&gt; Self</code> |  |
| <code>const fit(self) -&gt; Self</code> |  |
| <code>const follow_end(self, on: bool) -&gt; Self</code> |  |
| <code>const scroll_bind(self, scroll_y: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `SegmentedBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>label(self, name: impl Into&lt;String&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `SidebarBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const resizable(self, min: f32, max: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `SliderBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>change(self, change: impl Fn(f32) + 'static) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const glass(self) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const length(self, length: f32) -&gt; Self</code> |  |
| <code>marks(self, marks: impl IntoIterator&lt;Item = impl Into&lt;String&gt;&gt;) -&gt; Self</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, range: RangeInclusive&lt;f32&gt;) -&gt; Self</code> |  |
| <code>const orientation(self, orientation: <a href="#orientation">Orientation</a>) -&gt; Self</code> |  |
| <code>const step(self, step: f32) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `SparklineBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const fill(self, fill: <a href="#sparkfill">SparkFill</a>) -&gt; Self</code> |  |
| <code>const mark(self, mark: <a href="#sparkmark">SparkMark</a>) -&gt; Self</code> |  |
| <code>const range(self, low: f32, high: f32) -&gt; Self</code> |  |
| <code>rows(self, rows: usize) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>tones(self, tones: impl IntoIterator&lt;Item = Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `SplitterBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>child(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, initial_size: f32) -&gt; Self</code> |  |
| <code>child_constrained(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, initial: f32, min: f32, max: f32) -&gt; Self</code> |  |
| <code>max_size(self, idx: usize, px: f32) -&gt; Self</code> |  |
| <code>min_size(self, idx: usize, px: f32) -&gt; Self</code> |  |
| <code>pane(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="#panebuilder">PaneBuilder</a></code> |  |
| <code>const sizes_bind(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; Self</code> |  |
| <code>snap_points(self, handle_idx: usize, points: &amp;[f32]) -&gt; Self</code> |  |
| <code>const snap_threshold(self, threshold: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `StatBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>caption(self, caption: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>delta(self, change: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>into_node_builder(self) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |
| <code>const range(self, low: f32, high: f32) -&gt; Self</code> |  |
| <code>const tone(self, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>trend(self, values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `TabsBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const active(self, active: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>close(self, handler: impl Fn(usize) + 'static) -&gt; Self</code> |  |
| <code>const closeable(self, enabled: bool) -&gt; Self</code> |  |
| <code>disabled(self, indices: &amp;[usize]) -&gt; Self</code> |  |
| <code>into_container(self) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></code> |  |
| <code>const justify_list(self, justify: <a href="sigil-layout.html#justifycontent">JustifyContent</a>) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>labels&lt;S&gt;(self, labels: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;S&gt;&gt;) -&gt; Self where S: Into&lt;String&gt; + 'static</code> |  |
| <code>const line(self) -&gt; Self</code> |  |
| <code>const order(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;usize&gt;&gt;) -&gt; Self</code> |  |
| <code>const pill(self) -&gt; Self</code> |  |
| <code>const reorderable(self, enabled: bool) -&gt; Self</code> |  |
| <code>tab&lt;F, N&gt;(self, label: impl Into&lt;String&gt;, content: F) -&gt; Self where F: FnOnce() -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `TextEditBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const autofocus(self) -&gt; Self</code> |  |
| <code>const caret_position(self, position: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>const caret_rect(self, rect: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;<a href="sigil-geometry.html#rect">Rect</a>&gt;&gt;) -&gt; Self</code> |  |
| <code>caret_visible(&amp;mut self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;</code> |  |
| <code>change(self, f: impl Fn(&amp;str) + 'static) -&gt; Self</code> |  |
| <code>const chrome(self, enabled: bool) -&gt; Self</code> |  |
| <code>clipboard(self, clipboard: impl <a href="sigil-clipboard.html#clipboard">Clipboard</a> + 'static) -&gt; Self</code> |  |
| <code>const content_padding(self, horizontal: f32, vertical: f32) -&gt; Self</code> |  |
| <code>disabled(self, flag: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const focus_when(self, trigger: <a href="sigil-reactive.html#signal">Signal</a>&lt;u64&gt;) -&gt; Self</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; Self</code> |  |
| <code>placeholder(self, text: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>state(self, state: <a href="#texteditstate">TextEditState</a>) -&gt; Self</code> |  |
| <code>submit(self, f: impl Fn(&amp;str) + 'static) -&gt; Self</code> |  |
| <code>const submit_on(self, key: <a href="#submitkey">SubmitKey</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `TextFieldBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>const autofocus(self) -&gt; Self</code> |  |
| <code>blur(self, f: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>caret_visible(&amp;mut self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;</code> |  |
| <code>change(self, f: impl Fn(&amp;str) + 'static) -&gt; Self</code> |  |
| <code>const clearable(self) -&gt; Self</code> |  |
| <code>clipboard(self, clipboard: impl <a href="sigil-clipboard.html#clipboard">Clipboard</a> + 'static) -&gt; Self</code> |  |
| <code>disabled(self, flag: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const focus_caret(self, at: <a href="#focuscaret">FocusCaret</a>) -&gt; Self</code> |  |
| <code>const focus_when(self, trigger: <a href="sigil-reactive.html#signal">Signal</a>&lt;u64&gt;) -&gt; Self</code> |  |
| <code>invalid_when(self, invalid: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>leading(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; Self</code> |  |
| <code>const password(self) -&gt; Self</code> |  |
| <code>placeholder(self, text: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>const reveal(self, show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>scroll_offset(&amp;mut self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;</code> |  |
| <code>submit(self, f: impl Fn(&amp;str) + 'static) -&gt; Self</code> |  |
| <code>trailing(self, node: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>, <code><a href="#fieldcontrol">FieldControl</a></code>


### `ToggleBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>accessible(self, name: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>change(self, change: impl Fn(bool) + 'static) -&gt; Self</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>label(self, label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>new(on: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `TooltipBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>child(self, target: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>const clamp_top(self) -&gt; Self</code> |  |
| <code>const delay(self, ms: u64) -&gt; Self</code> |  |
| <code>new(label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>shortcut(self, shortcut: Option&lt;impl Into&lt;String&gt;&gt;) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="sigil-style.html#theme">Theme</a>) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `VerticalDivider`

**Methods**

| Method | Description |
| --- | --- |
| <code>const height(self, px: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `FieldBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#fieldbuilder">FieldBuilder</a>&lt;C&gt;::error(self, error: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Option&lt;String&gt;&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#fieldbuilder">FieldBuilder</a>&lt;C&gt;::required(self, required: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `TreeBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;::activate(self, activate: impl Fn(&amp;K) + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;::expanded(self, expanded: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;K&gt;&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;::indent(self, cols: f32) -&gt; Self</code> |  |
| <code>pub fn <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;::label(self, label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;::selected(self, selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;K&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `TableBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::density(self, density: <a href="#density">Density</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::empty(self, build: impl Fn() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::framed(self, framed: bool) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::header_height(self, px: f32) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::row_activate(self, activate: impl Fn(&amp;Row) + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::row_height(self, length: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::row_menu(self, build: impl Fn(&amp;Row) -&gt; Vec&lt;<a href="#menuitem">MenuItem</a>&gt; + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::scroll_bind(self, scroll_y: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::selectable_checkboxes(self) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::striped(self) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::viewport_fill(self) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::viewport_height(self, length: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;::viewport_width(self, px: f32) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `VirtualListBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::change(self, f: impl Fn(usize) + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::cursor(self, cursor: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::estimate(self, length: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::follow_end(self, on: bool) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::follow_row(self, target: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::identified(self) -&gt; Self where T: <a href="#identifiable">Identifiable</a></code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::initial_scroll(self, anchor: <a href="#scrollanchor">ScrollAnchor</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::item_height(self, f: impl Fn(usize, &amp;T) -&gt; f32 + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::key_by(self, f: impl Fn(usize, &amp;T) -&gt; u64 + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::keyboard(self, enabled: bool) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::measured(self) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::overscan(self, pages: usize) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::row_height(self, length: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::scroll_bind(self, scroll_y: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::scroll_into_view(self, index: usize) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::scroll_x_bind(self, scroll_x: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, max: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, width: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::selection(self, mode: <a href="#selectionmode">SelectionMode</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::selection_state(&amp;mut self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="#selectionstate">SelectionState</a>&gt;</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::shrink_to_content(self) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::viewport_fill(self) -&gt; Self</code> |  |
| <code>pub fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::viewport_height(self, length: impl <a href="sigil-layout.html#intopx">IntoPx</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::viewport_width(self, px: f32) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::content_height_bind(self, content_height: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::max_scroll_bind(self, max_scroll: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;::scrollbar(self, on: bool) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `NumberFieldBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#numberfieldbuilder">NumberFieldBuilder</a>&lt;T&gt;::disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#numberfieldbuilder">NumberFieldBuilder</a>&lt;T&gt;::invalid_when(self, invalid: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#numberfieldbuilder">NumberFieldBuilder</a>&lt;T&gt;::text_field(self) -&gt; <a href="#textfieldbuilder">TextFieldBuilder</a></code> |  |

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="#fieldcontrol">FieldControl</a></code>


### `Combobox`

**Methods**

| Method | Description |
| --- | --- |
| <code>close(self)</code> |  |
| <code>commit(self, index: usize)</code> |  |
| <code>commit_cursor(self) -&gt; bool</code> |  |
| <code>cursor(self) -&gt; usize</code> |  |
| <code>disabled(self, disabled: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>edit_query(self, text: &amp;str)</code> |  |
| <code>filtered(self) -&gt; Vec&lt;(usize, String)&gt;</code> |  |
| <code>is_open(self) -&gt; bool</code> |  |
| <code>move_cursor(self, delta: i32)</code> |  |
| <code>new(options: Vec&lt;String&gt;) -&gt; Self</code> |  |
| <code>open(self)</code> |  |
| <code>const placeholder(self, text: &amp;'static str) -&gt; Self</code> |  |
| <code>query(self) -&gt; String</code> |  |
| <code>selected(self) -&gt; Option&lt;usize&gt;</code> |  |
| <code>selected_option(self) -&gt; Option&lt;String&gt;</code> |  |
| <code>const selected_signal(self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;usize&gt;&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Copy</code>


### `Date`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(year: i32, month: u8, day: u8) -&gt; Option&lt;Self&gt;</code> |  |
| <code>parse(input: &amp;str) -&gt; Option&lt;Self&gt;</code> |  |
| <code>render(self) -&gt; String</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>day: u8</code> |  |
| <code>month: u8</code> |  |
| <code>year: i32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DragState`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#dragstate">DragState</a>&lt;T&gt;::cancel(&amp;self)</code> |  |
| <code>pub fn <a href="#dragstate">DragState</a>&lt;T&gt;::is_active(&amp;self) -&gt; bool</code> |  |
| <code>pub fn <a href="#dragstate">DragState</a>&lt;T&gt;::new() -&gt; Self</code> |  |
| <code>pub fn <a href="#dragstate">DragState</a>&lt;T&gt;::payload(&amp;self) -&gt; Option&lt;T&gt;</code> |  |
| <code>pub fn <a href="#dragstate">DragState</a>&lt;T&gt;::position(&amp;self) -&gt; (f32, f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>


### `ListRowSlots`

**Fields**

| Field | Description |
| --- | --- |
| <code>activate: Option&lt;Rc&lt;dyn Fn()&gt;&gt;</code> |  |
| <code>badge: Option&lt;(<a href="sigil-kernel.html#textcontent">TextContent</a>, <a href="sigil-style.html#tone">Tone</a>)&gt;</code> |  |
| <code>columns: Vec&lt;<a href="#listcolumn">ListColumn</a>&gt;</code> |  |
| <code>depth: u16</code> |  |
| <code>detail: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>disclosure: <a href="#disclosure">Disclosure</a></code> |  |
| <code>indent: Option&lt;f32&gt;</code> |  |
| <code>leading: Option&lt;Box&lt;dyn <a href="sigil-kernel.html#intonode">IntoNode</a>&gt;&gt;</code> |  |
| <code>marker: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>marker_tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |
| <code>shrink: bool</code> |  |
| <code>status: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>status_tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |
| <code>title_width: Option&lt;f32&gt;</code> |  |
| <code>tone: <a href="#rowtone">RowTone</a></code> |  |

**Trait implementations:** <code>Default</code>


### `MenuItem`

**Methods**

| Method | Description |
| --- | --- |
| <code>activate(self, on_activate: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>const disabled(self, yes: bool) -&gt; Self</code> |  |
| <code>new(label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>shortcut(self, hint: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const submenu(self, yes: bool) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>disabled: bool</code> |  |
| <code>label: String</code> |  |
| <code>shortcut: Option&lt;String&gt;</code> |  |
| <code>submenu_indicator: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `SortKey`

**Fields**

| Field | Description |
| --- | --- |
| <code>column: &amp;'static str</code> |  |
| <code>dir: <a href="#sortdir">SortDir</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TableColumn`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::align(self, align: <a href="#colalign">ColAlign</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::alignment(&amp;self) -&gt; <a href="#colalign">ColAlign</a></code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::cell&lt;N&gt;(self, render: impl Fn(&amp;Row) -&gt; N + 'static) -&gt; Self where N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::filterable(self, matches: impl Fn(&amp;Row, &amp;str) -&gt; bool + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::id(&amp;self) -&gt; &amp;'static str</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::is_numeric(&amp;self) -&gt; bool</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::is_sortable(&amp;self) -&gt; bool</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::min_width(self, px: f32) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::new(id: &amp;'static str, title: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::numeric(self) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::sort_by&lt;K&gt;(self, key: impl Fn(&amp;Row) -&gt; K + 'static) -&gt; Self where K: Ord</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::sortable(self, compare: impl Fn(&amp;Row, &amp;Row) -&gt; Ordering + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::title(&amp;self) -&gt; &amp;str</code> |  |
| <code>pub const fn <a href="#tablecolumn">TableColumn</a>&lt;Row&gt;::width(self, width: <a href="#colwidth">ColWidth</a>) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>


### `TableState`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::clear_column_width(&amp;self, id: &amp;'static str)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::clear_selection(&amp;self)</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::column_layout(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Rc&lt;<a href="#columnlayout">ColumnLayout</a>&gt;&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::column_width(&amp;self, id: &amp;'static str) -&gt; Option&lt;f32&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::columns(&amp;self) -&gt; &amp;[<a href="#tablecolumn">TableColumn</a>&lt;Row&gt;]</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::hide_column(&amp;self, id: &amp;'static str)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::move_column(&amp;self, from: usize, to: usize)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::new(rows: Vec&lt;Row&gt;, columns: Vec&lt;<a href="#tablecolumn">TableColumn</a>&lt;Row&gt;&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::pin_column_width(&amp;self, id: &amp;'static str, px: f32)</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::quick_filter(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::replace_rows(&amp;self, rows: impl Into&lt;Arc&lt;Vec&lt;Row&gt;&gt;&gt;)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::reset_columns(&amp;self)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::row_eq(self, eq: impl Fn(&amp;Row, &amp;Row) -&gt; bool + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::rows(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Arc&lt;Vec&lt;Row&gt;&gt;&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::select_only(&amp;self, row: u32)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::select_range_to(&amp;self, row: u32)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::selected_index(&amp;self) -&gt; Option&lt;u32&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::selected_row(&amp;self) -&gt; Option&lt;Row&gt; where Row: Clone</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::selection(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Rc&lt;HashSet&lt;u32&gt;&gt;&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::show_all_columns(&amp;self)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::show_column(&amp;self, id: &amp;'static str)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::sort(&amp;self, column: &amp;'static str, dir: <a href="#sortdir">SortDir</a>)</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::sort_key(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;<a href="#sortkey">SortKey</a>&gt;&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::toggle_selected(&amp;self, row: u32)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::toggle_sort(&amp;self, column: &amp;'static str)</code> |  |
| <code>pub const fn <a href="#tablestate">TableState</a>&lt;Row&gt;::visible(&amp;self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Rc&lt;Vec&lt;u32&gt;&gt;&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::visible_columns(&amp;self) -&gt; Vec&lt;(usize, &amp;<a href="#tablecolumn">TableColumn</a>&lt;Row&gt;)&gt;</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::cycle_sort(&amp;self)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::reverse_sort(&amp;self)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::select_by(&amp;self, delta: i32)</code> |  |
| <code>pub fn <a href="#tablestate">TableState</a>&lt;Row&gt;::identified(rows: Vec&lt;Row&gt;, columns: Vec&lt;<a href="#tablecolumn">TableColumn</a>&lt;Row&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>


### `ToastQueue`

**Methods**

| Method | Description |
| --- | --- |
| <code>error(self, message: impl Into&lt;String&gt;)</code> |  |
| <code>info(self, message: impl Into&lt;String&gt;)</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>push(self, level: <a href="#toastlevel">ToastLevel</a>, message: impl Into&lt;String&gt;)</code> |  |
| <code>success(self, message: impl Into&lt;String&gt;)</code> |  |
| <code>warning(self, message: impl Into&lt;String&gt;)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>


### `ColumnLayout`

**Fields**

| Field | Description |
| --- | --- |
| <code>hidden: HashSet&lt;&amp;'static str&gt;</code> |  |
| <code>order: Vec&lt;usize&gt;</code> |  |
| <code>widths: HashMap&lt;&amp;'static str, f32&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>


### `DiagSpan`

**Fields**

| Field | Description |
| --- | --- |
| <code>end: usize</code> |  |
| <code>line: usize</code> |  |
| <code>severity: <a href="#diagseverity">DiagSeverity</a></code> |  |
| <code>start: usize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `FuzzyMatch`

**Fields**

| Field | Description |
| --- | --- |
| <code>positions: Vec&lt;usize&gt;</code> |  |
| <code>score: usize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Highlight`

**Fields**

| Field | Description |
| --- | --- |
| <code>end: usize</code> |  |
| <code>start: usize</code> |  |
| <code>token: <a href="sigil-style.html#token">Token</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `HlSpan`

**Fields**

| Field | Description |
| --- | --- |
| <code>end: usize</code> |  |
| <code>start: usize</code> |  |
| <code>token: &amp;'static str</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `Hunk`

**Fields**

| Field | Description |
| --- | --- |
| <code>header: String</code> |  |
| <code>lines: Vec&lt;<a href="#line">Line</a>&gt;</code> |  |
| <code>meta: Vec&lt;String&gt;</code> |  |
| <code>new_start: usize</code> |  |
| <code>old_start: usize</code> |  |
| <code>path: Option&lt;String&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `IconSource`

**Trait implementations:** <code>Clone</code>, <code><a href="#intoiconsource">IntoIconSource</a></code>


### `Line`

**Fields**

| Field | Description |
| --- | --- |
| <code>kind: <a href="#linekind">LineKind</a></code> |  |
| <code>new: Option&lt;usize&gt;</code> |  |
| <code>old: Option&lt;usize&gt;</code> |  |
| <code>text: String</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `LineId`

**Fields**

| Field | Description |
| --- | --- |
| <code>hunk: usize</code> |  |
| <code>line: usize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>Ord</code>, <code>PartialEq</code>, <code>PartialOrd</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ListColumn`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(text: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, width: f32) -&gt; Self</code> |  |
| <code>const start(self) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>align: <a href="#colalign">ColAlign</a></code> |  |
| <code>text: <a href="sigil-kernel.html#textcontent">TextContent</a></code> |  |
| <code>width: f32</code> |  |

**Trait implementations:** <code>From&lt;(<a href="sigil-kernel.html#textcontent">TextContent</a>, f32)&gt;</code>


### `Patch`

**Methods**

| Method | Description |
| --- | --- |
| <code>changes(&amp;self) -&gt; impl Iterator&lt;Item = <a href="#lineid">LineId</a>&gt; + '_</code> |  |
| <code>hunk_changes(&amp;self, hunk: usize) -&gt; impl Iterator&lt;Item = <a href="#lineid">LineId</a>&gt; + '_</code> |  |
| <code>line(&amp;self, id: <a href="#lineid">LineId</a>) -&gt; Option&lt;&amp;<a href="#line">Line</a>&gt;</code> |  |
| <code>parse(text: &amp;str) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>hunks: Vec&lt;<a href="#hunk">Hunk</a>&gt;</code> |  |
| <code>trailer: Vec&lt;String&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>, <code><a href="#diffsource">DiffSource</a></code>


### `Selection`

**Methods**

| Method | Description |
| --- | --- |
| <code>contains(&amp;self, id: <a href="#lineid">LineId</a>) -&gt; bool</code> |  |
| <code>select_hunk(&amp;mut self, patch: &amp;<a href="#patch">Patch</a>, hunk: usize)</code> |  |
| <code>select_line(&amp;mut self, patch: &amp;<a href="#patch">Patch</a>, id: <a href="#lineid">LineId</a>, extend: bool)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>hunks: BTreeSet&lt;usize&gt;</code> |  |
| <code>lines: BTreeSet&lt;<a href="#lineid">LineId</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `SelectionState`

**Methods**

| Method | Description |
| --- | --- |
| <code>click(&amp;mut self, index: usize)</code> |  |
| <code>contains(&amp;self, index: usize) -&gt; bool</code> |  |
| <code>ctrl_click(&amp;mut self, index: usize)</code> |  |
| <code>const mode(&amp;self) -&gt; <a href="#selectionmode">SelectionMode</a></code> |  |
| <code>new(mode: <a href="#selectionmode">SelectionMode</a>) -&gt; Self</code> |  |
| <code>selected_indices(&amp;self) -&gt; impl Iterator&lt;Item = usize&gt; + '_</code> |  |
| <code>shift_click(&amp;mut self, index: usize)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `SubmenuBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled(self, label: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>item(self, label: impl Into&lt;String&gt;, action: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>submenu(self, label: impl Into&lt;String&gt;, build: impl FnOnce(Self) -&gt; Self) -&gt; Self</code> |  |

**Trait implementations:** <code>Default</code>


### `TextEditState`

**Methods**

| Method | Description |
| --- | --- |
| <code>insert(&amp;self, text: &amp;str)</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;, caret: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>replace(&amp;self, text: &amp;str)</code> |  |
| <code>select(&amp;self, byte: usize, extend: bool)</code> |  |
| <code>selection(&amp;self) -&gt; Option&lt;Range&lt;usize&gt;&gt;</code> |  |

**Trait implementations:** <code>Clone</code>


### `ToastEntry`

**Fields**

| Field | Description |
| --- | --- |
| <code>level: <a href="#toastlevel">ToastLevel</a></code> |  |
| <code>message: String</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `TreeNode`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::child(self, child: Self) -&gt; Self</code> |  |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::children(self, children: impl IntoIterator&lt;Item = Self&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::detail(self, detail: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::leading(self, glyph: impl <a href="#intoiconsource">IntoIconSource</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::new(id: K, label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; Self</code> |  |
| <code>pub fn <a href="#treenode">TreeNode</a>&lt;K&gt;::status(self, status: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; Self</code> |  |
| <code>pub const fn <a href="#treenode">TreeNode</a>&lt;K&gt;::tone(self, tone: <a href="#rowtone">RowTone</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>children: Vec&lt;Self&gt;</code> |  |
| <code>detail: Option&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</code> |  |
| <code>id: K</code> |  |
| <code>label: <a href="sigil-kernel.html#textcontent">TextContent</a></code> |  |
| <code>leading: Option&lt;<a href="#iconsource">IconSource</a>&gt;</code> |  |
| <code>status: Option&lt;(<a href="sigil-kernel.html#textcontent">TextContent</a>, <a href="sigil-style.html#tone">Tone</a>)&gt;</code> |  |
| <code>tone: <a href="#rowtone">RowTone</a></code> |  |

**Trait implementations:** <code>Clone</code>


### `TreeRow`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#treerow">TreeRow</a>&lt;K&gt;::has_children(&amp;self) -&gt; bool</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>depth: u16</code> |  |
| <code>disclosure: <a href="#disclosure">Disclosure</a></code> |  |
| <code>id: K</code> |  |
| <code>parent: Option&lt;usize&gt;</code> |  |
| <code>selected: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>


## Enums

### `ButtonGlyph`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Icon(<a href="sigil-icons.html#icon">Icon</a>)</code> |  |
| <code>Text(<a href="sigil-kernel.html#textcontent">TextContent</a>)</code> |  |

**Trait implementations:** <code>From&lt;<a href="sigil-icons.html#icon">Icon</a>&gt;</code>, <code>From&lt;T&gt;</code>


### `ColWidth`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Fixed(f32)</code> |  |
| <code>Fraction(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Density`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compact</code> |  |
| <code>Default</code> |  |
| <code>Grid</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DiffView`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Split</code> |  |
| <code>Unified</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Disclosure`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Collapsed</code> |  |
| <code>Expanded</code> |  |
| <code>Leaf</code> |  |
| <code>None</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Orientation`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Horizontal</code> |  |
| <code>Vertical</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SelectionMode`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Extend</code> |  |
| <code>Multi</code> |  |
| <code>None</code> |  |
| <code>Single</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Side`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bottom</code> |  |
| <code>Left</code> |  |
| <code>Right</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SortDir`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ascending</code> |  |
| <code>Descending</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SubmitKey`

**Variants**

| Variant | Description |
| --- | --- |
| <code>CtrlEnter</code> |  |
| <code>Enter</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PathColor`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Rgba(<a href="sigil-style.html#rgba8">Rgba8</a>)</code> |  |
| <code>Token(<a href="sigil-style.html#token">Token</a>)</code> |  |

**Trait implementations:** <code>From&lt;<a href="sigil-style.html#token">Token</a>&gt;</code>, <code>From&lt;<a href="sigil-style.html#rgba8">Rgba8</a>&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Axis`

**Variants**

| Variant | Description |
| --- | --- |
| <code>X</code> |  |
| <code>Y</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ChangeKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Added</code> |  |
| <code>Deleted</code> |  |
| <code>Modified</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ChartMark`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bar</code> |  |
| <code>Candle</code> |  |
| <code>Line</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ColAlign`

**Variants**

| Variant | Description |
| --- | --- |
| <code>End</code> |  |
| <code>Start</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DiagSeverity`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Error</code> |  |
| <code>Hint</code> |  |
| <code>Info</code> |  |
| <code>Warning</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>Ord</code>, <code>PartialEq</code>, <code>PartialOrd</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FocusCaret`

**Variants**

| Variant | Description |
| --- | --- |
| <code>End</code> |  |
| <code>Keep</code> |  |
| <code>SelectAll</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LineKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>const is_change(self) -&gt; bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Added</code> |  |
| <code>Context</code> |  |
| <code>Note</code> |  |
| <code>Removed</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PathCmd`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Close</code> |  |
| <code>CubicTo(f32, f32, f32, f32, f32, f32)</code> |  |
| <code>LineTo(f32, f32)</code> |  |
| <code>MoveTo(f32, f32)</code> |  |
| <code>QuadTo(f32, f32, f32, f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PathFill`

**Fields**

| Field | Description |
| --- | --- |
| <code>Linear::end: (f32, f32)</code> |  |
| <code>Linear::start: (f32, f32)</code> |  |
| <code>Linear::stops: Vec&lt;(f32, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;</code> |  |
| <code>Radial::center: (f32, f32)</code> |  |
| <code>Radial::radius: f32</code> |  |
| <code>Radial::stops: Vec&lt;(f32, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Linear</code> |  |
| <code>Radial</code> |  |
| <code>Solid(<a href="sigil-style.html#rgba8">Rgba8</a>)</code> |  |
| <code>Theme(<a href="sigil-style.html#token">Token</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `RowTone`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Header</code> |  |
| <code>Muted</code> |  |
| <code>Normal</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ScrollAnchor`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bottom</code> |  |
| <code>Index(usize)</code> |  |
| <code>Top</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SparkFill`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Shade</code> |  |
| <code>Solid</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SparkMark`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Block</code> |  |
| <code>Braille</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ToastLevel`

**Methods**

| Method | Description |
| --- | --- |
| <code>const tone(self) -&gt; <a href="sigil-style.html#tone">Tone</a></code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Error</code> |  |
| <code>Info</code> |  |
| <code>Success</code> |  |
| <code>Warning</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `DiffSource`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_patch(self) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;Rc&lt;<a href="#patch">Patch</a>&gt;&gt;</code> |  |


### `DragDropExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>drag_source(self, state: <a href="#dragstate">DragState</a>&lt;T&gt;, payload: T) -&gt; Self</code> |  |
| <code>drop_target(self, state: <a href="#dragstate">DragState</a>&lt;T&gt;, accepts: impl Fn(&amp;T) -&gt; bool + 'static, on_drop: impl Fn(T) + 'static) -&gt; Self</code> |  |


### `FieldControl`

**Methods**

| Method | Description |
| --- | --- |
| <code>invalid_when(self, invalid: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |


### `Identifiable`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Id: Clone + Eq + Hash</code> |  |
| <code>id(&amp;self) -&gt; Self::Id</code> |  |


### `IntoIconSource`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_icon_source(self) -&gt; <a href="#iconsource">IconSource</a></code> |  |


### `IntoList`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_list(self, render: R) -&gt; <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt;</code> |  |


### `NumericKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>clamp_to(self, range: &amp;RangeInclusive&lt;Self&gt;) -&gt; Self</code> |  |
| <code>parse(input: &amp;str) -&gt; Option&lt;Self&gt;</code> |  |
| <code>render(self) -&gt; String</code> |  |


### `RowRender`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Row: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</code> |  |
| <code>row(&amp;self, index: usize, item: &amp;T) -&gt; Self::Row</code> |  |


### `TreeKey`


## Functions

### `app_shell`

<pre>pub fn <a href="#app_shell">app_shell</a>(top: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, work: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, status: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, modal: Option&lt;impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `avatar`

<pre>pub fn <a href="#avatar">avatar</a>(initials: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `axis_drag`

<pre>pub fn <a href="#axis_drag">axis_drag</a>&lt;S&gt;(node: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>, axis: <a href="#axis">Axis</a>, snapshot: impl Fn() -&gt; S + 'static, on_delta: impl Fn(S, f32) + 'static, on_end: impl Fn() + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where S: Clone + 'static</pre>


### `button`

<pre>pub fn <a href="#button">button</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="#buttonbuilder">ButtonBuilder</a></pre>


### `card`

<pre>pub fn <a href="#card">card</a>(child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="#cardbuilder">CardBuilder</a></pre>


### `chart`

<pre>pub fn <a href="#chart">chart</a>(values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; <a href="#chartbuilder">ChartBuilder</a></pre>


### `checkbox`

<pre>pub fn <a href="#checkbox">checkbox</a>(checked: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="#checkboxbuilder">CheckboxBuilder</a></pre>


### `chip`

<pre>pub fn <a href="#chip">chip</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="#chipbuilder">ChipBuilder</a></pre>


### `code_edit`

<pre>pub fn <a href="#code_edit">code_edit</a>(lines: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;String&gt;&gt;, caret: <a href="sigil-reactive.html#signal">Signal</a>&lt;(usize, usize)&gt;) -&gt; <a href="#codeeditbuilder">CodeEditBuilder</a></pre>


### `color_field`

<pre>pub const fn <a href="#color_field">color_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="sigil-style.html#rgba8">Rgba8</a>&gt;) -&gt; <a href="#colorfieldbuilder">ColorFieldBuilder</a></pre>


### `combobox`

<pre>pub fn <a href="#combobox-1">combobox</a>(state: <a href="#combobox">Combobox</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `command_menu`

<pre>pub fn <a href="#command_menu">command_menu</a>() -&gt; <a href="#commandmenubuilder">CommandMenuBuilder</a></pre>


### `context_menu`

<pre>pub fn <a href="#context_menu">context_menu</a>(target: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, items: impl Fn() -&gt; Vec&lt;<a href="#menuitem">MenuItem</a>&gt; + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `date_field`

<pre>pub fn <a href="#date_field">date_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="#date">Date</a>&gt;) -&gt; <a href="#datefieldbuilder">DateFieldBuilder</a></pre>


### `diff`

<pre>pub fn <a href="#diff">diff</a>(patch: impl <a href="#diffsource">DiffSource</a> + 'static) -&gt; <a href="#diffbuilder">DiffBuilder</a></pre>


### `divider_horizontal`

<pre>pub fn <a href="#divider_horizontal">divider_horizontal</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `divider_vertical`

<pre>pub const fn <a href="#divider_vertical">divider_vertical</a>() -&gt; <a href="#verticaldivider">VerticalDivider</a></pre>


### `dot`

<pre>pub fn <a href="#dot">dot</a>(tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="#dotbuilder">DotBuilder</a></pre>


### `drag_ghost`

<pre>pub fn <a href="#drag_ghost">drag_ghost</a>&lt;T, N&gt;(state: <a href="#dragstate">DragState</a>&lt;T&gt;, render: impl Fn(&amp;T) -&gt; N + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where T: Clone + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `dropdown`

<pre>pub fn <a href="#dropdown">dropdown</a>(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, options: Vec&lt;String&gt;) -&gt; <a href="#dropdownbuilder">DropdownBuilder</a></pre>


### `empty_state`

<pre>pub fn <a href="#empty_state">empty_state</a>(title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, body: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `field`

<pre>pub fn <a href="#field">field</a>&lt;C: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static&gt;(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, control: C) -&gt; <a href="#fieldbuilder">FieldBuilder</a>&lt;C&gt;</pre>


### `fuzzy_score`

<pre>pub fn <a href="#fuzzy_score">fuzzy_score</a>(query: &amp;str, text: &amp;str) -&gt; Option&lt;<a href="#fuzzymatch">FuzzyMatch</a>&gt;</pre>


### `gauge`

<pre>pub fn <a href="#gauge">gauge</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; <a href="#gaugebuilder">GaugeBuilder</a></pre>


### `heatmap`

<pre>pub fn <a href="#heatmap">heatmap</a>&lt;F: Fn(usize, usize) -&gt; <a href="sigil-style.html#rgba8">Rgba8</a>&gt;(grid_w: usize, grid_h: usize, color: F) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `highlight`

<pre>pub fn <a href="#highlight-1">highlight</a>(language: &amp;str, source: &amp;str) -&gt; Vec&lt;Vec&lt;<a href="#highlight">Highlight</a>&gt;&gt;</pre>


### `hint_bar`

<pre>pub fn <a href="#hint_bar">hint_bar</a>(keys: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, right: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `hint_overlay`

<pre>pub fn <a href="#hint_overlay">hint_overlay</a>(hints: &amp;[<a href="#hintentry">HintEntry</a>&lt;'_&gt;]) -&gt; <a href="#hintoverlaybuilder">HintOverlayBuilder</a></pre>


### `hl_spans`

<pre>pub fn <a href="#hl_spans">hl_spans</a>(language: &amp;str, source: &amp;str) -&gt; Vec&lt;Vec&lt;<a href="#hlspan">HlSpan</a>&gt;&gt;</pre>


### `html`

<pre>pub fn <a href="#html">html</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `icon`

<pre>pub fn <a href="#icon">icon</a>(glyph: impl <a href="#intoiconsource">IntoIconSource</a>) -&gt; <a href="#iconbuilder">IconBuilder</a></pre>


### `icon_button`

<pre>pub fn <a href="#icon_button">icon_button</a>(glyph: impl Into&lt;<a href="#buttonglyph">ButtonGlyph</a>&gt;, activate: impl Fn() + 'static) -&gt; <a href="#buttonbuilder">ButtonBuilder</a></pre>


### `icon_svg`

<pre>pub fn <a href="#icon_svg">icon_svg</a>(svg: impl Into&lt;Rc&lt;str&gt;&gt;) -&gt; <a href="#iconbuilder">IconBuilder</a></pre>


### `image_bytes`

<pre>pub fn <a href="#image_bytes">image_bytes</a>(encoded: &amp;[u8]) -&gt; Option&lt;<a href="sigil-kernel.html#imagebuilder">ImageBuilder</a>&gt;</pre>


### `image_rgba`

<pre>pub fn <a href="#image_rgba">image_rgba</a>(width: u32, height: u32, rgba: Vec&lt;u8&gt;) -&gt; Option&lt;<a href="sigil-kernel.html#imagebuilder">ImageBuilder</a>&gt;</pre>


### `interactive`

<pre>pub fn <a href="#interactive">interactive</a>(inner: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>) -&gt; <a href="#interactivebuilder">InteractiveBuilder</a></pre>


### `kv_grid`

<pre>pub fn <a href="#kv_grid">kv_grid</a>&lt;K, V&gt;(rows: impl IntoIterator&lt;Item = (K, V)&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where K: Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, V: Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;</pre>


### `kv_row`

<pre>pub fn <a href="#kv_row">kv_row</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, value: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `language_for_path`

<pre>pub fn <a href="#language_for_path">language_for_path</a>(path: &amp;str) -&gt; Option&lt;&amp;'static str&gt;</pre>


### `list_row`

<pre>pub fn <a href="#list_row">list_row</a>(selected: bool, title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, detail: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `list_row_with`

<pre>pub fn <a href="#list_row_with">list_row_with</a>(selected: bool, title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, slots: <a href="#listrowslots">ListRowSlots</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `markdown`

<pre>pub fn <a href="#markdown">markdown</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `markdown_edit`

<pre>pub const fn <a href="#markdown_edit">markdown_edit</a>(source: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="#markdowneditbuilder">MarkdownEditBuilder</a></pre>


### `markdown_inline`

<pre>pub fn <a href="#markdown_inline">markdown_inline</a>(source: &amp;str) -&gt; <a href="sigil-kernel.html#textbuilder">TextBuilder</a></pre>


### `menu`

<pre>pub fn <a href="#menu">menu</a>(open: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="#menubuilder">MenuBuilder</a></pre>


### `menu_bar`

<pre>pub fn <a href="#menu_bar">menu_bar</a>() -&gt; <a href="#menubarbuilder">MenuBarBuilder</a></pre>


### `mermaid`

<pre>pub fn <a href="#mermaid">mermaid</a>(source: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;String&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `modal`

<pre>pub fn <a href="#modal">modal</a>(show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="#modalbuilder">ModalBuilder</a></pre>


### `notice_band`

<pre>pub fn <a href="#notice_band">notice_band</a>(body: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, tone: <a href="sigil-style.html#tone">Tone</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `number_field`

<pre>pub fn <a href="#number_field">number_field</a>&lt;T: <a href="#numerickind">NumericKind</a>&gt;(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;, range: RangeInclusive&lt;T&gt;) -&gt; <a href="#numberfieldbuilder">NumberFieldBuilder</a>&lt;T&gt;</pre>


### `panel`

<pre>pub fn <a href="#panel">panel</a>() -&gt; <a href="#panelbuilder">PanelBuilder</a></pre>


### `parse_hex`

<pre>pub fn <a href="#parse_hex">parse_hex</a>(input: &amp;str) -&gt; Option&lt;<a href="sigil-style.html#rgba8">Rgba8</a>&gt;</pre>


### `path`

<pre>pub const fn <a href="#path">path</a>(width: u32, height: u32) -&gt; <a href="#pathnode">PathNode</a></pre>


### `progress`

<pre>pub fn <a href="#progress">progress</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, max: f32) -&gt; <a href="#progressbuilder">ProgressBuilder</a></pre>


### `radio_group`

<pre>pub fn <a href="#radio_group">radio_group</a>(selected: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="#radiogroupbuilder">RadioGroupBuilder</a></pre>


### `scroll`

<pre>pub fn <a href="#scroll">scroll</a>(content: <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a>) -&gt; <a href="#scrollbuilder">ScrollBuilder</a></pre>


### `section_label`

<pre>pub fn <a href="#section_label">section_label</a>(title: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `segmented`

<pre>pub fn <a href="#segmented">segmented</a>(active: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, labels: Vec&lt;String&gt;) -&gt; <a href="#segmentedbuilder">SegmentedBuilder</a></pre>


### `sheet`

<pre>pub fn <a href="#sheet">sheet</a>(show: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="#modalbuilder">ModalBuilder</a></pre>


### `sidebar`

<pre>pub fn <a href="#sidebar">sidebar</a>(child: impl <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static) -&gt; <a href="#sidebarbuilder">SidebarBuilder</a></pre>


### `slider`

<pre>pub fn <a href="#slider">slider</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, range: RangeInclusive&lt;f32&gt;) -&gt; <a href="#sliderbuilder">SliderBuilder</a></pre>


### `spacer`

<pre>pub fn <a href="#spacer">spacer</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `sparkline`

<pre>pub fn <a href="#sparkline">sparkline</a>(values: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;f32&gt;&gt;) -&gt; <a href="#sparklinebuilder">SparklineBuilder</a></pre>


### `spinner`

<pre>pub fn <a href="#spinner">spinner</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `spinner_frame`

<pre>pub fn <a href="#spinner_frame">spinner_frame</a>(tick: u64) -&gt; &amp;'static str</pre>


### `splitter`

<pre>pub fn <a href="#splitter">splitter</a>(orientation: <a href="#orientation">Orientation</a>) -&gt; <a href="#splitterbuilder">SplitterBuilder</a></pre>


### `stat`

<pre>pub fn <a href="#stat">stat</a>(label: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;, value: impl Into&lt;<a href="sigil-kernel.html#textcontent">TextContent</a>&gt;) -&gt; <a href="#statbuilder">StatBuilder</a></pre>


### `status_bar`

<pre>pub fn <a href="#status_bar">status_bar</a>&lt;I&gt;(children: I) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a> where I: IntoIterator, &lt;I as IntoIterator&gt;::Item: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


### `surface`

<pre>pub fn <a href="#surface">surface</a>(key: u64) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `table`

<pre>pub fn <a href="#table">table</a>&lt;Row: 'static&gt;(state: <a href="#tablestate">TableState</a>&lt;Row&gt;) -&gt; <a href="#tablebuilder">TableBuilder</a>&lt;Row&gt;</pre>


### `tabs`

<pre>pub fn <a href="#tabs">tabs</a>() -&gt; <a href="#tabsbuilder">TabsBuilder</a></pre>


### `terminal`

<pre>pub fn <a href="#terminal">terminal</a>(lines: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;String&gt;&gt;, on_input: impl Fn(String) + 'static) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `text_edit`

<pre>pub fn <a href="#text_edit">text_edit</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="#texteditbuilder">TextEditBuilder</a></pre>


### `text_field`

<pre>pub fn <a href="#text_field">text_field</a>(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; <a href="#textfieldbuilder">TextFieldBuilder</a></pre>


### `text_runs`

<pre>pub fn <a href="#text_runs">text_runs</a>(marks: &amp;[<a href="#highlight">Highlight</a>], range: Range&lt;usize&gt;) -&gt; Vec&lt;<a href="sigil-style.html#textrun">TextRun</a>&gt;</pre>


### `to_hex`

<pre>pub fn <a href="#to_hex">to_hex</a>(color: <a href="sigil-style.html#rgba8">Rgba8</a>) -&gt; String</pre>


### `toast`

<pre>pub fn <a href="#toast">toast</a>(message: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;String&gt;&gt;) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `toast_queue`

<pre>pub fn <a href="#toast_queue">toast_queue</a>(queue: <a href="#toastqueue">ToastQueue</a>) -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `toggle`

<pre>pub fn <a href="#toggle">toggle</a>(on: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; <a href="#togglebuilder">ToggleBuilder</a></pre>


### `tooltip`

<pre>pub fn <a href="#tooltip">tooltip</a>(label: impl Into&lt;String&gt;) -&gt; <a href="#tooltipbuilder">TooltipBuilder</a></pre>


### `tree`

<pre>pub fn <a href="#tree">tree</a>&lt;K: <a href="#treekey">TreeKey</a>&gt;(nodes: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;<a href="#treenode">TreeNode</a>&lt;K&gt;&gt;&gt;) -&gt; <a href="#treebuilder">TreeBuilder</a>&lt;K&gt;</pre>


### `virtual_list`

<pre>pub fn <a href="#virtual_list">virtual_list</a>&lt;T, R, N, I&gt;(items: I, render: R) -&gt; <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt; where T: 'static, R: Fn(usize, &amp;T) -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static, I: <a href="#intolist">IntoList</a>&lt;T, R&gt;</pre>


### `virtual_list_bind`

<pre>pub fn <a href="#virtual_list_bind">virtual_list_bind</a>&lt;S, T, R, N&gt;(items: <a href="sigil-reactive.html#signal">Signal</a>&lt;S&gt;, render: R) -&gt; <a href="#virtuallistbuilder">VirtualListBuilder</a>&lt;T, R&gt; where S: Deref + Clone + 'static, &lt;S as Deref&gt;::Target: AsRef&lt;[T]&gt;, T: 'static, R: Fn(usize, &amp;T) -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a> + 'static</pre>


## Type aliases

### `HintEntry`

<pre>pub type <a href="#hintentry">HintEntry</a>&lt;'a&gt; = (&amp;'a str, &amp;'a str)</pre>


## Constants

### `DRAG_THRESHOLD_PX`

<pre>pub const <a href="#drag_threshold_px">DRAG_THRESHOLD_PX</a>: f32</pre>


### `DROP_TARGET_CLASS`

<pre>pub const <a href="#drop_target_class">DROP_TARGET_CLASS</a>: &amp;str</pre>


### `MAX_VISIBLE`

<pre>pub const <a href="#max_visible">MAX_VISIBLE</a>: usize</pre>


### `MIN_COL_WIDTH`

<pre>pub const <a href="#min_col_width">MIN_COL_WIDTH</a>: f32</pre>


### `TOAST_TTL`

<pre>pub const <a href="#toast_ttl">TOAST_TTL</a>: Duration</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
