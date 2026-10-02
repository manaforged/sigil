# Module `sigil::tasks`

| Item | Kind | Description |
| --- | --- | --- |
| [`AsyncRuntime`](#asyncruntime) | struct |  |
| [`SignalWriter`](#signalwriter) | struct |  |
| [`Task`](#task) | struct |  |
| [`OwnerHandle`](#ownerhandle) | struct |  |
| [`TaskHandle`](#taskhandle) | struct |  |
| [`ScheduleError`](#scheduleerror) | enum |  |
| [`Status`](#status) | enum |  |
| [`TaskWorkers`](#taskworkers) | enum |  |
| [`MessageReceiver`](#messagereceiver) | trait |  |
| [`installed`](#installed) | fn |  |
| [`on_message`](#on_message) | fn |  |
| [`on_message_or_close`](#on_message_or_close) | fn |  |
| [`panic_message`](#panic_message) | fn |  |
| [`signal_writer`](#signal_writer) | fn |  |
| [`sleep`](#sleep) | fn |  |
| [`spawn`](#spawn) | fn |  |
| [`task`](#task-1) | fn |  |

## Structs

### `AsyncRuntime`

**Methods**

| Method | Description |
| --- | --- |
| <code>install_current(&amp;self)</code> |  |
| <code>new() -&gt; Result&lt;Self&gt;</code> |  |
| <code>pump(&amp;mut self) -&gt; usize</code> |  |
| <code>set_waker&lt;F&gt;(&amp;mut self, waker: F) where F: Fn() + Send + Sync + 'static</code> |  |
| <code>with_workers(workers: <a href="#taskworkers">TaskWorkers</a>) -&gt; Result&lt;Self&gt;</code> |  |

**Trait implementations:** <code><a href="sigil-hal.html#scheduler">Scheduler</a></code>, <code>Debug</code>, <code>Drop</code>


### `SignalWriter`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#signalwriter">SignalWriter</a>&lt;T&gt;::set(&amp;self, value: T) -&gt; Result&lt;(), <a href="#scheduleerror">ScheduleError</a>&gt;</code> |  |
| <code>pub fn <a href="#signalwriter">SignalWriter</a>&lt;T&gt;::update&lt;F&gt;(&amp;self, update: F) -&gt; Result&lt;(), <a href="#scheduleerror">ScheduleError</a>&gt; where F: FnOnce(&amp;mut T) + Send + 'static</code> |  |

**Trait implementations:** <code>Clone</code>


### `Task`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#task">Task</a>&lt;T, E&gt;::run(self)</code> |  |
| <code>pub fn <a href="#task">Task</a>&lt;T, E&gt;::status(self) -&gt; <a href="#status">Status</a>&lt;T, E&gt; where T: Clone, E: Clone</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Copy</code>


### `OwnerHandle`

**Methods**

| Method | Description |
| --- | --- |
| <code>current() -&gt; Option&lt;Self&gt;</code> |  |
| <code>const from_id(id: <a href="sigil-reactive.html#ownerid">OwnerId</a>) -&gt; Self</code> |  |
| <code>const id(self) -&gt; <a href="sigil-reactive.html#ownerid">OwnerId</a></code> |  |
| <code>is_alive(&amp;self) -&gt; bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TaskHandle`

**Methods**

| Method | Description |
| --- | --- |
| <code>abort(&amp;self)</code> |  |
| <code>is_finished(&amp;self) -&gt; bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


## Enums

### `ScheduleError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Closed</code> |  |
| <code>Full</code> |  |

**Trait implementations:** <code>Error</code>, <code>Debug</code>, <code>Display</code>


### `Status`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#status">Status</a>&lt;T, E&gt;::is_err(&amp;self) -&gt; bool</code> |  |
| <code>pub const fn <a href="#status">Status</a>&lt;T, E&gt;::is_ok(&amp;self) -&gt; bool</code> |  |
| <code>pub const fn <a href="#status">Status</a>&lt;T, E&gt;::is_panicked(&amp;self) -&gt; bool</code> |  |
| <code>pub const fn <a href="#status">Status</a>&lt;T, E&gt;::is_pending(&amp;self) -&gt; bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Err(E)</code> |  |
| <code>Idle</code> |  |
| <code>Ok(T)</code> |  |
| <code>Panicked(String)</code> |  |
| <code>Pending</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `TaskWorkers`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Fixed(NonZeroUsize)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `MessageReceiver`

**Methods**

| Method | Description |
| --- | --- |
| <code>recv(&amp;mut self) -&gt; impl Future&lt;Output = Option&lt;T&gt;&gt; + Send + '_</code> |  |


## Functions

### `installed`

<pre>pub fn <a href="#installed">installed</a>() -&gt; bool</pre>


### `on_message`

<pre>pub fn <a href="#on_message">on_message</a>&lt;T, F&gt;(rx: impl <a href="#messagereceiver">MessageReceiver</a>&lt;T&gt;, f: F) where T: Send + 'static, F: Fn(T) + 'static</pre>


### `on_message_or_close`

<pre>pub fn <a href="#on_message_or_close">on_message_or_close</a>&lt;T, F, C&gt;(rx: impl <a href="#messagereceiver">MessageReceiver</a>&lt;T&gt;, f: F, on_close: C) where T: Send + 'static, F: Fn(T) + 'static, C: FnOnce() + 'static</pre>


### `panic_message`

<pre>pub fn <a href="#panic_message">panic_message</a>(payload: &amp;(dyn Any + Send)) -&gt; &amp;str</pre>


### `signal_writer`

<pre>pub fn <a href="#signal_writer">signal_writer</a>&lt;T&gt;(signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;T&gt;) -&gt; <a href="#signalwriter">SignalWriter</a>&lt;T&gt; where T: Send + 'static</pre>


### `sleep`

<pre>pub async fn <a href="#sleep">sleep</a>(duration: Duration)</pre>


### `spawn`

<pre>pub fn <a href="#spawn">spawn</a>&lt;T, E, F&gt;(future: F) -&gt; <a href="sigil-reactive.html#signal">Signal</a>&lt;<a href="#status">Status</a>&lt;T, E&gt;&gt; where T: Send + 'static, E: Send + 'static, F: Future&lt;Output = Result&lt;T, E&gt;&gt; + Send + 'static</pre>


### `task`

<pre>pub fn <a href="#task-1">task</a>&lt;T, E, F, Fut&gt;(make: F) -&gt; <a href="#task">Task</a>&lt;T, E&gt; where T: Send + 'static, E: Send + 'static, F: Fn() -&gt; Fut + 'static, Fut: Future&lt;Output = Result&lt;T, E&gt;&gt; + Send + 'static</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
