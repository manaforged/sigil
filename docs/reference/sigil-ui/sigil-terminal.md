# Module `sigil::terminal`

| Item | Kind | Description |
| --- | --- | --- |
| [`Access`](#access) | struct |  |
| [`Caps`](#caps) | struct |  |
| [`Cell`](#cell) | struct |  |
| [`Grid`](#grid) | struct |  |
| [`Hooks`](#hooks) | struct |  |
| [`Lines`](#lines) | struct |  |
| [`Marks`](#marks) | struct |  |
| [`Paint`](#paint) | struct |  |
| [`Placement`](#placement) | struct |  |
| [`Screen`](#screen) | struct |  |
| [`Backdrop`](#backdrop) | enum |  |
| [`Borders`](#borders) | enum |  |
| [`ClipRoute`](#cliproute) | enum |  |
| [`ColorDepth`](#colordepth) | enum |  |
| [`Cursor`](#cursor) | enum |  |
| [`Erase`](#erase) | enum |  |
| [`FocusCursor`](#focuscursor) | enum |  |
| [`Glyphs`](#glyphs) | enum |  |
| [`Ink`](#ink) | enum |  |
| [`Keyboard`](#keyboard) | enum |  |
| [`MouseTracking`](#mousetracking) | enum |  |
| [`Mux`](#mux) | enum |  |
| [`PrintAboveError`](#printaboveerror) | enum |  |
| [`Protocol`](#protocol) | enum |  |
| [`Underline`](#underline) | enum |  |
| [`base64`](#base64) | fn |  |
| [`bell`](#bell) | fn |  |
| [`blocks`](#blocks) | fn |  |
| [`blocks_window`](#blocks_window) | fn |  |
| [`calibrate`](#calibrate) | fn |  |
| [`cell_point`](#cell_point) | fn |  |
| [`click`](#click) | fn |  |
| [`detect`](#detect) | fn |  |
| [`detect_mux`](#detect_mux) | fn |  |
| [`focused`](#focused) | fn |  |
| [`glyphs`](#glyphs-1) | fn |  |
| [`install_clipboard`](#install_clipboard) | fn |  |
| [`key_down`](#key_down) | fn |  |
| [`map_key`](#map_key) | fn |  |
| [`map_mouse`](#map_mouse) | fn |  |
| [`paste`](#paste) | fn |  |
| [`png`](#png) | fn |  |
| [`png_window`](#png_window) | fn |  |
| [`pointer`](#pointer) | fn |  |
| [`present`](#present) | fn |  |
| [`print_above`](#print_above) | fn |  |
| [`raster`](#raster) | fn |  |
| [`raster_into`](#raster_into) | fn |  |
| [`raster_with`](#raster_with) | fn |  |
| [`render`](#render) | fn |  |
| [`set_mouse_capture`](#set_mouse_capture) | fn |  |
| [`set_mouse_tracking`](#set_mouse_tracking) | fn |  |
| [`set_title`](#set_title) | fn |  |
| [`sextants`](#sextants) | fn |  |
| [`sextants_window`](#sextants_window) | fn |  |
| [`step`](#step) | fn |  |
| [`suspend`](#suspend) | fn |  |
| [`wheel`](#wheel) | fn |  |
| [`AfterFrame`](#afterframe) | type |  |
| [`EventPump`](#eventpump) | type |  |
| [`CONTINUES`](#continues) | const |  |

## Structs

### `Access`

**Fields**

| Field | Description |
| --- | --- |
| <code>focus_cursor: <a href="#focuscursor">FocusCursor</a></code> |  |
| <code>status: Duration</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Caps`

**Methods**

| Method | Description |
| --- | --- |
| <code>detect(env: impl Fn(&amp;str) -&gt; Option&lt;String&gt;) -&gt; Self</code> |  |
| <code>const host(&amp;self) -&gt; <a href="sigil-app.html#hostcaps">HostCaps</a></code> |  |
| <code>probe() -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>ambiguous: <a href="sigil-app.html#ambiguous">Ambiguous</a></code> |  |
| <code>backdrop: Option&lt;<a href="#backdrop">Backdrop</a>&gt;</code> |  |
| <code>borders: <a href="#borders">Borders</a></code> |  |
| <code>clipboard: <a href="#cliproute">ClipRoute</a></code> |  |
| <code>color: <a href="#colordepth">ColorDepth</a></code> |  |
| <code>cursor: <a href="#cursor">Cursor</a></code> |  |
| <code>erase: <a href="#erase">Erase</a></code> |  |
| <code>glyphs: <a href="#glyphs">Glyphs</a></code> |  |
| <code>graphics: Option&lt;<a href="#protocol">Protocol</a>&gt;</code> |  |
| <code>hyperlinks: bool</code> |  |
| <code>keyboard: <a href="#keyboard">Keyboard</a></code> |  |
| <code>mux: Option&lt;<a href="#mux">Mux</a>&gt;</code> |  |
| <code>sync: bool</code> |  |
| <code>underline_styles: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Cell`

**Methods**

| Method | Description |
| --- | --- |
| <code>const blank() -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>bg: <a href="#ink">Ink</a></code> |  |
| <code>bold: bool</code> |  |
| <code>dim: bool</code> |  |
| <code>fg: <a href="#ink">Ink</a></code> |  |
| <code>italic: bool</code> |  |
| <code>lines: <a href="#lines">Lines</a></code> |  |
| <code>link: Option&lt;SmolStr&gt;</code> |  |
| <code>text: SmolStr</code> |  |
| <code>underline_ink: <a href="#ink">Ink</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Grid`

**Methods**

| Method | Description |
| --- | --- |
| <code>const cols(&amp;self) -&gt; usize</code> |  |
| <code>const cursor(&amp;self) -&gt; Option&lt;(usize, usize)&gt;</code> |  |
| <code>get(&amp;self, col: usize, row: usize) -&gt; Option&lt;<a href="#cell">Cell</a>&gt;</code> |  |
| <code>images(&amp;self) -&gt; &amp;[<a href="#placement">Placement</a>]</code> |  |
| <code>mirror(&amp;mut self, other: &amp;Self)</code> |  |
| <code>new(cols: u16, rows: u16) -&gt; Self</code> |  |
| <code>painted_width(&amp;self, row: usize) -&gt; usize</code> |  |
| <code>reset(&amp;mut self)</code> |  |
| <code>resize(&amp;mut self, cols: u16, rows: u16)</code> |  |
| <code>row(&amp;self, row: usize) -&gt; &amp;[<a href="#cell">Cell</a>]</code> |  |
| <code>const rows(&amp;self) -&gt; usize</code> |  |
| <code>const set_graphics(&amp;mut self, graphics: bool)</code> |  |
| <code>text(&amp;self) -&gt; String</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Hooks`

**Fields**

| Field | Description |
| --- | --- |
| <code>after_frame: Option&lt;<a href="#afterframe">AfterFrame</a>&lt;'a&gt;&gt;</code> |  |
| <code>external_events: Option&lt;<a href="#eventpump">EventPump</a>&lt;'a&gt;&gt;</code> |  |
| <code>region_rows: Option&lt;Box&lt;(dyn FnMut() -&gt; u16 + 'a)&gt;&gt;</code> |  |

**Trait implementations:** <code>Default</code>


### `Lines`

**Fields**

| Field | Description |
| --- | --- |
| <code>strike: bool</code> |  |
| <code>underline: <a href="#underline">Underline</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Marks`

**Methods**

| Method | Description |
| --- | --- |
| <code>collect(cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, focus: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, cell: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Paint`

**Fields**

| Field | Description |
| --- | --- |
| <code>caps: <a href="#caps">Caps</a></code> |  |
| <code>cell: f32</code> |  |
| <code>marks: &amp;'a <a href="#marks">Marks</a></code> |  |
| <code>theme: &amp;'a <a href="sigil-style.html#theme">Theme</a></code> |  |


### `Placement`

**Methods**

| Method | Description |
| --- | --- |
| <code>same(&amp;self, other: &amp;Self) -&gt; bool</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>col: u16</code> |  |
| <code>cols: u16</code> |  |
| <code>crop: Option&lt;(u32, u32)&gt;</code> |  |
| <code>row: u16</code> |  |
| <code>rows: u16</code> |  |
| <code>source: <a href="sigil-kernel.html#imagesource">ImageSource</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>


### `Screen`

**Methods**

| Method | Description |
| --- | --- |
| <code>const caps(&amp;self) -&gt; &amp;<a href="#caps">Caps</a></code> |  |
| <code>const exited(&amp;self) -&gt; bool</code> |  |
| <code>flush(&amp;mut self, out: &amp;mut impl Write) -&gt; Result&lt;()&gt;</code> |  |
| <code>flush_inline(&amp;mut self, out: &amp;mut impl Write) -&gt; Result&lt;()&gt;</code> |  |
| <code>frame(&amp;mut self, host: &amp;mut <a href="sigil.html#host">Host</a>, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, events: impl IntoIterator&lt;Item = <a href="sigil-events.html#shellevent">ShellEvent</a>&gt;, size: (u16, u16)) -&gt; Result&lt;&amp;<a href="#grid">Grid</a>, <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</code> |  |
| <code>const invalidate(&amp;mut self)</code> |  |
| <code>const needs_flush(&amp;self) -&gt; bool</code> |  |
| <code>new(cols: u16, rows: u16) -&gt; Self</code> |  |
| <code>const rows(&amp;self) -&gt; usize</code> |  |
| <code>const set_access(&amp;mut self, access: <a href="#access">Access</a>)</code> |  |
| <code>const set_caps(&amp;mut self, caps: <a href="#caps">Caps</a>)</code> |  |
| <code>const set_dt(&amp;mut self, dt: f32)</code> |  |
| <code>text(&amp;self) -&gt; String</code> |  |
| <code>with_caps(cols: u16, rows: u16, caps: <a href="#caps">Caps</a>) -&gt; Self</code> |  |


## Enums

### `Backdrop`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Dark</code> |  |
| <code>Light</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Borders`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ascii</code> |  |
| <code>Light</code> |  |
| <code>Rounded</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ClipRoute`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Osc52</code> |  |
| <code>Pbcopy</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ColorDepth`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ansi16</code> |  |
| <code>Ansi256</code> |  |
| <code>Monochrome</code> |  |
| <code>Truecolor</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Cursor`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bar</code> |  |
| <code>Block</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Erase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Background</code> |  |
| <code>Default</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FocusCursor`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Follow</code> |  |
| <code>Off</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Glyphs`

**Methods**

| Method | Description |
| --- | --- |
| <code>glyph(self, glyph: char) -&gt; char</code> |  |
| <code>text(self, text: &amp;str) -&gt; Cow&lt;'_, str&gt;</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ascii</code> |  |
| <code>Console</code> |  |
| <code>Unicode</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Ink`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bare</code> |  |
| <code>Exact(<a href="sigil-style.html#rgba8">Rgba8</a>)</code> |  |
| <code>Of(<a href="sigil-style.html#tone">Tone</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Keyboard`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Kitty</code> |  |
| <code>Legacy</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `MouseTracking`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Clicks</code> |  |
| <code>Drags</code> |  |
| <code>Motion</code> |  |
| <code>Off</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Mux`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Screen</code> |  |
| <code>Tmux</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PrintAboveError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>FullScreen</code> |  |
| <code>NotLive</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Protocol`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Iterm2</code> |  |
| <code>Kitty</code> |  |
| <code>Sixel</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Underline`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Curly</code> |  |
| <code>None</code> |  |
| <code>Straight</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `base64`

<pre>pub fn <a href="#base64">base64</a>(bytes: &amp;[u8]) -&gt; String</pre>


### `bell`

<pre>pub fn <a href="#bell">bell</a>(note: Option&lt;&amp;str&gt;) -&gt; Result&lt;()&gt;</pre>


### `blocks`

<pre>pub fn <a href="#blocks">blocks</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>, cols: usize, rows: usize) -&gt; Vec&lt;Vec&lt;(<a href="sigil-style.html#rgba8">Rgba8</a>, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;&gt;</pre>


### `blocks_window`

<pre>pub fn <a href="#blocks_window">blocks_window</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>, window: (u32, u32), cols: usize, rows: usize) -&gt; Vec&lt;Vec&lt;(<a href="sigil-style.html#rgba8">Rgba8</a>, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;&gt;</pre>


### `calibrate`

<pre>pub fn <a href="#calibrate">calibrate</a>(out: &amp;mut impl Write) -&gt; Result&lt;()&gt;</pre>


### `cell_point`

<pre>pub fn <a href="#cell_point">cell_point</a>(col: u16, row: u16) -&gt; <a href="sigil-geometry.html#point">Point</a></pre>


### `click`

<pre>pub fn <a href="#click">click</a>(col: u16, row: u16) -&gt; [<a href="sigil-events.html#shellevent">ShellEvent</a>; 2]</pre>


### `detect`

<pre>pub fn <a href="#detect">detect</a>(env: impl Fn(&amp;str) -&gt; Option&lt;String&gt;) -&gt; Option&lt;<a href="#protocol">Protocol</a>&gt;</pre>


### `detect_mux`

<pre>pub fn <a href="#detect_mux">detect_mux</a>(env: impl Fn(&amp;str) -&gt; Option&lt;String&gt;) -&gt; Option&lt;<a href="#mux">Mux</a>&gt;</pre>


### `focused`

<pre>pub fn <a href="#focused">focused</a>() -&gt; bool</pre>


### `glyphs`

<pre>pub fn <a href="#glyphs-1">glyphs</a>() -&gt; <a href="#glyphs">Glyphs</a></pre>


### `install_clipboard`

<pre>pub fn <a href="#install_clipboard">install_clipboard</a>(caps: <a href="#caps">Caps</a>)</pre>


### `key_down`

<pre>pub fn <a href="#key_down">key_down</a>(key: impl Into&lt;<a href="sigil-events.html#logicalkey">LogicalKey</a>&gt;) -&gt; <a href="sigil-events.html#shellevent">ShellEvent</a></pre>


### `map_key`

<pre>pub fn <a href="#map_key">map_key</a>(event: &amp;KeyEvent) -&gt; Option&lt;<a href="sigil-events.html#shellevent">ShellEvent</a>&gt;</pre>


### `map_mouse`

<pre>pub fn <a href="#map_mouse">map_mouse</a>(event: &amp;MouseEvent, cell: f32, last: &amp;mut <a href="sigil-geometry.html#point">Point</a>) -&gt; Option&lt;<a href="sigil-events.html#shellevent">ShellEvent</a>&gt;</pre>


### `paste`

<pre>pub fn <a href="#paste">paste</a>(text: &amp;str) -&gt; <a href="sigil-events.html#shellevent">ShellEvent</a></pre>


### `png`

<pre>pub fn <a href="#png">png</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>) -&gt; Vec&lt;u8&gt;</pre>


### `png_window`

<pre>pub fn <a href="#png_window">png_window</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>, top: u32, height: u32) -&gt; Vec&lt;u8&gt;</pre>


### `pointer`

<pre>pub fn <a href="#pointer">pointer</a>(phase: <a href="sigil-events.html#pointerphase">PointerPhase</a>, col: u16, row: u16) -&gt; <a href="sigil-events.html#shellevent">ShellEvent</a></pre>


### `present`

<pre>pub fn <a href="#present">present</a>&lt;N: <a href="sigil-kernel.html#intonode">IntoNode</a>&gt;(tree: impl Fn() -&gt; N + 'static, cols: u16, rows: u16, events: &amp;[<a href="sigil-events.html#shellevent">ShellEvent</a>]) -&gt; Result&lt;String, <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</pre>


### `print_above`

<pre>pub fn <a href="#print_above">print_above</a>(text: &amp;str) -&gt; Result&lt;()&gt;</pre>


### `raster`

<pre>pub fn <a href="#raster">raster</a>(scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, theme: &amp;<a href="sigil-style.html#theme">Theme</a>, cols: u16, rows: u16, cell: f32) -&gt; <a href="#grid">Grid</a></pre>


### `raster_into`

<pre>pub fn <a href="#raster_into">raster_into</a>(grid: &amp;mut <a href="#grid">Grid</a>, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, theme: &amp;<a href="sigil-style.html#theme">Theme</a>, cell: f32)</pre>


### `raster_with`

<pre>pub fn <a href="#raster_with">raster_with</a>(grid: &amp;mut <a href="#grid">Grid</a>, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, paint: &amp;<a href="#paint">Paint</a>&lt;'_&gt;)</pre>


### `render`

<pre>pub fn <a href="#render">render</a>(out: &amp;mut impl Write, prev: &amp;<a href="#grid">Grid</a>, next: &amp;<a href="#grid">Grid</a>, depth: <a href="#colordepth">ColorDepth</a>) -&gt; Result&lt;()&gt;</pre>


### `set_mouse_capture`

<pre>pub fn <a href="#set_mouse_capture">set_mouse_capture</a>(yes: bool) -&gt; Result&lt;()&gt;</pre>


### `set_mouse_tracking`

<pre>pub fn <a href="#set_mouse_tracking">set_mouse_tracking</a>(tracking: <a href="#mousetracking">MouseTracking</a>) -&gt; Result&lt;()&gt;</pre>


### `set_title`

<pre>pub fn <a href="#set_title">set_title</a>(title: &amp;str) -&gt; Result&lt;()&gt;</pre>


### `sextants`

<pre>pub fn <a href="#sextants">sextants</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>, cols: usize, rows: usize) -&gt; Vec&lt;Vec&lt;(char, <a href="sigil-style.html#rgba8">Rgba8</a>, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;&gt;</pre>


### `sextants_window`

<pre>pub fn <a href="#sextants_window">sextants_window</a>(source: &amp;<a href="sigil-kernel.html#imagesource">ImageSource</a>, window: (u32, u32), cols: usize, rows: usize) -&gt; Vec&lt;Vec&lt;(char, <a href="sigil-style.html#rgba8">Rgba8</a>, <a href="sigil-style.html#rgba8">Rgba8</a>)&gt;&gt;</pre>


### `step`

<pre>pub fn <a href="#step">step</a>(host: &amp;mut <a href="sigil.html#host">Host</a>, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, events: impl IntoIterator&lt;Item = <a href="sigil-events.html#shellevent">ShellEvent</a>&gt;, cols: u16, rows: u16) -&gt; Result&lt;<a href="#grid">Grid</a>, <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</pre>


### `suspend`

<pre>pub fn <a href="#suspend">suspend</a>&lt;R&gt;(run: impl FnOnce() -&gt; R) -&gt; Result&lt;R&gt;</pre>


### `wheel`

<pre>pub fn <a href="#wheel">wheel</a>(col: u16, row: u16, lines: f32) -&gt; <a href="sigil-events.html#shellevent">ShellEvent</a></pre>


## Type aliases

### `AfterFrame`

<pre>pub type <a href="#afterframe">AfterFrame</a>&lt;'a&gt; = Box&lt;(dyn FnMut(&amp;mut <a href="sigil.html#host">Host</a>, &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, &amp;<a href="#screen">Screen</a>) + 'a)&gt;</pre>


### `EventPump`

<pre>pub type <a href="#eventpump">EventPump</a>&lt;'a&gt; = Box&lt;(dyn FnMut(&amp;mut Vec&lt;<a href="sigil-events.html#shellevent">ShellEvent</a>&gt;) + 'a)&gt;</pre>


## Constants

### `CONTINUES`

<pre>pub const <a href="#continues">CONTINUES</a>: &amp;str</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
