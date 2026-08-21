use super::{draw, flow_edges};

#[test]
fn flowchart_chain_boxes_both_nodes() {
    let lines = draw("graph TD\nA[Start] --> B[End]\n");
    let body = lines.join("\n");
    assert!(body.contains("Start"), "{body}");
    assert!(body.contains("End"), "{body}");
    assert!(body.contains('┌'), "{body}");
}

#[test]
fn sequence_falls_back_to_framed_source() {
    let lines = draw("sequenceDiagram\nAlice->>Bob: hi\n");
    let body = lines.join("\n");
    assert!(body.contains("sequenceDiagram"), "{body}");
    assert!(body.contains('┌'), "{body}");
}

#[test]
fn parses_plain_ids() {
    let edges = flow_edges("flowchart TD\nA --> B\n").expect("flow");
    assert_eq!(edges, vec![("A".into(), "B".into())]);
}
