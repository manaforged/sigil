# Module `sigil::app`

| Item | Kind | Description |
| --- | --- | --- |
| [`DefaultFonts`](#defaultfonts) | struct |  |
| [`DefaultTextMeasure`](#defaulttextmeasure) | struct |  |
| [`EngineGeometry`](#enginegeometry) | struct |  |
| [`FrameHooks`](#framehooks) | struct |  |
| [`FrameInfo`](#frameinfo) | struct |  |
| [`FrameOutput`](#frameoutput) | struct |  |
| [`HostCaps`](#hostcaps) | struct |  |
| [`Player`](#player) | struct |  |
| [`Session`](#session) | struct |  |
| [`TextTuning`](#texttuning) | struct |  |
| [`ThemeWatcher`](#themewatcher) | struct |  |
| [`WindowOpts`](#windowopts) | struct |  |
| [`Ambiguous`](#ambiguous) | enum |  |
| [`Anchor`](#anchor) | enum |  |
| [`CellWidth`](#cellwidth) | enum |  |
| [`ErrorKind`](#errorkind) | enum |  |
| [`PanicPolicy`](#panicpolicy) | enum |  |
| [`ResizeDirection`](#resizedirection) | enum |  |
| [`WindowCommand`](#windowcommand) | enum |  |
| [`cell_text_lines`](#cell_text_lines) | fn |  |
| [`cell_width`](#cell_width) | fn |  |
| [`display_scale`](#display_scale) | fn |  |
| [`is_grid`](#is_grid) | fn |  |
| [`open`](#open) | fn |  |
| [`parse_theme`](#parse_theme) | fn |  |
| [`set_ambiguous`](#set_ambiguous) | fn |  |
| [`set_cell_width`](#set_cell_width) | fn |  |
| [`set_default_fonts`](#set_default_fonts) | fn |  |
| [`set_text_tuning`](#set_text_tuning) | fn |  |
| [`set_theme`](#set_theme) | fn |  |
| [`text_tuning`](#text_tuning) | fn |  |
| [`use_system_fonts`](#use_system_fonts) | fn |  |
| [`viewport_size`](#viewport_size) | fn |  |
| [`watch_theme`](#watch_theme) | fn |  |
| [`AccessibilityTree`](#accessibilitytree) | type |  |
| [`FrameInputHook`](#frameinputhook) | type |  |
| [`FrameLayoutHook`](#framelayouthook) | type |  |
| [`FramePaintHook`](#framepainthook) | type |  |

## Structs

### `DefaultFonts`

**Fields**

| Field | Description |
| --- | --- |
| <code>display: Option&lt;(Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |
| <code>fallbacks: Vec&lt;(Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |
| <code>icon: Option&lt;(Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |
| <code>mono: Option&lt;(Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |
| <code>sans: Option&lt;(Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |
| <code>sans_weights: Vec&lt;(u16, Arc&lt;Vec&lt;u8&gt;&gt;, u32)&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>


### `DefaultTextMeasure`

**Methods**

| Method | Description |
| --- | --- |
| <code>new() -&gt; Self</code> |  |

**Trait implementations:** <code>Default</code>, <code><a href="sigil-layout.html#textmeasure">TextMeasure</a></code>


### `EngineGeometry`

**Methods**

| Method | Description |
| --- | --- |
| <code>new() -&gt; Self</code> |  |

**Trait implementations:** <code>Default</code>, <code><a href="sigil-layout.html#textgeometry">TextGeometry</a></code>


### `FrameHooks`

**Fields**

| Field | Description |
| --- | --- |
| <code>after_layout: Option&lt;<a href="#framelayouthook">FrameLayoutHook</a>&lt;'a&gt;&gt;</code> |  |
| <code>input: Option&lt;<a href="#frameinputhook">FrameInputHook</a>&lt;'a&gt;&gt;</code> |  |
| <code>paint: Option&lt;<a href="#framepainthook">FramePaintHook</a>&lt;'a&gt;&gt;</code> |  |

**Trait implementations:** <code>Default</code>


### `FrameInfo`

**Fields**

| Field | Description |
| --- | --- |
| <code>animating: bool</code> |  |
| <code>dirty: bool</code> |  |
| <code>layout_ok: bool</code> |  |


### `FrameOutput`

**Fields**

| Field | Description |
| --- | --- |
| <code>animating: bool</code> |  |
| <code>exit: bool</code> |  |
| <code>intervals: usize</code> |  |
| <code>layout: Result&lt;(), <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</code> |  |


### `HostCaps`

**Methods**

| Method | Description |
| --- | --- |
| <code>current() -&gt; Self</code> |  |
| <code>const window() -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>hyperlinks: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Player`

**Methods**

| Method | Description |
| --- | --- |
| <code>new() -&gt; Self</code> |  |
| <code>play_path(&amp;self, path: &amp;Path) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;()&gt;</code> |  |
| <code>seek(&amp;self, ratio: f32) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;()&gt;</code> |  |
| <code>tick(&amp;self)</code> |  |
| <code>toggle(&amp;self)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>playing: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;</code> |  |
| <code>progress: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>


### `Session`

**Methods**

| Method | Description |
| --- | --- |
| <code>spawn(program: &amp;str) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;Self&gt;</code> |  |
| <code>write(&amp;self, bytes: &amp;[u8]) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;()&gt;</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>lines: <a href="sigil-reactive.html#signal">Signal</a>&lt;Vec&lt;String&gt;&gt;</code> |  |

**Trait implementations:** <code>Drop</code>


### `TextTuning`

**Fields**

| Field | Description |
| --- | --- |
| <code>dilation: f32</code> |  |
| <code>enabled: bool</code> |  |
| <code>gamma_dark_on_light: f32</code> |  |
| <code>gamma_light_on_dark: f32</code> |  |
| <code>raster: bool</code> |  |
| <code>sharpen: f32</code> |  |
| <code>snap: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ThemeWatcher`

**Trait implementations:** <code>Drop</code>


### `WindowOpts`

**Fields**

| Field | Description |
| --- | --- |
| <code>anchor: Option&lt;<a href="#anchor">Anchor</a>&gt;</code> |  |
| <code>fixed_size: bool</code> |  |
| <code>frameless: bool</code> |  |
| <code>min_size: Option&lt;<a href="sigil-geometry.html#logicalsize">LogicalSize</a>&gt;</code> |  |
| <code>overlay: bool</code> |  |
| <code>user_zoom: Option&lt;f32&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Enums

### `Ambiguous`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Narrow</code> |  |
| <code>Wide</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Anchor`

**Fields**

| Field | Description |
| --- | --- |
| <code>BottomCenter::margin: f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>BottomCenter</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `CellWidth`

**Fields**

| Field | Description |
| --- | --- |
| <code>Scalar::joiner: usize</code> |  |
| <code>Scalar::keycap: usize</code> |  |
| <code>Scalar::nonjoiner: usize</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Scalar</code> |  |
| <code>Unicode</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Copy</code>


### `ErrorKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Config</code> |  |
| <code>Font</code> |  |
| <code>Gpu</code> |  |
| <code>Io</code> |  |
| <code>Other</code> |  |
| <code>Task</code> |  |
| <code>Window</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PanicPolicy`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Abort</code> |  |
| <code>Isolate</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ResizeDirection`

**Variants**

| Variant | Description |
| --- | --- |
| <code>East</code> |  |
| <code>North</code> |  |
| <code>NorthEast</code> |  |
| <code>NorthWest</code> |  |
| <code>South</code> |  |
| <code>SouthEast</code> |  |
| <code>SouthWest</code> |  |
| <code>West</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `WindowCommand`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Close</code> |  |
| <code>Drag</code> |  |
| <code>DragResize(<a href="#resizedirection">ResizeDirection</a>)</code> |  |
| <code>Minimize</code> |  |
| <code>SetZoom(u16)</code> |  |
| <code>ToggleMaximize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `cell_text_lines`

<pre>pub fn <a href="#cell_text_lines">cell_text_lines</a>(text: &amp;str, max_cells: usize) -&gt; Vec&lt;(usize, usize)&gt;</pre>


### `cell_width`

<pre>pub fn <a href="#cell_width">cell_width</a>(text: &amp;str) -&gt; usize</pre>


### `display_scale`

<pre>pub fn <a href="#display_scale">display_scale</a>() -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;</pre>


### `is_grid`

<pre>pub fn <a href="#is_grid">is_grid</a>() -&gt; bool</pre>


### `open`

<pre>pub fn <a href="#open">open</a>(target: impl AsRef&lt;OsStr&gt;) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;()&gt;</pre>


### `parse_theme`

<pre>pub fn <a href="#parse_theme">parse_theme</a>(input: &amp;str) -&gt; Option&lt;<a href="sigil-style.html#theme">Theme</a>&gt;</pre>


### `set_ambiguous`

<pre>pub fn <a href="#set_ambiguous">set_ambiguous</a>(width: <a href="#ambiguous">Ambiguous</a>)</pre>


### `set_cell_width`

<pre>pub fn <a href="#set_cell_width">set_cell_width</a>(width: <a href="#cellwidth">CellWidth</a>)</pre>


### `set_default_fonts`

<pre>pub fn <a href="#set_default_fonts">set_default_fonts</a>(fonts: <a href="#defaultfonts">DefaultFonts</a>)</pre>


### `set_text_tuning`

<pre>pub fn <a href="#set_text_tuning">set_text_tuning</a>(tuning: <a href="#texttuning">TextTuning</a>)</pre>


### `set_theme`

<pre>pub fn <a href="#set_theme">set_theme</a>(theme: <a href="sigil-style.html#theme">Theme</a>)</pre>


### `text_tuning`

<pre>pub fn <a href="#text_tuning">text_tuning</a>() -&gt; <a href="#texttuning">TextTuning</a></pre>


### `use_system_fonts`

<pre>pub fn <a href="#use_system_fonts">use_system_fonts</a>()</pre>


### `viewport_size`

<pre>pub fn <a href="#viewport_size">viewport_size</a>() -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="sigil-geometry.html#logicalsize">LogicalSize</a>&gt;</pre>


### `watch_theme`

<pre>pub fn <a href="#watch_theme">watch_theme</a>(path: impl Into&lt;PathBuf&gt;) -&gt; <a href="sigil.html#appresult">AppResult</a>&lt;<a href="#themewatcher">ThemeWatcher</a>&gt;</pre>


## Type aliases

### `AccessibilityTree`

<pre>pub type <a href="#accessibilitytree">AccessibilityTree</a> = TreeUpdate</pre>


### `FrameInputHook`

<pre>pub type <a href="#frameinputhook">FrameInputHook</a>&lt;'a&gt; = Box&lt;(dyn FnOnce(&amp;mut <a href="sigil.html#host">Host</a>, &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>) -&gt; Option&lt;(f32, f32)&gt; + 'a)&gt;</pre>


### `FrameLayoutHook`

<pre>pub type <a href="#framelayouthook">FrameLayoutHook</a>&lt;'a&gt; = Box&lt;(dyn FnOnce(&amp;mut <a href="sigil.html#host">Host</a>, &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>) + 'a)&gt;</pre>


### `FramePaintHook`

<pre>pub type <a href="#framepainthook">FramePaintHook</a>&lt;'a&gt; = Box&lt;(dyn FnOnce(&amp;mut <a href="sigil.html#host">Host</a>, &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, &amp;<a href="#frameinfo">FrameInfo</a>) + 'a)&gt;</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
