use super::*;

#[test]
fn generate_emits_requested_count() {
    assert_eq!(generate(10).len(), 10);
}

#[test]
fn same_index_is_deterministic() {
    let a = quote_at(0);
    let b = quote_at(0);
    assert_eq!(a.symbol, b.symbol);
    assert_eq!(a.name, b.name);
    assert_eq!(a.volume, b.volume);
}

#[test]
fn change_sign_follows_last_done() {
    let mut quote = quote_at(0);
    quote.last_done = quote.prev_close + 1.0;
    assert!(quote.up());
    quote.last_done = quote.prev_close - 1.0;
    assert!(!quote.up());
}
