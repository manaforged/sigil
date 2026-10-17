# Module `sigil`

| Item | Kind | Description |
| --- | --- | --- |
| [`Error`](#error) | struct |  |
| [`AppConfig`](#appconfig) | struct |  |
| [`Host`](#host) | struct |  |
| [`request_exit`](#request_exit) | fn |  |
| [`run`](#run) | fn |  |
| [`AppResult`](#appresult) | type |  |
| [`wgpu`](#wgpu) | use |  |
| [`T`](#t) | impl |  |
| [`alloc::boxed::Box`](#allocboxedbox) | impl |  |
| [`F`](#f) | impl |  |
| [`f32`](#f32) | impl |  |
| [`alloc::sync::Arc`](#allocsyncarc) | impl |  |
| [`f64`](#f64) | impl |  |
| [`i32`](#i32) | impl |  |
| [`i64`](#i64) | impl |  |
| [`usize`](#usize) | impl |  |
| [`str`](#str) | impl |  |
| [`alloc::string::String`](#allocstringstring) | impl |  |
| [`bool`](#bool) | impl |  |
| [`alloc::vec::Vec`](#allocvecvec) | impl |  |
| [`tokio::sync::mpsc::bounded::Receiver`](#tokiosyncmpscboundedreceiver) | impl |  |
| [`tokio::sync::mpsc::unbounded::UnboundedReceiver`](#tokiosyncmpscunboundedunboundedreceiver) | impl |  |
| [`core::io::error::Error`](#coreioerrorerror) | impl |  |
| [`R`](#r) | impl |  |
| [`K`](#k) | impl |  |

## Structs

### `Error`

**Methods**

| Method | Description |
| --- | --- |
| <code>const kind(&amp;self) -&gt; <a href="sigil-app.html#errorkind">ErrorKind</a></code> |  |
| <code>new&lt;E: Error + Send + Sync + 'static&gt;(err: E) -&gt; Self</code> |  |
| <code>source_error(&amp;self) -&gt; &amp;(dyn Error + Send + Sync + 'static)</code> |  |
| <code>with_kind&lt;E: Error + Send + Sync + 'static&gt;(kind: <a href="sigil-app.html#errorkind">ErrorKind</a>, err: E) -&gt; Self</code> |  |

**Trait implementations:** <code>From&lt;<a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</code>, <code>From&lt;<a href="sigil-renderer.html#renderererror">RendererError</a>&gt;</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>


### `AppConfig`

**Methods**

| Method | Description |
| --- | --- |
| <code>const anchor(self, anchor: <a href="sigil-app.html#anchor">Anchor</a>) -&gt; Self</code> |  |
| <code>canvas(self, id: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>const click(self, click: <a href="sigil-events.html#clickconfig">ClickConfig</a>) -&gt; Self</code> |  |
| <code>const esc_exits(self, yes: bool) -&gt; Self</code> |  |
| <code>const exits_on_esc(&amp;self) -&gt; bool</code> |  |
| <code>const frame_interval(self, interval: Duration) -&gt; Self</code> |  |
| <code>const frameless(self, yes: bool) -&gt; Self</code> |  |
| <code>const inline(self, yes: bool) -&gt; Self</code> |  |
| <code>const min_size(self, width: u32, height: u32) -&gt; Self</code> |  |
| <code>const modify_other_keys(self, yes: bool) -&gt; Self</code> |  |
| <code>const mouse_tracking(self, tracking: <a href="sigil-terminal.html#mousetracking">MouseTracking</a>) -&gt; Self</code> |  |
| <code>new(title: impl Into&lt;String&gt;) -&gt; Self</code> |  |
| <code>on_close(self, decide: impl Fn() -&gt; <a href="sigil-app.html#closeaction">CloseAction</a> + 'static) -&gt; Self</code> |  |
| <code>on_exit(self, run: impl Fn() + 'static) -&gt; Self</code> |  |
| <code>const on_panic(&amp;self) -&gt; <a href="sigil-app.html#panicpolicy">PanicPolicy</a></code> |  |
| <code>const overlay(self, yes: bool) -&gt; Self</code> |  |
| <code>const panic_policy(self, policy: <a href="sigil-app.html#panicpolicy">PanicPolicy</a>) -&gt; Self</code> |  |
| <code>const quit_on_ctrl_c(self, yes: bool) -&gt; Self</code> |  |
| <code>const reduced_motion(self, reduced: bool) -&gt; Self</code> |  |
| <code>const resizable(self, yes: bool) -&gt; Self</code> |  |
| <code>const size(self, width: u32, height: u32) -&gt; Self</code> |  |
| <code>suspend_on(self, shortcut: impl Into&lt;Option&lt;<a href="sigil-events.html#shortcut">Shortcut</a>&gt;&gt;) -&gt; Self</code> |  |
| <code>const task_workers(self, workers: <a href="sigil-tasks.html#taskworkers">TaskWorkers</a>) -&gt; Self</code> |  |
| <code>const terminal(self) -&gt; Self</code> |  |
| <code>const terminal_access(self, access: <a href="sigil-terminal.html#access">Access</a>) -&gt; Self</code> |  |
| <code>terminal_hooks(self, hooks: <a href="sigil-terminal.html#hooks">Hooks</a>&lt;'static&gt;) -&gt; Self</code> |  |
| <code>theme_file(self, path: impl Into&lt;PathBuf&gt;) -&gt; Self</code> |  |
| <code>title(&amp;self) -&gt; &amp;str</code> |  |
| <code>const user_zoom(self, zoom: f32) -&gt; Self</code> |  |
| <code>const window(self) -&gt; Self</code> |  |
| <code>const window_size(&amp;self) -&gt; (u32, u32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>From&lt;&amp;str&gt;</code>, <code>From&lt;String&gt;</code>, <code>Debug</code>


### `Host`

**Methods**

| Method | Description |
| --- | --- |
| <code>const CELL: f32</code> |  |
| <code>accessibility_tree(&amp;self, router: &amp;<a href="sigil-events.html#eventrouter">EventRouter</a>) -&gt; <a href="sigil-app.html#accessibilitytree">AccessibilityTree</a></code> |  |
| <code>apply_focus_requests(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>)</code> |  |
| <code>cells() -&gt; Self</code> |  |
| <code>const cx(&amp;mut self) -&gt; &amp;mut <a href="sigil-kernel.html#cx">Cx</a></code> |  |
| <code>emit(&amp;mut self, viewport: (f32, f32)) -&gt; <a href="sigil-scene.html#commandscene">CommandScene</a></code> |  |
| <code>emit_since(&amp;mut self, viewport: (f32, f32), delta: &amp;<a href="sigil-kernel.html#framedelta">FrameDelta</a>, prev: &amp;<a href="sigil-scene.html#commandscene">CommandScene</a>) -&gt; (<a href="sigil-scene.html#commandscene">CommandScene</a>, <a href="sigil-scene.html#emitdiff">EmitDiff</a>)</code> |  |
| <code>focus_intent(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, intent: <a href="sigil-events.html#focusintent">FocusIntent</a>)</code> |  |
| <code>hit_path(&amp;mut self, x: f32, y: f32) -&gt; Vec&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>const host_mode(&amp;self) -&gt; <a href="sigil-kernel.html#hostmode">HostMode</a></code> |  |
| <code>layout(&amp;mut self, viewport: (f32, f32)) -&gt; Result&lt;Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;, <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</code> |  |
| <code>mount(&amp;mut self, app: impl <a href="sigil-kernel.html#intonode">IntoNode</a>)</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>perform(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, target: <a href="sigil-kernel.html#nodeid">NodeId</a>, request: &amp;ActionRequest)</code> |  |
| <code>repair_focus(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>)</code> |  |
| <code>reset_emit(&amp;mut self)</code> |  |
| <code>const root(&amp;self) -&gt; Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>route(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, event: <a href="sigil-events.html#shellevent">ShellEvent</a>)</code> |  |
| <code>scene(&amp;mut self, viewport: (f32, f32)) -&gt; Result&lt;<a href="sigil-scene.html#commandscene">CommandScene</a>, <a href="sigil-layout.html#layouterror">LayoutError</a>&gt;</code> |  |
| <code>const set_caps(&amp;mut self, caps: <a href="sigil-app.html#hostcaps">HostCaps</a>)</code> |  |
| <code>surfaces(&amp;self) -&gt; Vec&lt;(u64, <a href="sigil-geometry.html#rect">Rect</a>)&gt;</code> |  |
| <code>sync_hover(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>)</code> |  |
| <code>take_exit(&amp;self) -&gt; bool</code> |  |
| <code>theme(&amp;self) -&gt; &amp;<a href="sigil-style.html#theme">Theme</a></code> |  |
| <code>frame(&amp;mut self, router: &amp;mut <a href="sigil-events.html#eventrouter">EventRouter</a>, viewport: (f32, f32), dt: f32, hooks: <a href="sigil-app.html#framehooks">FrameHooks</a>&lt;'_&gt;) -&gt; <a href="sigil-app.html#frameoutput">FrameOutput</a></code> |  |

**Trait implementations:** <code>Default</code>


## Functions

### `request_exit`

<pre>pub fn <a href="#request_exit">request_exit</a>()</pre>


### `run`

<pre>pub fn <a href="#run">run</a>&lt;C, F, N&gt;(config: C, app: F) -&gt; <a href="#appresult">AppResult</a> where C: Into&lt;<a href="#appconfig">AppConfig</a>&gt;, F: Fn() -&gt; N + 'static, N: <a href="sigil-kernel.html#intonode">IntoNode</a></pre>


## Type aliases

### `AppResult`

<pre>pub type <a href="#appresult">AppResult</a>&lt;T&gt; = Result&lt;T, <a href="#error">Error</a>&gt;</pre>


## Re-exports

### `wgpu`


## Implementations

### `T`

**Trait implementations:** <code><a href="sigil-events.html#nodeeventsext">NodeEventsExt</a></code>, <code><a href="sigil-style.html#paint">Paint</a></code>, <code><a href="sigil-style.html#statevariants">StateVariants</a></code>, <code><a href="sigil-widgets.html#intoiconsource">IntoIconSource</a></code>


### `alloc::boxed::Box`

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>


### `F`

**Trait implementations:** <code><a href="sigil-kernel.html#intonode">IntoNode</a></code>, <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;T&gt;</code>, <code><a href="sigil-widgets.html#rowrender">RowRender</a>&lt;T&gt;</code>


### `f32`

**Trait implementations:** <code><a href="sigil-layout.html#intopx">IntoPx</a></code>, <code><a href="sigil-widgets.html#numerickind">NumericKind</a></code>


### `alloc::sync::Arc`

**Trait implementations:** <code><a href="sigil-widgets.html#identifiable">Identifiable</a></code>


### `f64`

**Trait implementations:** <code><a href="sigil-widgets.html#numerickind">NumericKind</a></code>


### `i32`

**Trait implementations:** <code><a href="sigil-widgets.html#numerickind">NumericKind</a></code>


### `i64`

**Trait implementations:** <code><a href="sigil-widgets.html#numerickind">NumericKind</a></code>


### `usize`

**Trait implementations:** <code><a href="sigil-widgets.html#numerickind">NumericKind</a></code>


### `str`

**Trait implementations:** <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;String&gt;</code>


### `alloc::string::String`

**Trait implementations:** <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;String&gt;</code>


### `bool`

**Trait implementations:** <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;</code>


### `alloc::vec::Vec`

**Trait implementations:** <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;Vec&lt;T&gt;&gt;</code>, <code><a href="sigil-widgets.html#intolist">IntoList</a>&lt;T, R&gt;</code>


### `tokio::sync::mpsc::bounded::Receiver`

**Trait implementations:** <code><a href="sigil-tasks.html#messagereceiver">MessageReceiver</a>&lt;T&gt;</code>


### `tokio::sync::mpsc::unbounded::UnboundedReceiver`

**Trait implementations:** <code><a href="sigil-tasks.html#messagereceiver">MessageReceiver</a>&lt;T&gt;</code>


### `core::io::error::Error`

**Trait implementations:** <code>From&lt;<a href="sigil-terminal.html#printaboveerror">PrintAboveError</a>&gt;</code>


### `R`

**Trait implementations:** <code><a href="sigil-widgets.html#diffsource">DiffSource</a></code>


### `K`

**Trait implementations:** <code><a href="sigil-widgets.html#treekey">TreeKey</a></code>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
