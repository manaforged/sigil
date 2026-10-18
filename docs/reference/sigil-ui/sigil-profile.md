# Module `sigil::profile`

| Item | Kind | Description |
| --- | --- | --- |
| [`Attribution`](#attribution) | struct |  |
| [`BackendStatus`](#backendstatus) | struct |  |
| [`Frame`](#frame) | struct |  |
| [`Span`](#span) | struct |  |
| [`BackendKind`](#backendkind) | enum |  |
| [`Category`](#category) | enum |  |
| [`Cause`](#cause) | enum |  |
| [`Counter`](#counter) | enum |  |
| [`FallbackCause`](#fallbackcause) | enum |  |
| [`Hotspot`](#hotspot) | enum |  |
| [`Phase`](#phase) | enum |  |
| [`attribute`](#attribute) | fn |  |
| [`begin_frame`](#begin_frame) | fn |  |
| [`count`](#count) | fn |  |
| [`count_add`](#count_add) | fn |  |
| [`disable`](#disable) | fn |  |
| [`dump`](#dump) | fn |  |
| [`dump_if_enabled`](#dump_if_enabled) | fn |  |
| [`enable`](#enable) | fn |  |
| [`enabled`](#enabled) | fn |  |
| [`frame_budget_ns`](#frame_budget_ns) | fn |  |
| [`frames`](#frames) | fn |  |
| [`gauge`](#gauge) | fn |  |
| [`gpu_pass`](#gpu_pass) | fn |  |
| [`interact`](#interact) | fn |  |
| [`note_input`](#note_input) | fn |  |
| [`reset`](#reset) | fn |  |
| [`span`](#span-1) | fn |  |
| [`summary`](#summary) | fn |  |
| [`tick_frame`](#tick_frame) | fn |  |
| [`CounterValue`](#countervalue) | type |  |
| [`PhaseTime`](#phasetime) | type |  |
| [`BUDGET_ENV`](#budget_env) | const |  |
| [`DEFAULT_BUDGET_MS`](#default_budget_ms) | const |  |
| [`FRAME_BUDGET_NS`](#frame_budget_ns-1) | const |  |
| [`INTERACT_BUDGET_NS`](#interact_budget_ns) | const |  |
| [`MAX_SUBJECTS`](#max_subjects) | const |  |
| [`OTHER_SUBJECT`](#other_subject) | const |  |

## Structs

### `Attribution`

**Fields**

| Field | Description |
| --- | --- |
| <code>count: u64</code> |  |
| <code>hotspot: <a href="#hotspot">Hotspot</a></code> |  |
| <code>subject: String</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `BackendStatus`

**Methods**

| Method | Description |
| --- | --- |
| <code>const TERMINAL: Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>fallback: <a href="#fallbackcause">FallbackCause</a></code> |  |
| <code>kind: <a href="#backendkind">BackendKind</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Frame`

**Methods**

| Method | Description |
| --- | --- |
| <code>counter(&amp;self, counter: <a href="#counter">Counter</a>) -&gt; u64</code> |  |
| <code>hottest(&amp;self) -&gt; Option&lt;<a href="#phasetime">PhaseTime</a>&gt;</code> |  |
| <code>total_ns(&amp;self) -&gt; u64</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>backend: Option&lt;<a href="#backendstatus">BackendStatus</a>&gt;</code> |  |
| <code>cause: <a href="#cause">Cause</a></code> |  |
| <code>counts: Vec&lt;<a href="#countervalue">CounterValue</a>&gt;</code> |  |
| <code>end_us: u64</code> |  |
| <code>gpu_passes: Vec&lt;(&amp;'static str, u64)&gt;</code> |  |
| <code>hot: Vec&lt;<a href="#attribution">Attribution</a>&gt;</code> |  |
| <code>index: u32</code> |  |
| <code>phases: Vec&lt;<a href="#phasetime">PhaseTime</a>&gt;</code> |  |
| <code>wall_ns: u64</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>


### `Span`

**Trait implementations:** <code>Drop</code>


## Enums

### `BackendKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cpu</code> |  |
| <code>Gpu</code> |  |
| <code>GpuSoftware</code> |  |
| <code>Terminal</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Category`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>A11y</code> |  |
| <code>App</code> |  |
| <code>Layout</code> |  |
| <code>Paint</code> |  |
| <code>Reactive</code> |  |
| <code>Render</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Cause`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |
| <code>const rank(self) -&gt; u8</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>A11y</code> |  |
| <code>Expose</code> |  |
| <code>Hover</code> |  |
| <code>Idle</code> |  |
| <code>Ime</code> |  |
| <code>Input</code> |  |
| <code>Key</code> |  |
| <code>Pointer</code> |  |
| <code>Resize</code> |  |
| <code>Scroll</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Counter`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>AllocCompositeTextureBindGroups</code> |  |
| <code>AllocCompositeUniformBindGroups</code> |  |
| <code>AllocLayerBytes</code> |  |
| <code>AllocLayerTextures</code> |  |
| <code>AllocRasterBindGroups</code> |  |
| <code>AllocSpriteBindGroups</code> |  |
| <code>AllocUploadBuffers</code> |  |
| <code>AllocUploadBytes</code> |  |
| <code>BuildProfile</code> |  |
| <code>EmitCopied</code> |  |
| <code>EmitNodes</code> |  |
| <code>EmitReplays</code> |  |
| <code>GpuBatches</code> |  |
| <code>GpuCompositePipelines</code> |  |
| <code>GpuGlyphs</code> |  |
| <code>GpuLayerCacheBudgetBytes</code> |  |
| <code>GpuLayerCacheBytes</code> |  |
| <code>GpuLayerCacheTextures</code> |  |
| <code>GpuLayerPixels</code> |  |
| <code>GpuLayerReuses</code> |  |
| <code>GpuLayers</code> |  |
| <code>GpuPassReuses</code> |  |
| <code>GpuPlanFull</code> |  |
| <code>GpuPlanRegions</code> |  |
| <code>GpuPlanUnchanged</code> |  |
| <code>GpuRects</code> |  |
| <code>GpuRedrawnPixels</code> |  |
| <code>GpuRenderPasses</code> |  |
| <code>GpuSharedUploadBytes</code> |  |
| <code>GpuSharedUploadWrites</code> |  |
| <code>GpuSplitSubmissions</code> |  |
| <code>GpuSprites</code> |  |
| <code>GpuTimedPasses</code> |  |
| <code>GpuUploadBuffers</code> |  |
| <code>GpuUploadBytes</code> |  |
| <code>GpuUploadCapacityBytes</code> |  |
| <code>GpuUploadWrites</code> |  |
| <code>KernelMounted</code> |  |
| <code>KernelPatched</code> |  |
| <code>KernelRemoved</code> |  |
| <code>LayoutDirtyKernel</code> |  |
| <code>LayoutDirtyNodes</code> |  |
| <code>LayoutDirtySeed</code> |  |
| <code>LayoutFull</code> |  |
| <code>LayoutFullMajority</code> |  |
| <code>LayoutFullNoDirty</code> |  |
| <code>LayoutFullUnmirrored</code> |  |
| <code>LayoutHit</code> |  |
| <code>LayoutIncr</code> |  |
| <code>LayoutIntrinsicMiss</code> |  |
| <code>LayoutMiss</code> |  |
| <code>LayoutOverlay</code> |  |
| <code>LayoutResize</code> |  |
| <code>LayoutResizedNodes</code> |  |
| <code>LayoutRoots</code> |  |
| <code>LayoutSolvedNodes</code> |  |
| <code>LayoutTranslate</code> |  |
| <code>PathRaster</code> |  |
| <code>RenderFrames</code> |  |
| <code>SceneCmds</code> |  |
| <code>SceneEmit</code> |  |
| <code>SceneGlyphs</code> |  |
| <code>SceneImages</code> |  |
| <code>SceneRects</code> |  |
| <code>SceneReplay</code> |  |
| <code>TextAtlasCompactions</code> |  |
| <code>TextAtlasMisses</code> |  |
| <code>TextMeasure</code> |  |
| <code>TextOutlineMisses</code> |  |
| <code>TuiBytes</code> |  |
| <code>TuiCellsRastered</code> |  |
| <code>TuiDirty</code> |  |
| <code>TuiEvents</code> |  |
| <code>TuiFrames</code> |  |
| <code>TuiNodes</code> |  |
| <code>TuiRowsWritten</code> |  |
| <code>TuiWrites</code> |  |
| <code>WgpuBindGroups</code> |  |
| <code>WgpuBuffers</code> |  |
| <code>WgpuCommandBuffers</code> |  |
| <code>WgpuRenderPipelines</code> |  |
| <code>WgpuTextureViews</code> |  |
| <code>WgpuTextures</code> |  |
| <code>WorkloadCols</code> |  |
| <code>WorkloadFps</code> |  |
| <code>WorkloadHeadless</code> |  |
| <code>WorkloadItems</code> |  |
| <code>WorkloadRows</code> |  |
| <code>WorkloadScenario</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FallbackCause`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>DeviceError</code> |  |
| <code>ForcedByEnv</code> |  |
| <code>NoAdapter</code> |  |
| <code>None</code> |  |
| <code>UnknownEnv</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Hotspot`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>EmitResolved</code> |  |
| <code>GpuFullRepaint</code> |  |
| <code>GpuLayer</code> |  |
| <code>LayoutDirtyKernel</code> |  |
| <code>LayoutDirtySeed</code> |  |
| <code>LayoutResized</code> |  |
| <code>LayoutRoot</code> |  |
| <code>RowRebuild</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Phase`

**Methods**

| Method | Description |
| --- | --- |
| <code>const category(self) -&gt; <a href="#category">Category</a></code> |  |
| <code>const is_top_level(self) -&gt; bool</code> |  |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>A11y</code> |  |
| <code>AppUpdate</code> |  |
| <code>Effect</code> |  |
| <code>Emit</code> |  |
| <code>Gpu</code> |  |
| <code>GpuWait</code> |  |
| <code>HitRefresh</code> |  |
| <code>Layout</code> |  |
| <code>LayoutAttach</code> |  |
| <code>LayoutChildren</code> |  |
| <code>LayoutDirty</code> |  |
| <code>LayoutFinish</code> |  |
| <code>LayoutOverlays</code> |  |
| <code>LayoutPrepare</code> |  |
| <code>LayoutSeed</code> |  |
| <code>LayoutSig</code> |  |
| <code>LayoutSolve</code> |  |
| <code>LayoutSync</code> |  |
| <code>LayoutTranslate</code> |  |
| <code>Paint</code> |  |
| <code>RenderCompositePipeline</code> |  |
| <code>RenderCpu</code> |  |
| <code>RenderEncode</code> |  |
| <code>RenderPlan</code> |  |
| <code>RenderRetain</code> |  |
| <code>RenderSubmit</code> |  |
| <code>TuiPaint</code> |  |
| <code>TuiPresent</code> |  |
| <code>TuiRaster</code> |  |
| <code>TuiWrite</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Display</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `attribute`

<pre>pub fn <a href="#attribute">attribute</a>(hotspot: <a href="#hotspot">Hotspot</a>, n: u64, subject: impl FnOnce() -&gt; String)</pre>


### `begin_frame`

<pre>pub fn <a href="#begin_frame">begin_frame</a>(backend: <a href="#backendstatus">BackendStatus</a>)</pre>


### `count`

<pre>pub fn <a href="#count">count</a>(counter: <a href="#counter">Counter</a>)</pre>


### `count_add`

<pre>pub fn <a href="#count_add">count_add</a>(counter: <a href="#counter">Counter</a>, n: u64)</pre>


### `disable`

<pre>pub fn <a href="#disable">disable</a>()</pre>


### `dump`

<pre>pub fn <a href="#dump">dump</a>(path: impl AsRef&lt;Path&gt;) -&gt; Result&lt;()&gt;</pre>


### `dump_if_enabled`

<pre>pub fn <a href="#dump_if_enabled">dump_if_enabled</a>()</pre>


### `enable`

<pre>pub fn <a href="#enable">enable</a>()</pre>


### `enabled`

<pre>pub fn <a href="#enabled">enabled</a>() -&gt; bool</pre>


### `frame_budget_ns`

<pre>pub fn <a href="#frame_budget_ns">frame_budget_ns</a>() -&gt; u64</pre>


### `frames`

<pre>pub fn <a href="#frames">frames</a>() -&gt; Vec&lt;<a href="#frame">Frame</a>&gt;</pre>


### `gauge`

<pre>pub fn <a href="#gauge">gauge</a>(counter: <a href="#counter">Counter</a>, value: u64)</pre>


### `gpu_pass`

<pre>pub fn <a href="#gpu_pass">gpu_pass</a>(label: &amp;'static str, ns: u64)</pre>


### `interact`

<pre>pub fn <a href="#interact">interact</a>(cause: <a href="#cause">Cause</a>)</pre>


### `note_input`

<pre>pub fn <a href="#note_input">note_input</a>(event: &amp;<a href="sigil-events.html#shellevent">ShellEvent</a>)</pre>


### `reset`

<pre>pub fn <a href="#reset">reset</a>()</pre>


### `span`

<pre>pub fn <a href="#span-1">span</a>(phase: <a href="#phase">Phase</a>) -&gt; <a href="#span">Span</a></pre>


### `summary`

<pre>pub fn <a href="#summary">summary</a>() -&gt; String</pre>


### `tick_frame`

<pre>pub fn <a href="#tick_frame">tick_frame</a>()</pre>


## Type aliases

### `CounterValue`

<pre>pub type <a href="#countervalue">CounterValue</a> = (<a href="#counter">Counter</a>, u64)</pre>


### `PhaseTime`

<pre>pub type <a href="#phasetime">PhaseTime</a> = (<a href="#phase">Phase</a>, u64)</pre>


## Constants

### `BUDGET_ENV`

<pre>pub const <a href="#budget_env">BUDGET_ENV</a>: &amp;str</pre>


### `DEFAULT_BUDGET_MS`

<pre>pub const <a href="#default_budget_ms">DEFAULT_BUDGET_MS</a>: f64</pre>


### `FRAME_BUDGET_NS`

<pre>pub const <a href="#frame_budget_ns-1">FRAME_BUDGET_NS</a>: u64</pre>


### `INTERACT_BUDGET_NS`

<pre>pub const <a href="#interact_budget_ns">INTERACT_BUDGET_NS</a>: u64</pre>


### `MAX_SUBJECTS`

<pre>pub const <a href="#max_subjects">MAX_SUBJECTS</a>: usize</pre>


### `OTHER_SUBJECT`

<pre>pub const <a href="#other_subject">OTHER_SUBJECT</a>: &amp;str</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
