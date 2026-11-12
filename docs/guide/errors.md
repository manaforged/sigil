# Errors

Sigil has one error type, `sigil::Error`. `err.kind()` returns a `sigil::ErrorKind`; branch on the kind, never on the message. `ErrorKind` is `#[non_exhaustive]`, so a new kind is not a breaking change. `source()` returns the cause when there is one, such as the wgpu or winit error behind a GPU or event-loop failure.

```rust
use sigil::{Error, ErrorKind};

fn check(err: &Error) -> bool {
    err.kind() == ErrorKind::Closed
}

let err = Error::message(ErrorKind::Clipboard, "read-only");
assert_eq!(err.to_string(), "read-only");
assert!(!check(&err));
```

`Error::new(kind, err)` wraps any `std::error::Error` with a kind, and `Error::message(kind, message)` builds one from a kind and a message. `Error::node()` returns the node of an `ErrorKind::NodeMissing` error.

## Migrating from the per-module error types

Each error type below is gone. Its messages are unchanged.

| Old | New |
| --- | --- |
| `app::ErrorKind` | `sigil::ErrorKind` |
| `Error::source_error()` | `std::error::Error::source()` |
| `tasks::ScheduleError::Closed` | `ErrorKind::Closed` |
| `tasks::ScheduleError::Full` | Removed; it was never returned |
| `a11y::BridgeError` | `sigil::Error`; it was never returned |
| `events::ShortcutParseError::Empty` | `ErrorKind::EmptyShortcut` |
| `events::ShortcutParseError::UnknownModifier(name)` | `ErrorKind::UnknownModifier` |
| `events::ShortcutParseError::UnknownKey(name)` | `ErrorKind::UnknownKey` |
| `clipboard::ClipError::new(msg)` | `Error::message(ErrorKind::Clipboard, msg)` |
| `shell::ShellError::EventLoop(err)` | `ErrorKind::EventLoop`; `source()` returns the winit error |
| `shell::ShellError::EventLoopClosed` | `ErrorKind::Closed` |
| `shell::ShellError::WakerPoisoned` | `ErrorKind::Poisoned` |
| `shell::ShellError::AlreadyRun` | `ErrorKind::AlreadyRun` |
| `shell::ShellError::Window(msg)` | `ErrorKind::Window` |
| `text::FontLoadError::Fetch(msg)` | `ErrorKind::Fetch` |
| `text::FontLoadError::Http(status)` | `ErrorKind::Http` |
| `text::FontLoadError::InvalidFont`, `text::TextError::InvalidFont`, `ShaperError::InvalidFont` | `ErrorKind::Font` |
| `renderer::RendererError::NoAdapter` | `ErrorKind::NoAdapter` |
| `renderer::RendererError::DeviceRequest(err)`, `SurfaceCreate(err)` | `ErrorKind::Gpu`; `source()` returns the wgpu error |
| `renderer::RendererError::NoSupportedFormat`, `SurfaceValidation` | `ErrorKind::Gpu` |
| `renderer::RendererError::NotInitialized` | `ErrorKind::NotInitialized` |
| `renderer::RendererError::InvalidViewport` | `ErrorKind::InvalidViewport` |
| `renderer::RendererError::Software(msg)` | `ErrorKind::Software` |
| `renderer::RendererError::Layer(LayerError::Empty)`, `renderer::LayerError::Empty` | `ErrorKind::EmptyLayer` |
| `renderer::RendererError::Layer(LayerError::TooLarge { .. })`, `renderer::LayerError::TooLarge { .. }` | `ErrorKind::LayerTooLarge` |
| `layout::LayoutError::LayoutInvalid(msg)` | `ErrorKind::Layout` |
| `layout::LayoutError::NodeMissing(id)` | `ErrorKind::NodeMissing`; `err.node()` returns `Some(id)` |
| `terminal::PrintAboveError::NotLive` | `ErrorKind::NotLive`, inside the `io::Error` |
| `terminal::PrintAboveError::FullScreen` | `ErrorKind::NotInline`, inside the `io::Error` |

A layout or renderer error that reaches `sigil::run` through `?` keeps its own kind. Before, it became `ErrorKind::Other` or `ErrorKind::Gpu`. `clipboard::copy` and `clipboard::image` return the back end's error as it is, so a `Clipboard` implementation's kind reaches the caller; before, they always reported `ErrorKind::Io`.
