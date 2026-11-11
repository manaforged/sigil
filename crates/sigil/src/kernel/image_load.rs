use crate::kernel::image::data_uri_bytes;
use crate::kernel::{ImageSource, cached_image};
use crate::reactive::{Signal, signal};
use crate::tasks::Status;
use crate::tasks::current::scope_parts;
use crate::tasks::dispatch::spawn_into;
use crate::{Error, ErrorKind};

pub(crate) type Load = Status<ImageSource, Error>;

impl ImageSource {
    /// Loads a file path, `file://` URL, base64 `data:` URI, or http(s) URL (feature `image-http` outside a browser) off the UI thread, cached by location and, for a file, its modified time and size.
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

#[cfg(native_http)]
fn fetch(url: &str) -> Result<ImageSource, Error> {
    crate::net::get(url)
        .map_err(|err| crate::net::fetch_failed("image", &err))
        .and_then(|bytes| ImageSource::decode_uncached(&bytes))
}

#[cfg(not(native_http))]
const fn fetch(_url: &str) -> Result<ImageSource, Error> {
    Err(Error::fixed(
        ErrorKind::Fetch,
        "loading an image from an http URL needs the image-http feature",
    ))
}

fn file_path(location: &str) -> std::path::PathBuf {
    match url::Url::parse(location) {
        Ok(url) if url.scheme() == "file" => {
            url.to_file_path().unwrap_or_else(|()| location.into())
        }
        _ => location.into(),
    }
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
        return fetch(location);
    }
    let path = file_path(location);
    let shown = path.display();
    let bytes = std::fs::read(&path)
        .map_err(|err| Error::message(ErrorKind::Io, format_args!("{shown}: {err}")))?;
    ImageSource::decode_uncached(&bytes)
        .map_err(|err| Error::message(err.kind(), format_args!("{shown}: {err}")))
}
