# Module `sigil::reactive`

| Item | Kind | Description |
| --- | --- | --- |
| [`IntervalId`](#intervalid) | struct |  |
| [`Signal`](#signal) | struct |  |
| [`EventChannel`](#eventchannel) | struct |  |
| [`Owner`](#owner) | struct |  |
| [`OwnerId`](#ownerid) | struct |  |
| [`Runtime`](#runtime) | struct |  |
| [`Subscription`](#subscription) | struct |  |
| [`DrawPhase`](#drawphase) | enum |  |
| [`Reactive`](#reactive) | trait |  |
| [`batch`](#batch) | fn |  |
| [`cancel_interval`](#cancel_interval) | fn |  |
| [`defer`](#defer) | fn |  |
| [`derived`](#derived) | fn |  |
| [`effect`](#effect) | fn |  |
| [`effect_named`](#effect_named) | fn |  |
| [`interval`](#interval) | fn |  |
| [`next_interval_deadline`](#next_interval_deadline) | fn |  |
| [`on_cleanup`](#on_cleanup) | fn |  |
| [`signal`](#signal-1) | fn |  |
| [`untrack`](#untrack) | fn |  |

## Structs

### `IntervalId`

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Signal`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#signal">Signal</a>&lt;T&gt;::update(self, f: impl FnOnce(&amp;mut T))</code> |  |
| <code>pub fn <a href="#signal">Signal</a>&lt;T&gt;::with&lt;R&gt;(self, f: impl FnOnce(&amp;T) -&gt; R) -&gt; R</code> |  |
| <code>pub fn <a href="#signal">Signal</a>&lt;T&gt;::get(self) -&gt; T</code> |  |
| <code>pub fn <a href="#signal">Signal</a>&lt;T&gt;::peek(self) -&gt; T</code> |  |
| <code>pub fn <a href="#signal">Signal</a>&lt;T&gt;::set(self, value: T)</code> |  |

**Trait implementations:** <code><a href="sigil-widgets.html#intolist">IntoList</a>&lt;T, R&gt;</code>, <code>AddAssign&lt;U&gt;</code>, <code>DivAssign&lt;U&gt;</code>, <code>MulAssign&lt;U&gt;</code>, <code>RemAssign&lt;U&gt;</code>, <code>SubAssign&lt;U&gt;</code>, <code>BitAndAssign&lt;U&gt;</code>, <code>BitOrAssign&lt;U&gt;</code>, <code>BitXorAssign&lt;U&gt;</code>, <code>ShlAssign&lt;U&gt;</code>, <code>ShrAssign&lt;U&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code><a href="#reactive">Reactive</a>&lt;T&gt;</code>, <code>Display</code>


### `EventChannel`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#eventchannel">EventChannel</a>&lt;T&gt;::emit(&amp;self, value: T)</code> |  |
| <code>pub fn <a href="#eventchannel">EventChannel</a>&lt;T&gt;::new() -&gt; Self</code> |  |
| <code>pub fn <a href="#eventchannel">EventChannel</a>&lt;T&gt;::subscribe(&amp;self, f: impl Fn(T) + 'static) -&gt; <a href="#subscription">Subscription</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>


### `Owner`

**Methods**

| Method | Description |
| --- | --- |
| <code>current() -&gt; Option&lt;<a href="#ownerid">OwnerId</a>&gt;</code> |  |
| <code>dispose(self)</code> |  |
| <code>const id(self) -&gt; <a href="#ownerid">OwnerId</a></code> |  |
| <code>new(name: &amp;'static str, parent: Option&lt;<a href="#ownerid">OwnerId</a>&gt;) -&gt; Self</code> |  |
| <code>with&lt;R&gt;(self, f: impl FnOnce() -&gt; R) -&gt; R</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `OwnerId`

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Runtime`

**Methods**

| Method | Description |
| --- | --- |
| <code>begin_effect()</code> |  |
| <code>begin_layout()</code> |  |
| <code>begin_paint()</code> |  |
| <code>end_frame()</code> |  |
| <code>frame_dirty() -&gt; bool</code> |  |
| <code>mark_frame_dirty()</code> |  |
| <code>panic_origin_info() -&gt; Option&lt;(&amp;'static str, &amp;'static Location&lt;'static&gt;)&gt;</code> |  |
| <code>recover_from_panic()</code> |  |
| <code>scope&lt;R&gt;(name: &amp;'static str, f: impl FnOnce() -&gt; R) -&gt; R</code> |  |

**Trait implementations:** <code>Debug</code>


### `Subscription`

**Trait implementations:** <code>Debug</code>, <code>Drop</code>


## Enums

### `DrawPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Effect</code> |  |
| <code>Idle</code> |  |
| <code>Layout</code> |  |
| <code>Paint</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `Reactive`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_signal(self) -&gt; <a href="#signal">Signal</a>&lt;T&gt; where Self: Sized, T: Clone + PartialEq + 'static</code> |  |
| <code>read(&amp;self) -&gt; T</code> |  |

**Trait implementations:** <code><a href="#reactive">Reactive</a>&lt;Vec&lt;T&gt;&gt;</code>


## Functions

### `batch`

<pre>pub fn <a href="#batch">batch</a>&lt;R&gt;(f: impl FnOnce() -&gt; R) -&gt; R</pre>


### `cancel_interval`

<pre>pub fn <a href="#cancel_interval">cancel_interval</a>(id: <a href="#intervalid">IntervalId</a>)</pre>


### `defer`

<pre>pub fn <a href="#defer">defer</a>(f: impl FnOnce() + 'static)</pre>


### `derived`

<pre>pub fn <a href="#derived">derived</a>&lt;T, F&gt;(f: F) -&gt; <a href="#signal">Signal</a>&lt;T&gt; where T: Clone + PartialEq + 'static, F: FnMut() -&gt; T + 'static</pre>


### `effect`

<pre>pub fn <a href="#effect">effect</a>&lt;F: FnMut() + 'static&gt;(f: F)</pre>


### `effect_named`

<pre>pub fn <a href="#effect_named">effect_named</a>&lt;F: FnMut() + 'static&gt;(name: &amp;'static str, f: F)</pre>


### `interval`

<pre>pub fn <a href="#interval">interval</a>(period: Duration, callback: impl Fn() + 'static) -&gt; <a href="#intervalid">IntervalId</a></pre>


### `next_interval_deadline`

<pre>pub fn <a href="#next_interval_deadline">next_interval_deadline</a>() -&gt; Option&lt;Instant&gt;</pre>


### `on_cleanup`

<pre>pub fn <a href="#on_cleanup">on_cleanup</a>(f: impl FnOnce() + 'static)</pre>


### `signal`

<pre>pub fn <a href="#signal-1">signal</a>&lt;T: 'static&gt;(initial: T) -&gt; <a href="#signal">Signal</a>&lt;T&gt;</pre>


### `untrack`

<pre>pub fn <a href="#untrack">untrack</a>&lt;R&gt;(f: impl FnOnce() -&gt; R) -&gt; R</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
