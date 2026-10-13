#[allow(dead_code)]
#[path = "story-02-counter.rs"]
pub(crate) mod counter;

fn main() -> sigil::AppResult {
    sigil::run("Counter", counter::counter)
}

#[test]
fn plus_increments() {
    sigil_kittest::Harness::new(counter::counter)
        .click("+")
        .expect_text("Count: 1");
}
