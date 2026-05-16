//! Scale handling regression coverage for the launch stack.

#![allow(missing_docs)]

use sigil_app as _;
use sigil_async as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use taffy as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_hal::Scale;
use sigil_kernel::Cx;

#[test]
fn cx_scale_defaults_to_one_for_launch() {
    let cx = Cx::new_root();

    assert_eq!(cx.scale(), Scale::ONE);
}
