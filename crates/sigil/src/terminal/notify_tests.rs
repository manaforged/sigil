use super::{Activity, activity_sequence};

#[test]
fn activity_writes_the_osc_7501_report_hosts_read() {
    assert_eq!(
        activity_sequence("summonr", Activity::Working),
        "\x1b]7501;state=working:app=summonr\x1b\\"
    );
    assert_eq!(
        activity_sequence("a:state=idle\x1b", Activity::Blocked),
        "\x1b]7501;state=blocked:app=a-state-idle-\x1b\\"
    );
}
