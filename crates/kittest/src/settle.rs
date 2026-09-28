use sigil::animation_runtime::active_count;

use crate::{Error, SETTLE_SAFETY_CAP};

pub(crate) fn settle(
    busy: impl Fn() -> bool,
    mut frame: impl FnMut() -> Result<(), Error>,
) -> Result<(), Error> {
    for _ in 0..SETTLE_SAFETY_CAP {
        if !busy() {
            return Ok(());
        }
        frame()?;
    }
    if busy() {
        return Err(Error::Unsettled {
            frames: SETTLE_SAFETY_CAP,
            active: active_count(),
        });
    }
    Ok(())
}
