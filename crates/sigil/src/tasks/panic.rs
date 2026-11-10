use core::any::Any;

/// Returns the text of a caught panic payload.
#[must_use]
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload.downcast_ref::<&'static str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .map_or("<non-string panic payload>", String::as_str)
        },
        |message| message,
    )
}
