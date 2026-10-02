# Module `sigil::renderer`

| Item | Kind | Description |
| --- | --- | --- |
| [`CpuRenderer`](#cpurenderer) | struct |  |
| [`WgpuRenderer`](#wgpurenderer) | struct |  |
| [`CpuSurface`](#cpusurface) | struct |  |
| [`OffscreenTarget`](#offscreentarget) | struct |  |
| [`ScreenSubmission`](#screensubmission) | struct |  |
| [`WgpuSurface`](#wgpusurface) | struct |  |
| [`WindowHandle`](#windowhandle) | struct |  |
| [`LayerError`](#layererror) | enum |  |
| [`RendererError`](#renderererror) | enum |  |
| [`into_window_handle`](#into_window_handle) | fn |  |

## Structs

### `CpuRenderer`

**Methods**

| Method | Description |
| --- | --- |
| <code>const is_transparent(&amp;self) -&gt; bool</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>render(&amp;mut self, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, clear: <a href="sigil-style.html#color">Color</a>, viewport: (f32, f32), size: <a href="sigil-geometry.html#physicalsize">PhysicalSize</a>) -&gt; Result&lt;Pixmap, <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>render_command_scene(&amp;mut self, surface: &amp;mut <a href="#cpusurface">CpuSurface</a>, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, clear: <a href="sigil-style.html#color">Color</a>, viewport: (f32, f32)) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>const set_transparent(&amp;mut self, transparent: bool)</code> |  |

**Trait implementations:** <code><a href="sigil-hal.html#renderer">Renderer</a></code>, <code>Default</code>


### `WgpuRenderer`

**Methods**

| Method | Description |
| --- | --- |
| <code>device(&amp;self) -&gt; Option&lt;&amp;Device&gt;</code> |  |
| <code>instance(&amp;self) -&gt; &amp;Instance</code> |  |
| <code>const is_transparent(&amp;self) -&gt; bool</code> |  |
| <code>new() -&gt; Result&lt;Self, <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>prepare_compositing(&amp;mut self, format: TextureFormat) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>queue(&amp;self) -&gt; Option&lt;&amp;Queue&gt;</code> |  |
| <code>render_command_scene(&amp;mut self, surface: &amp;mut <a href="#wgpusurface">WgpuSurface</a>, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, clear_color: <a href="sigil-style.html#color">Color</a>, viewport: (f32, f32)) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>const set_transparent(&amp;mut self, transparent: bool)</code> |  |
| <code>drain_gpu_timings(&amp;mut self)</code> |  |
| <code>record_screen(&amp;mut self, target: &amp;<a href="#screensubmission">ScreenSubmission</a>&lt;'_&gt;, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, encoder: &amp;mut CommandEncoder) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>submit_screen(&amp;mut self, target: &amp;<a href="#screensubmission">ScreenSubmission</a>&lt;'_&gt;, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |
| <code>render_command_scene_offscreen(&amp;mut self, target: <a href="#offscreentarget">OffscreenTarget</a>&lt;'_&gt;, scene: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>, clear_color: <a href="sigil-style.html#color">Color</a>) -&gt; Result&lt;(), <a href="#renderererror">RendererError</a>&gt;</code> |  |

**Trait implementations:** <code><a href="sigil-hal.html#renderer">Renderer</a></code>


### `CpuSurface`

**Methods**

| Method | Description |
| --- | --- |
| <code>const size(&amp;self) -&gt; <a href="sigil-geometry.html#physicalsize">PhysicalSize</a></code> |  |


### `OffscreenTarget`

**Fields**

| Field | Description |
| --- | --- |
| <code>device: &amp;'a Device</code> |  |
| <code>format: TextureFormat</code> |  |
| <code>queue: &amp;'a Queue</code> |  |
| <code>scale: f32</code> |  |
| <code>view: &amp;'a TextureView</code> |  |
| <code>viewport: (f32, f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>From&lt;&amp;<a href="#screensubmission">ScreenSubmission</a>&lt;'a&gt;&gt;</code>, <code>Copy</code>


### `ScreenSubmission`

**Fields**

| Field | Description |
| --- | --- |
| <code>clear: Option&lt;<a href="sigil-style.html#color">Color</a>&gt;</code> |  |
| <code>device: &amp;'a Device</code> |  |
| <code>format: TextureFormat</code> |  |
| <code>queue: &amp;'a Queue</code> |  |
| <code>scale: f32</code> |  |
| <code>view: &amp;'a TextureView</code> |  |
| <code>viewport: (f32, f32)</code> |  |

**Trait implementations:** <code>Debug</code>


### `WgpuSurface`

**Methods**

| Method | Description |
| --- | --- |
| <code>const format(&amp;self) -&gt; TextureFormat</code> |  |
| <code>const size(&amp;self) -&gt; <a href="sigil-geometry.html#physicalsize">PhysicalSize</a></code> |  |


### `WindowHandle`


## Enums

### `LayerError`

**Fields**

| Field | Description |
| --- | --- |
| <code>TooLarge::max: u32</code> |  |
| <code>TooLarge::requested: u32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Empty</code> |  |
| <code>TooLarge</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `RendererError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>DeviceRequest(RequestDeviceError)</code> |  |
| <code>InvalidViewport</code> |  |
| <code>Layer(<a href="#layererror">LayerError</a>)</code> |  |
| <code>NoAdapter</code> |  |
| <code>NoSupportedFormat</code> |  |
| <code>NotInitialized</code> |  |
| <code>Software(String)</code> |  |
| <code>SurfaceCreate(CreateSurfaceError)</code> |  |
| <code>SurfaceValidation</code> |  |

**Trait implementations:** <code>From&lt;RequestDeviceError&gt;</code>, <code>From&lt;CreateSurfaceError&gt;</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>


## Functions

### `into_window_handle`

<pre>pub fn <a href="#into_window_handle">into_window_handle</a>&lt;W&gt;(window: W) -&gt; <a href="#windowhandle">WindowHandle</a> where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
