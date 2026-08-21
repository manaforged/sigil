use super::*;

#[test]
fn parse_round_trips_the_save_format() {
    let todos = parse("[x] done thing\n[ ] pending thing\ngarbage line\n");
    assert_eq!(
        todos,
        vec![
            Todo {
                text: "done thing".to_owned(),
                done: true
            },
            Todo {
                text: "pending thing".to_owned(),
                done: false
            },
        ]
    );
}
