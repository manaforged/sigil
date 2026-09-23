use core::any::Any;

#[must_use]
pub fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload.downcast_ref::<&'static str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .map_or("<non-string panic payload>", String::as_str)
        },
        |message| message,
    )
}
