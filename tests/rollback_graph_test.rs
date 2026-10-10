use ma::graph_layout::{EdgeRoute, compute, compute_with_max_width};
use ma::graph_parser::parse_graph;

const ROLLBACK: &str = include_str!("fixtures/rollback.mmd");

#[test]
fn rollback_aligns_the_vertical_flow_and_keeps_a_stem_under_the_merge() {
    let diagram = parse_graph(ROLLBACK).unwrap();
    let layout = compute(&diagram).unwrap();
    let pages = layout.nodes.iter().find(|node| node.id == "pages").unwrap();
    for id in ["setup", "web", "server", "core", "find", "traffic"] {
        let node = layout.nodes.iter().find(|node| node.id == id).unwrap();
        assert_eq!(node.center_x, pages.center_x, "{id}");
    }
    for sg in &layout.subgraphs {
        assert!(
            sg.x <= pages.center_x && pages.center_x < sg.x + sg.width,
            "{} must keep the vertical spine inside the frame",
            sg.id
        );
    }
    let phase = layout.subgraphs.iter().find(|sg| sg.id == "phase").unwrap();
    let rendered = ma::graph_renderer::render(&layout);
    assert!(
        rendered.contains("┌─ rollback-release-servers.yaml ─"),
        "entry title stays at ┌─ Title ─:\n{rendered}"
    );
    let rows: Vec<_> = rendered.lines().collect();
    assert!(rows[phase.y - 2].contains('┌'), "{rendered}");
    assert!(rows[phase.y - 2].contains('╌'), "{rendered}");
    assert!(!rows[phase.y - 2].contains('▼'), "{rendered}");
    assert!(rows[phase.y - 1].contains('▼'), "{rendered}");
}

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

#[test]
fn rollback_places_pages_before_entry_and_phase_after_entry() {
    let diagram = parse_graph(ROLLBACK).unwrap();
    for layout in [
        compute(&diagram).unwrap(),
        compute_with_max_width(&diagram, 100).unwrap(),
    ] {
        let pages = layout.nodes.iter().find(|node| node.id == "pages").unwrap();
        let entry = layout.subgraphs.iter().find(|sg| sg.id == "entry").unwrap();
        let phase = layout.subgraphs.iter().find(|sg| sg.id == "phase").unwrap();
        assert!(pages.y + pages.height < entry.y);
        assert!(entry.y + entry.height < phase.y);
        assert!(
            layout
                .edges
                .iter()
                .all(|edge| edge.route == EdgeRoute::Forward)
        );
        let rendered = ma::graph_renderer::render(&layout);
        assert!(rendered.contains("release_ref の形式と存在"));
        assert!(rendered.contains("対象サービスを 3 段に分ける"));
        assert!(
            rendered.contains("rollback-release-servers.yaml"),
            "{rendered}"
        );
        assert_eq!(rendered.matches("成功したら").count(), 2, "{rendered}");
    }
}
