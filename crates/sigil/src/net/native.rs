use std::cell::RefCell;
use std::time::Duration;

use leyline::blocking::Session;

use crate::{Error, ErrorKind};

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BODY_BYTES: usize = 64 << 20;

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

pub(crate) fn get(url: &str) -> Result<Vec<u8>, Error> {
    SESSION.with_borrow_mut(|slot| {
        let session = match slot {
            Some(session) => session,
            None => slot.insert(open().map_err(|err| failed(url, &err))?),
        };
        session
            .block_on(async { session.get(url).error_for_status().bytes().await })
            .map(|body| body.to_vec())
            .map_err(|err| failed(url, &err))
    })
}

fn open() -> leyline::Result<Session> {
    Session::new(
        leyline::Session::builder()
            .timeout(TIMEOUT)
            .max_body_size(MAX_BODY_BYTES),
    )
}

fn failed(url: &str, err: &leyline::Error) -> Error {
    let kind = if err.status().is_some() {
        ErrorKind::Http
    } else {
        ErrorKind::Fetch
    };
    Error::message(kind, format_args!("{url}: {err}"))
}
