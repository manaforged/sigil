use sigil::animation::active_count;

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
        Err(Error::unsettled(SETTLE_SAFETY_CAP, active_count()))
    } else {
        Ok(())
    }
}
