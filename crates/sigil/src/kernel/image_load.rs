use crate::kernel::image::data_uri_bytes;
use crate::kernel::{
    Cx, ImageSource, IntoNode, NodeId, ObjectFit, cached_image, fragment, image, node, text,
};
use crate::reactive::{Signal, signal};
use crate::tasks::Status;
use crate::tasks::current::scope_parts;
use crate::tasks::dispatch::spawn_into;
use crate::{Error, ErrorKind};

type Load = Status<ImageSource, Error>;
type View = Box<dyn Fn() -> Box<dyn IntoNode>>;
type ErrorView = Box<dyn Fn(&Error) -> Box<dyn IntoNode>>;

impl ImageSource {
    /// Loads a file path, `file://` URL, base64 `data:` URI, or http(s) URL in a browser off the UI thread, cached by location and, for a file, its modified time and size.
    #[must_use]
    pub fn load(location: impl Into<String>) -> Signal<Load> {
        let location = location.into();
        let key = cache_key(&location);
        if let Some(hit) = cached_image(key, || None) {
            return signal(Status::Ok(hit));
        }
        let status = signal(Status::Pending);
        #[cfg(target_arch = "wasm32")]
        if is_http(&location) {
            fetch_into(status, key, location);
            return status;
        }
        let target = status.into_sendable();
        let job = location.clone();
        let started = scope_parts("ImageSource::load").and_then(|(owner, parts)| {
            let work = async move { crate::tasks::blocking(move || read(&job)).await };
            spawn_into(&parts, owner, work, move |done| {
                target.into_signal().try_set_always(remember(key, done));
            })
        });
        if started.is_none() {
            status.set_always(remember(
                key,
                read(&location).map_or_else(Status::Err, Status::Ok),
            ));
        }
        status
    }
}

fn remember(key: u64, done: Load) -> Load {
    if let Status::Ok(source) = &done {
        drop(cached_image(key, || Some(source.clone())));
    }
    done
}

fn cache_key(location: &str) -> u64 {
    let key = crate::fnv::bytes(crate::fnv::OFFSET, location.as_bytes());
    if location.starts_with("data:") || is_http(location) {
        return key;
    }
    let Ok(meta) = std::fs::metadata(file_path(location)) else {
        return key;
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |at| at.as_nanos());
    let key = crate::fnv::bytes(key, &modified.to_le_bytes());
    crate::fnv::mix(key, meta.len())
}

fn file_path(location: &str) -> &str {
    location.strip_prefix("file://").unwrap_or(location)
}

fn is_http(location: &str) -> bool {
    location.starts_with("http://") || location.starts_with("https://")
}

#[cfg(target_arch = "wasm32")]
fn fetch_into(status: Signal<Load>, key: u64, url: String) {
    crate::net::fetch_bytes(url, move |fetched| {
        let done = fetched
            .map_err(|err| crate::net::fetch_failed("image", &err))
            .and_then(|bytes| ImageSource::decode_uncached(&bytes));
        status.try_set_always(remember(key, done.map_or_else(Status::Err, Status::Ok)));
    });
}

fn read(location: &str) -> Result<ImageSource, Error> {
    if location.starts_with("data:") {
        let bytes = data_uri_bytes(location).ok_or_else(|| {
            Error::fixed(
                ErrorKind::Other,
                "the data URI is not a base64 raster image",
            )
        })?;
        return ImageSource::decode_uncached(&bytes);
    }
    if is_http(location) {
        return Err(Error::fixed(
            ErrorKind::Fetch,
            "loading an image from an http URL is not supported yet",
        ));
    }
    let path = file_path(location);
    let bytes = std::fs::read(path)
        .map_err(|err| Error::message(ErrorKind::Io, format_args!("{path}: {err}")))?;
    ImageSource::decode_uncached(&bytes)
        .map_err(|err| Error::message(err.kind(), format_args!("{path}: {err}")))
}

/// An image that loads off the UI thread and shows a placeholder, then the image or an error.
#[must_use]
pub struct ImageLoadBuilder {
    status: Signal<Load>,
    alt: Option<String>,
    fit: ObjectFit,
    placeholder: View,
    error: ErrorView,
}

/// An image loaded from a file path, `file://` URL, base64 `data:` URI, or http(s) URL in a browser.
///
/// ```
/// use sigil::prelude::*;
///
/// fn logo() -> impl IntoNode {
///     load_image("assets/logo.png")
///         .alt("Logo")
///         .placeholder(|| text("Loading logo"))
///         .error(|err| text(format!("No logo: {err}")))
/// }
/// ```
pub fn load_image(location: impl Into<String>) -> ImageLoadBuilder {
    ImageLoadBuilder {
        status: ImageSource::load(location),
        alt: None,
        fit: ObjectFit::Fill,
        placeholder: Box::new(|| node(text("Loading image"))),
        error: Box::new(|err| node(text(format!("Could not load image: {err}")))),
    }
}

impl ImageLoadBuilder {
    /// Sets the alt text of the loaded image.
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.alt = Some(text.into());
        self
    }

    /// Sets how the loaded image fills its box.
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the view shown while the image loads.
    pub fn placeholder<N: IntoNode + 'static>(mut self, view: impl Fn() -> N + 'static) -> Self {
        self.placeholder = Box::new(move || node(view()));
        self
    }

    /// Sets the view shown when the image fails to load, given the reason.
    pub fn error<N: IntoNode + 'static>(mut self, view: impl Fn(&Error) -> N + 'static) -> Self {
        self.error = Box::new(move |err| node(view(err)));
        self
    }

    /// Returns the load status signal.
    #[must_use]
    pub const fn status(&self) -> Signal<Load> {
        self.status
    }

    fn view(&self, status: Load) -> Box<dyn IntoNode> {
        match status {
            Status::Ok(source) => {
                let shown = image(source).fit(self.fit);
                node(match &self.alt {
                    Some(alt) => shown.alt(alt.clone()),
                    None => shown,
                })
            }
            Status::Err(err) => (self.error)(&err),
            Status::Panicked(reason) => (self.error)(&Error::message(ErrorKind::Task, reason)),
            Status::Idle | Status::Pending => (self.placeholder)(),
        }
    }
}

impl core::fmt::Debug for ImageLoadBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImageLoadBuilder")
            .field("status", &self.status)
            .field("alt", &self.alt)
            .field("fit", &self.fit)
            .finish_non_exhaustive()
    }
}

impl IntoNode for ImageLoadBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let status = self.status;
        fragment()
            .children_reactive(move || [self.view(status.get())])
            .build(cx)
    }
}
