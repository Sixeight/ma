use ma::graph_layout::{EdgeRoute, compute, compute_with_max_width};
use ma::graph_parser::parse_graph;

const ROLLBACK: &str = include_str!("fixtures/rollback.mmd");

#[test]
fn rollback_retains_setup_label_declared_after_reference() {
    let diagram = parse_graph(ROLLBACK).unwrap();
    let setup = diagram
        .nodes
        .iter()
        .find(|node| node.id == "setup")
        .unwrap();
    assert!(setup.label.contains("release_ref の形式と存在"));
    assert!(setup.label.contains("対象サービスを 3 段に分ける"));
}
