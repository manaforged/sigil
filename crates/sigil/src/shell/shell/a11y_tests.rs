use super::*;

#[test]
fn activation_flags_one_pending_publish_until_deactivated() {
    let activation = Activation::default();
    assert!(!activation.is_active());
    assert!(!activation.take_pending());

    activation.activate();
    assert!(activation.is_active());
    assert!(activation.take_pending());
    assert!(!activation.take_pending());
    assert!(activation.is_active());

    activation.activate();
    activation.deactivate();
    assert!(!activation.is_active());
    assert!(!activation.take_pending());
}
