# Module `sigil_kittest`

| Item | Kind | Description |
| --- | --- | --- |
| [`Captured`](#captured) | struct |  |
| [`CharMeasure`](#charmeasure) | struct |  |
| [`FixedMeasure`](#fixedmeasure) | struct |  |
| [`Harness`](#harness) | struct |  |
| [`Node`](#node) | struct |  |
| [`Query`](#query) | struct |  |
| [`WithCx`](#withcx) | struct |  |
| [`Error`](#error) | enum |  |
| [`InteractionMode`](#interactionmode) | enum |  |
| [`Regen`](#regen) | enum |  |
| [`Target`](#target) | trait |  |
| [`assert_goldens_differ_in`](#assert_goldens_differ_in) | fn |  |
| [`assert_png_matches_golden`](#assert_png_matches_golden) | fn |  |
| [`assert_rendered_matches_golden`](#assert_rendered_matches_golden) | fn |  |
| [`assert_text_matches_golden`](#assert_text_matches_golden) | fn |  |
| [`capture_filmstrip`](#capture_filmstrip) | fn |  |
| [`capture_widget_scaled_focus_descendant`](#capture_widget_scaled_focus_descendant) | fn |  |
| [`golden_path_in`](#golden_path_in) | fn |  |
| [`real_text_measure`](#real_text_measure) | fn |  |
| [`regen_enabled`](#regen_enabled) | fn |  |
| [`regen_mode`](#regen_mode) | fn |  |
| [`render_widget_to_png`](#render_widget_to_png) | fn |  |
| [`render_widget_to_png_focus_descendant`](#render_widget_to_png_focus_descendant) | fn |  |
| [`render_widget_to_png_scaled`](#render_widget_to_png_scaled) | fn |  |
| [`render_widget_to_png_scaled_focus_descendant`](#render_widget_to_png_scaled_focus_descendant) | fn |  |
| [`render_widget_to_png_stub`](#render_widget_to_png_stub) | fn |  |
| [`render_widget_to_png_with`](#render_widget_to_png_with) | fn |  |
| [`stub_text_measure`](#stub_text_measure) | fn |  |
| [`Rendered`](#rendered) | type |  |
| [`EXPECT_TIMEOUT`](#expect_timeout) | const |  |
| [`FILMSTRIP_GAP_PX`](#filmstrip_gap_px) | const |  |
| [`SETTLE_SAFETY_CAP`](#settle_safety_cap) | const |  |
| [`SNAPSHOT_MAX_DELTA_PCT`](#snapshot_max_delta_pct) | const |  |
| [`assert_goldens_differ`](#assert_goldens_differ) | macro |  |
| [`golden_path`](#golden_path) | macro |  |
| [`sigil::hal::geometry::Point`](#sigilhalgeometrypoint) | impl |  |
| [`str`](#str) | impl |  |

## Structs

### `Captured`

**Fields**

| Field | Description |
| --- | --- |
| <code>image: RgbaImage</code> |  |
| <code>top_layer: Option&lt;Rect&gt;</code> |  |


### `CharMeasure`

**Fields**

| Field | Description |
| --- | --- |
| <code>char_width: f32</code> |  |
| <code>line_height: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>, <code>TextMeasure</code>


### `FixedMeasure`

**Fields**

| Field | Description |
| --- | --- |
| <code>height: f32</code> |  |
| <code>width: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>, <code>TextMeasure</code>


### `Harness`

**Methods**

| Method | Description |
| --- | --- |
| <code>accessibility_snapshot(&amp;mut self) -&gt; String</code> |  |
| <code>accessibility_tree(&amp;mut self) -&gt; AccessibilityTree</code> |  |
| <code>advance(&amp;mut self, by: Duration) -&gt; &amp;mut Self</code> |  |
| <code>assert_aligned(&amp;mut self)</code> |  |
| <code>gpu(self, width: u32, height: u32, scale: f32) -&gt; Self</code> |  |
| <code>host(&amp;mut self) -&gt; (&amp;mut Host, &amp;mut EventRouter)</code> |  |
| <code>misalignments(&amp;mut self) -&gt; Vec&lt;Misalignment&gt;</code> |  |
| <code>new(app: impl IntoNode + 'static) -&gt; Self</code> |  |
| <code>pixels(self, width: u32, height: u32) -&gt; Self</code> |  |
| <code>send(&amp;mut self, events: impl IntoIterator&lt;Item = ShellEvent&gt;) -&gt; &amp;mut Self</code> |  |
| <code>settle(&amp;mut self) -&gt; &amp;mut Self</code> |  |
| <code>size(self, cols: u16, rows: u16) -&gt; Self</code> |  |
| <code>text(&amp;mut self) -&gt; String</code> |  |
| <code>click(&amp;mut self, target: &amp;impl <a href="#target">Target</a> + ?Sized) -&gt; &amp;mut Self</code> |  |
| <code>drag(&amp;mut self, from: &amp;impl <a href="#target">Target</a> + ?Sized, to: &amp;impl <a href="#target">Target</a> + ?Sized) -&gt; &amp;mut Self</code> |  |
| <code>focus_next(&amp;mut self) -&gt; &amp;mut Self</code> |  |
| <code>focus_previous(&amp;mut self) -&gt; &amp;mut Self</code> |  |
| <code>hover(&amp;mut self, target: &amp;impl <a href="#target">Target</a> + ?Sized) -&gt; &amp;mut Self</code> |  |
| <code>press(&amp;mut self, key: impl Into&lt;LogicalKey&gt;) -&gt; &amp;mut Self</code> |  |
| <code>press_with(&amp;mut self, key: impl Into&lt;LogicalKey&gt;, modifiers: Modifiers) -&gt; &amp;mut Self</code> |  |
| <code>scroll(&amp;mut self, target: &amp;impl <a href="#target">Target</a> + ?Sized, lines: f32) -&gt; &amp;mut Self</code> |  |
| <code>type_into(&amp;mut self, label: &amp;str, text: &amp;str) -&gt; &amp;mut Self</code> |  |
| <code>type_text(&amp;mut self, text: &amp;str) -&gt; &amp;mut Self</code> |  |
| <code>expect_text(&amp;mut self, text: &amp;str) -&gt; &amp;mut Self</code> |  |
| <code>find_all(&amp;mut self, query: &amp;<a href="#query">Query</a>) -&gt; Vec&lt;<a href="#node">Node</a>&gt;</code> |  |
| <code>focused(&amp;mut self) -&gt; Option&lt;<a href="#node">Node</a>&gt;</code> |  |
| <code>get(&amp;mut self, query: &amp;<a href="#query">Query</a>) -&gt; <a href="#node">Node</a></code> |  |
| <code>get_by_role(&amp;mut self, role: Role, name: &amp;str) -&gt; <a href="#node">Node</a></code> |  |
| <code>get_by_text(&amp;mut self, text: &amp;str) -&gt; <a href="#node">Node</a></code> |  |
| <code>query(&amp;mut self, query: &amp;<a href="#query">Query</a>) -&gt; Option&lt;<a href="#node">Node</a>&gt;</code> |  |
| <code>refresh(&amp;mut self, node: &amp;<a href="#node">Node</a>) -&gt; Option&lt;<a href="#node">Node</a>&gt;</code> |  |
| <code>image(&amp;mut self) -&gt; <a href="#rendered">Rendered</a></code> |  |
| <code>snapshot(&amp;mut self, name: &amp;str)</code> |  |
| <code>snapshot_with(&amp;mut self, name: &amp;str, max_delta_pct: f32)</code> |  |

**Trait implementations:** <code>Drop</code>


### `Node`

**Methods**

| Method | Description |
| --- | --- |
| <code>const bounds(&amp;self) -&gt; Option&lt;Rect&gt;</code> |  |
| <code>center(&amp;self) -&gt; Option&lt;Point&gt;</code> |  |
| <code>const is_checked(&amp;self) -&gt; Option&lt;bool&gt;</code> |  |
| <code>const is_disabled(&amp;self) -&gt; bool</code> |  |
| <code>const is_expanded(&amp;self) -&gt; Option&lt;bool&gt;</code> |  |
| <code>const is_focused(&amp;self) -&gt; bool</code> |  |
| <code>const is_selected(&amp;self) -&gt; Option&lt;bool&gt;</code> |  |
| <code>name(&amp;self) -&gt; Option&lt;&amp;str&gt;</code> |  |
| <code>const numeric(&amp;self) -&gt; Option&lt;f64&gt;</code> |  |
| <code>const role(&amp;self) -&gt; Role</code> |  |
| <code>value(&amp;self) -&gt; Option&lt;&amp;str&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>, <code><a href="#target">Target</a></code>


### `Query`

**Methods**

| Method | Description |
| --- | --- |
| <code>name(self, name: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>role(role: Role) -&gt; Self</code> |  |
| <code>text(text: impl Into&lt;String&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>, <code>Display</code>


### `WithCx`

**Trait implementations:** <code>IntoNode</code>


## Enums

### `Error`

**Fields**

| Field | Description |
| --- | --- |
| <code>Unsettled::active: usize</code> |  |
| <code>Unsettled::frames: u32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Device(String)</code> |  |
| <code>Layout(String)</code> |  |
| <code>NoAdapter(String)</code> |  |
| <code>Readback(String)</code> |  |
| <code>Render(String)</code> |  |
| <code>Unsettled</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>


### `InteractionMode`

**Methods**

| Method | Description |
| --- | --- |
| <code>const tag(self) -&gt; &amp;'static str</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>Dragged::dx: f32</code> |  |
| <code>Dragged::dy: f32</code> |  |
| <code>FocusedAfterFrames::dt: f32</code> |  |
| <code>FocusedAfterFrames::frames: u32</code> |  |
| <code>HoverAfterFrames::dt: f32</code> |  |
| <code>HoverAfterFrames::frames: u32</code> |  |
| <code>PressedAfterFrames::dt: f32</code> |  |
| <code>PressedAfterFrames::frames: u32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Clicked</code> |  |
| <code>Default</code> |  |
| <code>Disabled</code> |  |
| <code>Dragged</code> |  |
| <code>Focused</code> |  |
| <code>FocusedAfterFrames</code> |  |
| <code>Hover</code> |  |
| <code>HoverAfterFrames</code> |  |
| <code>Pressed</code> |  |
| <code>PressedAfterFrames</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `Regen`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Adapter</code> |  |
| <code>All</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `Target`

**Methods**

| Method | Description |
| --- | --- |
| <code>point(&amp;self, ui: &amp;mut <a href="#harness">Harness</a>) -&gt; Point</code> |  |


## Functions

### `assert_goldens_differ_in`

<pre>pub fn <a href="#assert_goldens_differ_in">assert_goldens_differ_in</a>(manifest_dir: &amp;str, name_a: &amp;str, name_b: &amp;str)</pre>


### `assert_png_matches_golden`

<pre>pub fn <a href="#assert_png_matches_golden">assert_png_matches_golden</a>(actual: &amp;RgbaImage, golden: &amp;Path, max_delta_pct: f32)</pre>


### `assert_rendered_matches_golden`

<pre>pub fn <a href="#assert_rendered_matches_golden">assert_rendered_matches_golden</a>(rendered: <a href="#rendered">Rendered</a>, golden: &amp;Path, max_delta_pct: f32)</pre>


### `assert_text_matches_golden`

<pre>pub fn <a href="#assert_text_matches_golden">assert_text_matches_golden</a>(actual: &amp;str, golden: &amp;Path)</pre>


### `capture_filmstrip`

<pre>pub fn <a href="#capture_filmstrip">capture_filmstrip</a>&lt;N: IntoNode&gt;(build: impl Fn() -&gt; N, size_px: [u32; 2], hit_point: (f32, f32), frame_count: u32, dt: f32, text_measure: impl Fn() -&gt; Box&lt;dyn TextMeasure&gt;) -&gt; <a href="#rendered">Rendered</a></pre>


### `capture_widget_scaled_focus_descendant`

<pre>pub fn <a href="#capture_widget_scaled_focus_descendant">capture_widget_scaled_focus_descendant</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, logical_size_px: [u32; 2], scale: f32, hit_point: (f32, f32), text_measure: Box&lt;dyn TextMeasure&gt;, settle: &amp;mut dyn FnMut() -&gt; bool) -&gt; Result&lt;Option&lt;<a href="#captured">Captured</a>&gt;, <a href="#error">Error</a>&gt;</pre>


### `golden_path_in`

<pre>pub fn <a href="#golden_path_in">golden_path_in</a>(manifest_dir: &amp;str, stem: &amp;str) -&gt; PathBuf</pre>


### `real_text_measure`

<pre>pub fn <a href="#real_text_measure">real_text_measure</a>() -&gt; Box&lt;dyn TextMeasure&gt;</pre>


### `regen_enabled`

<pre>pub fn <a href="#regen_enabled">regen_enabled</a>() -&gt; bool</pre>


### `regen_mode`

<pre>pub fn <a href="#regen_mode">regen_mode</a>() -&gt; Option&lt;<a href="#regen">Regen</a>&gt;</pre>


### `render_widget_to_png`

<pre>pub fn <a href="#render_widget_to_png">render_widget_to_png</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, size_px: [u32; 2], hit_point: (f32, f32)) -&gt; <a href="#rendered">Rendered</a></pre>


### `render_widget_to_png_focus_descendant`

<pre>pub fn <a href="#render_widget_to_png_focus_descendant">render_widget_to_png_focus_descendant</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, size_px: [u32; 2], hit_point: (f32, f32), text_measure: Box&lt;dyn TextMeasure&gt;) -&gt; <a href="#rendered">Rendered</a></pre>


### `render_widget_to_png_scaled`

<pre>pub fn <a href="#render_widget_to_png_scaled">render_widget_to_png_scaled</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, logical_size_px: [u32; 2], scale: f32, hit_point: (f32, f32), text_measure: Box&lt;dyn TextMeasure&gt;) -&gt; <a href="#rendered">Rendered</a></pre>


### `render_widget_to_png_scaled_focus_descendant`

<pre>pub fn <a href="#render_widget_to_png_scaled_focus_descendant">render_widget_to_png_scaled_focus_descendant</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, logical_size_px: [u32; 2], scale: f32, hit_point: (f32, f32), text_measure: Box&lt;dyn TextMeasure&gt;) -&gt; <a href="#rendered">Rendered</a></pre>


### `render_widget_to_png_stub`

<pre>pub fn <a href="#render_widget_to_png_stub">render_widget_to_png_stub</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, size_px: [u32; 2], hit_point: (f32, f32)) -&gt; <a href="#rendered">Rendered</a></pre>


### `render_widget_to_png_with`

<pre>pub fn <a href="#render_widget_to_png_with">render_widget_to_png_with</a>(build: impl IntoNode, mode: <a href="#interactionmode">InteractionMode</a>, size_px: [u32; 2], hit_point: (f32, f32), text_measure: Box&lt;dyn TextMeasure&gt;) -&gt; <a href="#rendered">Rendered</a></pre>


### `stub_text_measure`

<pre>pub fn <a href="#stub_text_measure">stub_text_measure</a>() -&gt; Box&lt;dyn TextMeasure&gt;</pre>


## Type aliases

### `Rendered`

<pre>pub type <a href="#rendered">Rendered</a> = Result&lt;Option&lt;RgbaImage&gt;, <a href="#error">Error</a>&gt;</pre>


## Constants

### `EXPECT_TIMEOUT`

<pre>pub const <a href="#expect_timeout">EXPECT_TIMEOUT</a>: Duration</pre>


### `FILMSTRIP_GAP_PX`

<pre>pub const <a href="#filmstrip_gap_px">FILMSTRIP_GAP_PX</a>: u32</pre>


### `SETTLE_SAFETY_CAP`

<pre>pub const <a href="#settle_safety_cap">SETTLE_SAFETY_CAP</a>: u32</pre>


### `SNAPSHOT_MAX_DELTA_PCT`

<pre>pub const <a href="#snapshot_max_delta_pct">SNAPSHOT_MAX_DELTA_PCT</a>: f32</pre>


## Macros

### `assert_goldens_differ`

<pre>pub macro <a href="#assert_goldens_differ">assert_goldens_differ</a>!</pre>


### `golden_path`

<pre>pub macro <a href="#golden_path">golden_path</a>!</pre>


## Implementations

### `sigil::hal::geometry::Point`

**Trait implementations:** <code><a href="#target">Target</a></code>


### `str`

**Trait implementations:** <code><a href="#target">Target</a></code>


---

Generated by truesight from `sigil-kittest` 0.1.0. See the [API overview](index.md).
