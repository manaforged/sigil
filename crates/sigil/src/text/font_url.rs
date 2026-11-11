use crate::Error;
use crate::net::{fetch_bytes, fetch_failed};
use crate::reactive::{Signal, signal};
use crate::tasks::Status;
use crate::text::fonts::{add_fallback_font, wake_font_listeners};

#[allow(
    clippy::must_use_candidate,
    reason = "starts the font fetch; the status signal is optional"
)]
pub fn load_font_url(url: &str, face_index: u32) -> Signal<Status<(), Error>> {
    let status = signal(Status::Pending);
    fetch_bytes(url.to_owned(), move |fetched| {
        let outcome = fetched
            .map_err(|err| fetch_failed("font", &err))
            .and_then(|bytes| add_fallback_font(bytes, face_index));
        status.try_set_always(outcome.map_or_else(Status::Err, Status::Ok));
        wake_font_listeners();
    });
    status
}
