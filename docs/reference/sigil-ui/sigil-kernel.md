# Module `sigil::kernel`

| Item | Kind | Description |
| --- | --- | --- |
| [`NodeBuilder`](#nodebuilder) | struct |  |
| [`A11yProps`](#a11yprops) | struct |  |
| [`Cx`](#cx) | struct |  |
| [`FrameDelta`](#framedelta) | struct |  |
| [`ImageBuilder`](#imagebuilder) | struct |  |
| [`ImageCrop`](#imagecrop) | struct |  |
| [`ImageRaster`](#imageraster) | struct |  |
| [`ImageSource`](#imagesource) | struct |  |
| [`Node`](#node) | struct |  |
| [`NodeExtensions`](#nodeextensions) | struct |  |
| [`NodeId`](#nodeid) | struct |  |
| [`NodeTag`](#nodetag) | struct |  |
| [`StatusView`](#statusview) | struct |  |
| [`Surface`](#surface) | struct |  |
| [`SwitchBuilder`](#switchbuilder) | struct |  |
| [`TextBuilder`](#textbuilder) | struct |  |
| [`TextCursor`](#textcursor) | struct |  |
| [`TopLayer`](#toplayer) | struct |  |
| [`AccessibleName`](#accessiblename) | enum |  |
| [`DeltaKind`](#deltakind) | enum |  |
| [`FocusScopeKind`](#focusscopekind) | enum |  |
| [`HostMode`](#hostmode) | enum |  |
| [`NodeType`](#nodetype) | enum |  |
| [`ObjectFit`](#objectfit) | enum |  |
| [`TextContent`](#textcontent) | enum |  |
| [`IntoNode`](#intonode) | trait |  |
| [`announce`](#announce) | fn |  |
| [`announce_assertive`](#announce_assertive) | fn |  |
| [`container`](#container) | fn |  |
| [`for_each`](#for_each) | fn |  |
| [`fragment`](#fragment) | fn |  |
| [`image`](#image) | fn |  |
| [`match_status`](#match_status) | fn |  |
| [`node`](#node-1) | fn |  |
| [`schedule_keyed_rebuild`](#schedule_keyed_rebuild) | fn |  |
| [`show`](#show) | fn |  |
| [`switch`](#switch) | fn |  |
| [`text`](#text) | fn |  |
| [`Action`](#action) | use |  |
| [`Live`](#live) | use |  |
| [`Role`](#role) | use |  |
| [`SortDirection`](#sortdirection) | use |  |

## Structs

### `NodeBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>action(self, action: Action) -&gt; Self</code> |  |
| <code>add(self, cx: &amp;mut <a href="#cx">Cx</a>) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>const checked(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>child(self, child: impl <a href="#intonode">IntoNode</a> + 'static) -&gt; Self</code> |  |
| <code>children_keyed&lt;F, I, K, N&gt;(self, f: F) -&gt; Self where F: Fn() -&gt; I + 'static, I: IntoIterator&lt;Item = (K, N)&gt;, K: Clone + Eq + Hash + 'static, N: <a href="#intonode">IntoNode</a> + 'static</code> |  |
| <code>children_reactive&lt;F, I, N&gt;(self, f: F) -&gt; Self where F: Fn() -&gt; I + 'static, I: IntoIterator&lt;Item = N&gt;, N: <a href="#intonode">IntoNode</a> + 'static</code> |  |
| <code>const column_count(self, count: usize) -&gt; Self</code> |  |
| <code>const column_index(self, index: usize) -&gt; Self</code> |  |
| <code>description(self, description: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const expanded(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>focusable(self, yes: bool) -&gt; Self</code> |  |
| <code>const hittable(self, yes: bool) -&gt; Self</code> |  |
| <code>keyboard_shortcut(self, shortcut: impl Into&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;) -&gt; Self</code> |  |
| <code>label(self, name: impl Into&lt;<a href="#accessiblename">AccessibleName</a>&gt;) -&gt; Self</code> |  |
| <code>const labelled_by(self, node: <a href="#nodeid">NodeId</a>) -&gt; Self</code> |  |
| <code>mutate(self, apply: impl FnOnce(&amp;mut <a href="#node">Node</a>) + 'static) -&gt; Self</code> |  |
| <code>on_added(self, hook: impl FnOnce(&amp;mut <a href="#cx">Cx</a>, <a href="#nodeid">NodeId</a>) + 'static) -&gt; Self</code> |  |
| <code>owns(self, node: <a href="#nodeid">NodeId</a>) -&gt; Self</code> |  |
| <code>placeholder(self, placeholder: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const role(self, role: Role) -&gt; Self</code> |  |
| <code>const row_count(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;) -&gt; Self</code> |  |
| <code>const row_index(self, index: usize) -&gt; Self</code> |  |
| <code>const selected(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const sort_direction(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;SortDirection&gt;&gt;) -&gt; Self</code> |  |
| <code>const tab_index(self, index: i32) -&gt; Self</code> |  |
| <code>const text_cursor(self, caret: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;, anchor: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;usize&gt;&gt;) -&gt; Self</code> |  |
| <code>const text_value(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;) -&gt; Self</code> |  |
| <code>const traps_tab(self, yes: bool) -&gt; Self</code> |  |
| <code>value(self, signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, min: f32, max: f32) -&gt; Self</code> |  |
| <code>const value_step(self, step: f32) -&gt; Self</code> |  |
| <code>with&lt;T: 'static&gt;(self, value: T) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="sigil-events.html#nodeeventsext">NodeEventsExt</a></code>, <code>From&lt;<a href="#imagebuilder">ImageBuilder</a>&gt;</code>, <code>From&lt;<a href="#textbuilder">TextBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#buttonbuilder">ButtonBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#cardbuilder">CardBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#chartbuilder">ChartBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#checkboxbuilder">CheckboxBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#codeeditbuilder">CodeEditBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#colorfieldbuilder">ColorFieldBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#commandmenubuilder">CommandMenuBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#datefieldbuilder">DateFieldBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#diffbuilder">DiffBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#dotbuilder">DotBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#dropdownbuilder">DropdownBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#gaugebuilder">GaugeBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#iconbuilder">IconBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#markdowneditbuilder">MarkdownEditBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#menubarbuilder">MenuBarBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#menubuilder">MenuBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#modalbuilder">ModalBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#panebuilder">PaneBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#panelbuilder">PanelBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#pathnode">PathNode</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#progressbuilder">ProgressBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#radiogroupbuilder">RadioGroupBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#scrollbuilder">ScrollBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#segmentedbuilder">SegmentedBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#sliderbuilder">SliderBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#sparklinebuilder">SparklineBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#splitterbuilder">SplitterBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#statbuilder">StatBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#tabsbuilder">TabsBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#texteditbuilder">TextEditBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#textfieldbuilder">TextFieldBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#togglebuilder">ToggleBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#tooltipbuilder">TooltipBuilder</a>&gt;</code>, <code>From&lt;<a href="sigil-widgets.html#verticaldivider">VerticalDivider</a>&gt;</code>, <code><a href="#intonode">IntoNode</a></code>, <code><a href="sigil-layout.html#layoutbuilderext">LayoutBuilderExt</a></code>, <code><a href="sigil-layout.html#layoutreactiveext">LayoutReactiveExt</a></code>, <code><a href="sigil-style.html#nodestyleext">NodeStyleExt</a></code>, <code><a href="sigil-widgets.html#dragdropext">DragDropExt</a>&lt;T&gt;</code>


### `A11yProps`

**Methods**

| Method | Description |
| --- | --- |
| <code>is_disabled(&amp;self) -&gt; bool</code> |  |
| <code>is_focusable(&amp;self) -&gt; bool</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>actions: SmallVec&lt;[Action; 4]&gt;</code> |  |
| <code>checked: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>column_count: Option&lt;usize&gt;</code> |  |
| <code>column_index: Option&lt;usize&gt;</code> |  |
| <code>described_by: Option&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>description: Option&lt;<a href="#accessiblename">AccessibleName</a>&gt;</code> |  |
| <code>disabled: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>expanded: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>focus_scope: Option&lt;<a href="#focusscopekind">FocusScopeKind</a>&gt;</code> |  |
| <code>focusable: bool</code> |  |
| <code>hittable: bool</code> |  |
| <code>invalid: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>keyboard_shortcut: Option&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;</code> |  |
| <code>labelled_by: Option&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>level: Option&lt;usize&gt;</code> |  |
| <code>live: Option&lt;Live&gt;</code> |  |
| <code>name: <a href="#accessiblename">AccessibleName</a></code> |  |
| <code>owns: SmallVec&lt;[<a href="#nodeid">NodeId</a>; 2]&gt;</code> |  |
| <code>placeholder: Option&lt;String&gt;</code> |  |
| <code>position_in_set: Option&lt;usize&gt;</code> |  |
| <code>required: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>role: Role</code> |  |
| <code>row_count: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;&gt;</code> |  |
| <code>row_index: Option&lt;usize&gt;</code> |  |
| <code>selected: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>size_of_set: Option&lt;usize&gt;</code> |  |
| <code>sort_direction: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;SortDirection&gt;&gt;&gt;</code> |  |
| <code>step: Option&lt;f32&gt;</code> |  |
| <code>tab_index: Option&lt;i32&gt;</code> |  |
| <code>tab_stop: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;&gt;</code> |  |
| <code>text_cursor: Option&lt;<a href="#textcursor">TextCursor</a>&gt;</code> |  |
| <code>text_value: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;&gt;</code> |  |
| <code>traps_tab: bool</code> |  |
| <code>value_max: Option&lt;f32&gt;</code> |  |
| <code>value_min: Option&lt;f32&gt;</code> |  |
| <code>value_now: Option&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>


### `Cx`

**Methods**

| Method | Description |
| --- | --- |
| <code>add&lt;C: <a href="#intonode">IntoNode</a>&gt;(&amp;mut self, child: C) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>add_child&lt;C: <a href="#intonode">IntoNode</a>&gt;(&amp;mut self, parent: <a href="#nodeid">NodeId</a>, child: C) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>clear_dirty(&amp;mut self)</code> |  |
| <code>dirty(&amp;self) -&gt; impl Iterator&lt;Item = <a href="#nodeid">NodeId</a>&gt; + '_</code> |  |
| <code>const dirty_all(&amp;self) -&gt; bool</code> |  |
| <code>get(&amp;self, id: <a href="#nodeid">NodeId</a>) -&gt; Option&lt;&amp;<a href="#node">Node</a>&gt;</code> |  |
| <code>get_mut(&amp;mut self, id: <a href="#nodeid">NodeId</a>) -&gt; Option&lt;&amp;mut <a href="#node">Node</a>&gt;</code> |  |
| <code>get_mut_untracked(&amp;mut self, id: <a href="#nodeid">NodeId</a>) -&gt; Option&lt;&amp;mut <a href="#node">Node</a>&gt;</code> |  |
| <code>has_specific_dirty(&amp;self) -&gt; bool</code> |  |
| <code>const host_mode(&amp;self) -&gt; <a href="#hostmode">HostMode</a></code> |  |
| <code>const instance_id(&amp;self) -&gt; u64</code> |  |
| <code>is_empty(&amp;self) -&gt; bool</code> |  |
| <code>len(&amp;self) -&gt; usize</code> |  |
| <code>mark_mutated(&amp;mut self)</code> |  |
| <code>const mutation_generation(&amp;self) -&gt; u64</code> |  |
| <code>new_node(&amp;mut self, kind: <a href="#nodetype">NodeType</a>, a11y: <a href="#a11yprops">A11yProps</a>) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>new_root() -&gt; Self</code> |  |
| <code>remove(&amp;mut self, id: <a href="#nodeid">NodeId</a>) -&gt; bool</code> |  |
| <code>const root_owner(&amp;self) -&gt; <a href="sigil-reactive.html#ownerid">OwnerId</a></code> |  |
| <code>roots(&amp;self) -&gt; &amp;[<a href="#nodeid">NodeId</a>]</code> |  |
| <code>const scale(&amp;self) -&gt; <a href="sigil-geometry.html#scale">Scale</a></code> |  |
| <code>set_scale(&amp;mut self, scale: <a href="sigil-geometry.html#scale">Scale</a>)</code> |  |
| <code>describe(&amp;self, id: <a href="#nodeid">NodeId</a>) -&gt; String</code> |  |
| <code>dump_tree(&amp;self, root: <a href="#nodeid">NodeId</a>) -&gt; String</code> |  |
| <code>first_child(&amp;self, id: <a href="#nodeid">NodeId</a>) -&gt; Option&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>parent(&amp;self, id: <a href="#nodeid">NodeId</a>) -&gt; Option&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>visit_preorder(&amp;self, root: <a href="#nodeid">NodeId</a>, visit: impl FnMut(<a href="#nodeid">NodeId</a>, &amp;<a href="#node">Node</a>))</code> |  |
| <code>flush_announcements(&amp;mut self)</code> |  |
| <code>flush_reactive_children(&amp;mut self)</code> |  |
| <code>take_delta(&amp;mut self) -&gt; <a href="#framedelta">FrameDelta</a></code> |  |

**Trait implementations:** <code>Drop</code>


### `FrameDelta`

**Methods**

| Method | Description |
| --- | --- |
| <code>is_empty(&amp;self) -&gt; bool</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>layout: AHashSet&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>paint: AHashSet&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>structure: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `ImageBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>alt(self, text: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>crop(self, top: u32, height: u32) -&gt; Self</code> |  |
| <code>const fit(self, fit: <a href="#objectfit">ObjectFit</a>) -&gt; Self</code> |  |
| <code>mutate(self, apply: impl FnOnce(&amp;mut <a href="#node">Node</a>) + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="#intonode">IntoNode</a></code>, <code><a href="sigil-widgets-layout_ext.html#widgetlayoutext">WidgetLayoutExt</a></code>


### `ImageCrop`

**Fields**

| Field | Description |
| --- | --- |
| <code>height: u32</code> |  |
| <code>top: u32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ImageRaster`

**Methods**

| Method | Description |
| --- | --- |
| <code>at(&amp;self, width: u32, height: u32, scale: f32) -&gt; Option&lt;<a href="#imagesource">ImageSource</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>


### `ImageSource`

**Methods**

| Method | Description |
| --- | --- |
| <code>from_rgba8(width: u32, height: u32, rgba: Vec&lt;u8&gt;) -&gt; Option&lt;Self&gt;</code> |  |
| <code>const height(&amp;self) -&gt; u32</code> |  |
| <code>const id(&amp;self) -&gt; u64</code> |  |
| <code>rgba(&amp;self) -&gt; &amp;[u8]</code> |  |
| <code>const width(&amp;self) -&gt; u32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>


### `Node`

**Methods**

| Method | Description |
| --- | --- |
| <code>attach&lt;T: 'static&gt;(&amp;mut self, value: T) -&gt; Option&lt;T&gt;</code> |  |
| <code>attach_untracked&lt;T: 'static&gt;(&amp;mut self, value: T) -&gt; Option&lt;T&gt;</code> |  |
| <code>children(&amp;self) -&gt; &amp;[<a href="#nodeid">NodeId</a>]</code> |  |
| <code>get_extension&lt;T: 'static&gt;(&amp;self) -&gt; Option&lt;&amp;T&gt;</code> |  |
| <code>get_extension_mut&lt;T: 'static&gt;(&amp;mut self) -&gt; Option&lt;&amp;mut T&gt;</code> |  |
| <code>get_extension_mut_untracked&lt;T: 'static&gt;(&amp;mut self) -&gt; Option&lt;&amp;mut T&gt;</code> |  |
| <code>const parent(&amp;self) -&gt; Option&lt;<a href="#nodeid">NodeId</a>&gt;</code> |  |
| <code>remove_extension&lt;T: 'static&gt;(&amp;mut self) -&gt; Option&lt;T&gt;</code> |  |
| <code>const tag(&amp;self) -&gt; Option&lt;<a href="#nodetag">NodeTag</a>&gt;</code> |  |
| <code>const version(&amp;self) -&gt; u32</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>a11y: <a href="#a11yprops">A11yProps</a></code> |  |
| <code>kind: <a href="#nodetype">NodeType</a></code> |  |
| <code>owner: <a href="sigil-reactive.html#ownerid">OwnerId</a></code> |  |

**Trait implementations:** <code>Debug</code>


### `NodeExtensions`

**Methods**

| Method | Description |
| --- | --- |
| <code>attach&lt;T: 'static&gt;(&amp;mut self, value: T) -&gt; Option&lt;T&gt;</code> |  |
| <code>get&lt;T: 'static&gt;(&amp;self) -&gt; Option&lt;&amp;T&gt;</code> |  |
| <code>get_mut&lt;T: 'static&gt;(&amp;mut self) -&gt; Option&lt;&amp;mut T&gt;</code> |  |
| <code>is_empty(&amp;self) -&gt; bool</code> |  |
| <code>remove&lt;T: 'static&gt;(&amp;mut self) -&gt; Option&lt;T&gt;</code> |  |

**Trait implementations:** <code>Default</code>, <code>Debug</code>


### `NodeId`

**Methods**

| Method | Description |
| --- | --- |
| <code>from_accesskit(id: NodeId) -&gt; Self</code> |  |
| <code>to_accesskit(self) -&gt; NodeId</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>Ord</code>, <code>PartialEq</code>, <code>PartialOrd</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `NodeTag`

**Methods**

| Method | Description |
| --- | --- |
| <code>const id(self) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>record(self, kind: <a href="#deltakind">DeltaKind</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StatusView`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#statusview">StatusView</a>&lt;T, E&gt;::error&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, view: impl Fn(&amp;E) -&gt; N + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#statusview">StatusView</a>&lt;T, E&gt;::idle&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, view: impl Fn() -&gt; N + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#statusview">StatusView</a>&lt;T, E&gt;::ok&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, view: impl Fn(&amp;T) -&gt; N + 'static) -&gt; Self</code> |  |
| <code>pub fn <a href="#statusview">StatusView</a>&lt;T, E&gt;::pending&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, view: impl Fn() -&gt; N + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="#intonode">IntoNode</a></code>


### `Surface`

**Fields**

| Field | Description |
| --- | --- |
| <code>key: u64</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `SwitchBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#switchbuilder">SwitchBuilder</a>&lt;K&gt;::case&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, key: K, view: impl Fn() -&gt; N + 'static) -&gt; Self</code> |  |
| <code>pub const fn <a href="#switchbuilder">SwitchBuilder</a>&lt;K&gt;::keep_alive(self, keep: bool) -&gt; Self</code> |  |
| <code>pub fn <a href="#switchbuilder">SwitchBuilder</a>&lt;K&gt;::otherwise&lt;N: <a href="#intonode">IntoNode</a> + 'static&gt;(self, view: impl Fn() -&gt; N + 'static) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="#intonode">IntoNode</a></code>


### `TextBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>add(self, cx: &amp;mut <a href="#cx">Cx</a>) -&gt; <a href="#nodeid">NodeId</a></code> |  |
| <code>description(self, description: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const ellipsize(self) -&gt; Self</code> |  |
| <code>const focusable(self, yes: bool) -&gt; Self</code> |  |
| <code>const hittable(self, yes: bool) -&gt; Self</code> |  |
| <code>keyboard_shortcut(self, shortcut: impl Into&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;) -&gt; Self</code> |  |
| <code>label(self, name: impl Into&lt;<a href="#accessiblename">AccessibleName</a>&gt;) -&gt; Self</code> |  |
| <code>mutate(self, apply: impl FnOnce(&amp;mut <a href="#node">Node</a>) + 'static) -&gt; Self</code> |  |
| <code>const nowrap(self) -&gt; Self</code> |  |
| <code>const role(self, role: Role) -&gt; Self</code> |  |
| <code>runs(self, runs: Vec&lt;<a href="sigil-style.html#textrun">TextRun</a>&gt;) -&gt; Self</code> |  |
| <code>const tab_index(self, index: i32) -&gt; Self</code> |  |
| <code>const wrap(self) -&gt; Self</code> |  |

**Trait implementations:** <code><a href="#intonode">IntoNode</a></code>, <code><a href="sigil-layout.html#layoutbuilderext">LayoutBuilderExt</a></code>, <code><a href="sigil-style.html#nodestyleext">NodeStyleExt</a></code>


### `TextCursor`

**Fields**

| Field | Description |
| --- | --- |
| <code>anchor: <a href="sigil-reactive.html#signal">Signal</a>&lt;Option&lt;usize&gt;&gt;</code> |  |
| <code>caret: <a href="sigil-reactive.html#signal">Signal</a>&lt;usize&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `TopLayer`

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


## Enums

### `AccessibleName`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Derived(<a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;)</code> |  |
| <code>None</code> |  |
| <code>Static(String)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>From&lt;&amp;'static str&gt;</code>, <code>From&lt;String&gt;</code>, <code>From&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;&gt;</code>, <code>Debug</code>


### `DeltaKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>const layout_if(affects_layout: bool) -&gt; Self</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Layout</code> |  |
| <code>Paint</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FocusScopeKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Inert</code> |  |
| <code>Modal</code> |  |
| <code>Popup</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `HostMode`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cell</code> |  |
| <code>Pixel</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `NodeType`

**Fields**

| Field | Description |
| --- | --- |
| <code>Image::fit: <a href="#objectfit">ObjectFit</a></code> |  |
| <code>Image::source: <a href="#imagesource">ImageSource</a></code> |  |
| <code>Text::content: <a href="#textcontent">TextContent</a></code> |  |
| <code>Text::ellipsis: bool</code> |  |
| <code>Text::wrap: bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Container</code> |  |
| <code>Fragment</code> |  |
| <code>Image</code> |  |
| <code>Text</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `ObjectFit`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Contain</code> |  |
| <code>Cover</code> |  |
| <code>Fill</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextContent`

**Methods**

| Method | Description |
| --- | --- |
| <code>as_static_str(&amp;self) -&gt; Option&lt;&amp;str&gt;</code> |  |
| <code>refresh(&amp;self, out: &amp;mut String) -&gt; bool</code> |  |
| <code>resolve(&amp;self) -&gt; String</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compute(Rc&lt;dyn Fn() -&gt; String&gt;)</code> |  |
| <code>Reactive(<a href="sigil-reactive.html#signal">Signal</a>&lt;String&gt;)</code> |  |
| <code>Static(String)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>From&lt;&amp;str&gt;</code>, <code>From&lt;String&gt;</code>, <code>Debug</code>, <code>String + 'static&gt; From&lt;F&gt;</code>, <code>From&lt;<a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;&gt;</code>


## Traits

### `IntoNode`

**Methods**

| Method | Description |
| --- | --- |
| <code>build(self, cx: &amp;mut <a href="#cx">Cx</a>) -&gt; <a href="#nodeid">NodeId</a></code> |  |


## Functions

### `announce`

<pre>pub fn <a href="#announce">announce</a>(text: impl Into&lt;String&gt;)</pre>


### `announce_assertive`

<pre>pub fn <a href="#announce_assertive">announce_assertive</a>(text: impl Into&lt;String&gt;)</pre>


### `container`

<pre>pub fn <a href="#container">container</a>() -&gt; <a href="#nodebuilder">NodeBuilder</a></pre>


### `for_each`

<pre>pub fn <a href="#for_each">for_each</a>&lt;T, K, N&gt;(items: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;T&gt;&gt;, key: impl Fn(&amp;T) -&gt; K + 'static, render: impl Fn(&amp;T) -&gt; N + 'static) -&gt; <a href="#nodebuilder">NodeBuilder</a> where T: 'static, K: Clone + Eq + Hash + 'static, N: <a href="#intonode">IntoNode</a> + 'static</pre>


### `fragment`

<pre>pub fn <a href="#fragment">fragment</a>() -&gt; <a href="#nodebuilder">NodeBuilder</a></pre>


### `image`

<pre>pub fn <a href="#image">image</a>(source: <a href="#imagesource">ImageSource</a>) -&gt; <a href="#imagebuilder">ImageBuilder</a></pre>


### `match_status`

<pre>pub fn <a href="#match_status">match_status</a>&lt;T: 'static, E: 'static&gt;(status: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;<a href="sigil-tasks.html#status">Status</a>&lt;T, E&gt;&gt;) -&gt; <a href="#statusview">StatusView</a>&lt;T, E&gt;</pre>


### `node`

<pre>pub fn <a href="#node-1">node</a>(child: impl <a href="#intonode">IntoNode</a> + 'static) -&gt; Box&lt;dyn <a href="#intonode">IntoNode</a>&gt;</pre>


### `schedule_keyed_rebuild`

<pre>pub fn <a href="#schedule_keyed_rebuild">schedule_keyed_rebuild</a>&lt;K, N&gt;(parent: <a href="#nodeid">NodeId</a>, prior: Rc&lt;RefCell&lt;Vec&lt;(K, <a href="#nodeid">NodeId</a>)&gt;&gt;&gt;, incoming: Vec&lt;(K, Option&lt;N&gt;)&gt;) where K: Clone + Eq + Hash + 'static, N: <a href="#intonode">IntoNode</a> + 'static</pre>


### `show`

<pre>pub fn <a href="#show">show</a>&lt;A, B&gt;(cond: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;, then: impl Fn() -&gt; A + 'static, otherwise: impl Fn() -&gt; B + 'static) -&gt; <a href="#nodebuilder">NodeBuilder</a> where A: <a href="#intonode">IntoNode</a> + 'static, B: <a href="#intonode">IntoNode</a> + 'static</pre>


### `switch`

<pre>pub fn <a href="#switch">switch</a>&lt;K: Clone + PartialEq + 'static&gt;(key: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;K&gt;) -&gt; <a href="#switchbuilder">SwitchBuilder</a>&lt;K&gt;</pre>


### `text`

<pre>pub fn <a href="#text">text</a>(source: impl Into&lt;<a href="#textcontent">TextContent</a>&gt;) -&gt; <a href="#textbuilder">TextBuilder</a></pre>


## Re-exports

### `Action`


### `Live`


### `Role`


### `SortDirection`


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
