use super::*;

#[test]
fn activation_is_off_until_a_platform_request() {
    let activation = Activation::default();
    assert!(!activation.is_active());
    assert!(!activation.take_pending());
}

#[test]
fn activation_flags_one_pending_publish() {
    let activation = Activation::default();
    activation.activate();
    assert!(activation.is_active());
    assert!(activation.take_pending());
    assert!(!activation.take_pending());
    assert!(activation.is_active());
}

#[test]
fn deactivation_clears_both_flags() {
    let activation = Activation::default();
    activation.activate();
    activation.deactivate();
    assert!(!activation.is_active());
    assert!(!activation.take_pending());
}
