use super::*;

#[test]
fn first_page_places_then_fills_the_shelf() {
    let mut pack = ImagePack::default();
    assert!(pack.try_place(16, 16).is_none());
    let a = pack.push_page(0, 16, 16).unwrap();
    assert_eq!(a.x, 0);
    assert_eq!(a.y, 0);
    let b = pack.try_place(16, 16).unwrap();
    assert_eq!(b.x, 17);
    assert_eq!(b.y, 0);
}

#[test]
fn overflow_opens_a_new_shelf() {
    let mut pack = ImagePack::default();
    assert!(pack.push_page(0, PAGE - 1, 16).is_some());
    let next = pack.try_place(32, 16).unwrap();
    assert_eq!(next.x, 0);
    assert_eq!(next.y, 17);
}
