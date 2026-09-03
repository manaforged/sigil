# Async

Background work returns a **signal**. It does not return an
`Entity` update token.

```rust,ignore
let rows = spawn(async move { fetch_rows().await });
// Signal<Status<Vec<Row>, _>>
label(derived(move || match rows.get() {
    Status::Pending => "Loading…".into(),
    Status::Ok(list) => format!("{} rows", list.len()),
    Status::Err(_) => "Failed".into(),
}));
```

## `spawn`

`sigil::prelude::spawn` is the product helper.

- Must run inside a reactive scope (`launch` or `Runtime::scope`).
- Uses the current owner. Dispose that owner and the future aborts.
- Requires `AsyncRuntime::install_current`. `launch` does this.

The future's `Output` is `Result<T, E>`. The signal starts
`Status::Pending`. On completion the main-thread dispatcher writes
`Ok` or `Err`. Cancellation does **not** write `Err`; the signal
stays `Pending` and the task drops.

## `dispatch_to_owner`

Post a `FnOnce()` to the main thread against an `OwnerHandle`. The
closure runs on the next drain with that owner current, or it is
dropped if the owner is gone.

Use this when a worker already exists and you only need to write
signals. Use `spawn` when you have a future.

For repeated writes to one signal, create a `SignalWriter` on the reactive
thread and move it into the worker:

```rust,ignore
let rows = signal(Vec::new());
let writer = signal_writer(rows);
std::thread::spawn(move || {
    writer.update(|rows| rows.push(load_row()?))?;
    Ok::<_, ScheduleError>(())
});
```

The writer keeps the current owner. Writes are dropped if that owner no
longer exists.

## What not to call

`EntityCx::spawn` + `AsyncContext::update` is the hidden Entity seam.
Widgets do not use it. Tests in `g5_executor` still do. Product
async is `spawn` → `Signal<Status>`.

## Frame timing

`launch` pumps scheduler writes and entity completions **before**
`begin_effect`, so finished work is visible in this frame's layout and paint.
Both queues share the winit waker, so the loop does not wait for an unrelated
OS event.
