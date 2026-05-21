//! Scale handling regression coverage for the launch stack.

#![allow(missing_docs)]

use app as _;
use events as _;
use hal as _;
use layout as _;
use reactive as _;
use renderer as _;
use shell as _;
use style as _;
use tasks as _;
use text as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use hal::Scale;
use kernel::Cx;

#[test]
fn cx_scale_defaults_to_one_for_launch() {
    let cx = Cx::new_root();

    assert_eq!(cx.scale(), Scale::ONE);
}
