use super::Date;

#[test]
fn parse_rejects_bad_days() {
    assert!(Date::parse("2026-02-29").is_none());
    assert_eq!(
        Date::parse("2024-02-29"),
        Some(Date {
            year: 2024,
            month: 2,
            day: 29
        })
    );
    assert!(Date::parse("2026-13-01").is_none());
    assert_eq!(
        Date::parse("2026-08-17")
            .expect("2026-08-17 is a valid date")
            .render(),
        "2026-08-17"
    );
}
