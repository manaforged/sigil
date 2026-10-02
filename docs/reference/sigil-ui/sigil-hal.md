# Module `sigil::hal`

| Item | Kind | Description |
| --- | --- | --- |
| [`HitPosition`](#hitposition) | struct |  |
| [`RenderedGlyph`](#renderedglyph) | struct |  |
| [`TextAffinity`](#textaffinity) | enum |  |
| [`A11yBridge`](#a11ybridge) | trait |  |
| [`ParagraphMetrics`](#paragraphmetrics) | trait |  |
| [`Renderer`](#renderer) | trait |  |
| [`Scheduler`](#scheduler) | trait |  |
| [`Shell`](#shell) | trait |  |
| [`TextEngine`](#textengine) | trait |  |

## Structs

### `HitPosition`

**Fields**

| Field | Description |
| --- | --- |
| <code>affinity: <a href="#textaffinity">TextAffinity</a></code> |  |
| <code>byte_index: usize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `RenderedGlyph`

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="sigil-style.html#color">Color</a></code> |  |
| <code>font_id: u64</code> |  |
| <code>font_index: u32</code> |  |
| <code>glyph_id: u32</code> |  |
| <code>size: f32</code> |  |
| <code>skew: f32</code> |  |
| <code>variation_coords: Box&lt;[i16]&gt;</code> |  |
| <code>x: f32</code> |  |
| <code>y: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


## Enums

### `TextAffinity`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Downstream</code> |  |
| <code>Upstream</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `A11yBridge`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Action</code> |  |
| <code>type Error: Error + Send + Sync + 'static</code> |  |
| <code>type NodeId: Copy + Eq + Hash</code> |  |
| <code>type Update</code> |  |
| <code>publish(&amp;mut self, update: Self::Update) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>take_action(&amp;mut self) -&gt; Option&lt;(Self::NodeId, Self::Action)&gt;</code> |  |


### `ParagraphMetrics`

**Methods**

| Method | Description |
| --- | --- |
| <code>height(&amp;self) -&gt; f32</code> |  |
| <code>line_count(&amp;self) -&gt; usize</code> |  |
| <code>max_content_width(&amp;self) -&gt; f32</code> |  |
| <code>min_content_width(&amp;self) -&gt; f32</code> |  |
| <code>width(&amp;self) -&gt; f32</code> |  |


### `Renderer`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Error: Error + Send + Sync + 'static</code> |  |
| <code>type Surface</code> |  |
| <code>type WindowHandle</code> |  |
| <code>clear(&amp;mut self, surface: &amp;mut Self::Surface, color: <a href="sigil-style.html#color">Color</a>) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>create_surface(&amp;mut self, window: Self::WindowHandle, initial_size: <a href="sigil-geometry.html#physicalsize">PhysicalSize</a>) -&gt; Result&lt;Self::Surface, Self::Error&gt;</code> |  |
| <code>resize(&amp;mut self, surface: &amp;mut Self::Surface, size: <a href="sigil-geometry.html#physicalsize">PhysicalSize</a>) -&gt; Result&lt;(), Self::Error&gt;</code> |  |


### `Scheduler`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Error: Error + Send + Sync + 'static</code> |  |
| <code>type OwnerHandle: Clone + Send + Sync + 'static</code> |  |
| <code>type SpawnTask: Send + 'static</code> |  |
| <code>drain(&amp;mut self) -&gt; usize</code> |  |
| <code>enqueue_main(&amp;self, owner: Self::OwnerHandle, f: Box&lt;(dyn FnOnce() + Send + 'static)&gt;) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>spawn&lt;F&gt;(&amp;self, owner: Self::OwnerHandle, future: F) -&gt; Self::SpawnTask where F: Future&lt;Output = ()&gt; + Send + 'static</code> |  |


### `Shell`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Error: Error + Send + Sync + 'static</code> |  |
| <code>type Waker: Clone + Send + Sync + 'static</code> |  |
| <code>type Window</code> |  |
| <code>create_window(&amp;mut self, title: &amp;str, initial_size: <a href="sigil-geometry.html#physicalsize">PhysicalSize</a>) -&gt; Result&lt;Self::Window, Self::Error&gt;</code> |  |
| <code>create_window_logical(&amp;mut self, title: &amp;str, initial: <a href="sigil-geometry.html#logicalsize">LogicalSize</a>) -&gt; Result&lt;Self::Window, Self::Error&gt;</code> |  |
| <code>create_window_with(&amp;mut self, title: &amp;str, initial: <a href="sigil-geometry.html#logicalsize">LogicalSize</a>, opts: <a href="sigil-app.html#windowopts">WindowOpts</a>) -&gt; Result&lt;Self::Window, Self::Error&gt;</code> |  |
| <code>drain_events(&amp;mut self) -&gt; IntoIter&lt;<a href="sigil-events.html#shellevent">ShellEvent</a>&gt;</code> |  |
| <code>run&lt;F&gt;(self, frame: F) -&gt; Result&lt;(), Self::Error&gt; where F: FnMut(&amp;mut Self) + 'static</code> |  |
| <code>set_ime_allowed(&amp;mut self, window: &amp;Self::Window, allowed: bool) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>set_ime_cursor_area(&amp;mut self, window: &amp;Self::Window, rect: <a href="sigil-geometry.html#rect">Rect</a>) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>wake(waker: &amp;Self::Waker) -&gt; Result&lt;(), Self::Error&gt;</code> |  |
| <code>waker(&amp;self) -&gt; Self::Waker</code> |  |
| <code>window_command(&amp;mut self, window: &amp;Self::Window, command: <a href="sigil-app.html#windowcommand">WindowCommand</a>) -&gt; Result&lt;(), Self::Error&gt;</code> |  |


### `TextEngine`

**Methods**

| Method | Description |
| --- | --- |
| <code>type Error: Error + Send + Sync + 'static</code> |  |
| <code>type Paragraph: <a href="#paragraphmetrics">ParagraphMetrics</a></code> |  |
| <code>clear_cache(&amp;self)</code> |  |
| <code>layout_paragraph(&amp;self, text: &amp;str, max_width: Option&lt;f32&gt;) -&gt; Result&lt;Self::Paragraph, Self::Error&gt;</code> |  |


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
