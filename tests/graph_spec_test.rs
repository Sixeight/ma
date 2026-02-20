use pretty_assertions::assert_eq;

// =============================================================================
// Direction
// =============================================================================

#[test]
fn spec_graph_td() {
    let input = "graph TD\n    A --> B\n";
    let output = ma::render(input).unwrap();
    let a = output.find("│ A │").unwrap();
    let b = output.find("│ B │").unwrap();
    assert!(a < b, "A above B in TD");
    assert!(output.contains('▼'), "TD arrow has ▼");
}

#[test]
fn spec_graph_tb_same_as_td() {
    let td = ma::render("graph TD\n    A --> B\n").unwrap();
    let tb = ma::render("graph TB\n    A --> B\n").unwrap();
    assert_eq!(td, tb, "TB produces same output as TD");
}

#[test]
fn spec_graph_lr() {
    let input = "graph LR\n    A --> B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "LR single row = 3 lines (box height)");
    assert!(output.contains('>'), "LR arrow has >");
    assert!(!output.contains('▼'), "LR does not use ▼");
}

#[test]
fn spec_flowchart_keyword() {
    let graph = ma::render("graph TD\n    A --> B\n").unwrap();
    let flowchart = ma::render("flowchart TD\n    A --> B\n").unwrap();
    assert_eq!(graph, flowchart, "flowchart keyword behaves same as graph");
}

// =============================================================================
// Nodes
// =============================================================================

#[test]
fn spec_node_implicit_label() {
    let input = "graph TD\n    MyNode --> Other\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ MyNode │"), "uses ID as label");
    assert!(output.contains("│ Other │"));
}

#[test]
fn spec_node_explicit_label() {
    let input = "graph TD\n    A[Start] --> B[End]\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ Start │"), "uses bracket label");
    assert!(output.contains("│ End │"));
    assert!(!output.contains("│ A │"), "does not show raw ID");
    assert!(!output.contains("│ B │"));
}

#[test]
fn spec_node_label_with_spaces() {
    let input = "graph TD\n    A[Hello World]\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ Hello World │"));
}

#[test]
fn spec_node_dedup_first_label_wins() {
    let input = "graph TD\n    A[First] --> B\n    A[Second] --> C\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("First"), "first-seen label kept");
    assert!(!output.contains("Second"), "later label ignored");
}

#[test]
fn spec_node_box_structure() {
    let input = "graph TD\n    A[Hi]\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌────┐
│ Hi │
└────┘";
    assert_eq!(output, expected);
}

// =============================================================================
// Node Shapes
// =============================================================================

#[test]
fn spec_node_round_shape() {
    let input = "graph TD\n    A(Hello)\n";
    let output = ma::render(input).unwrap();
    let expected = "\
╭───────╮
│ Hello │
╰───────╯";
    assert_eq!(output, expected);
}

#[test]
fn spec_node_diamond_shape() {
    let input = "graph TD\n    A{Hello}\n";
    let output = ma::render(input).unwrap();
    let expected = "\
╱───────╲
│ Hello │
╲───────╱";
    assert_eq!(output, expected);
}

#[test]
fn spec_node_circle_shape() {
    let input = "graph TD\n    A((Hello))\n";
    let output = ma::render(input).unwrap();
    let expected = "\
╭───────────╮
│   Hello   │
╰───────────╯";
    assert_eq!(output, expected);
}

#[test]
fn spec_node_round_with_edge() {
    let input = "graph TD\n    A(Start) --> B[End]\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('╭'), "round node has ╭ corner");
    assert!(output.contains('╯'), "round node has ╯ corner");
    assert!(output.contains('┌'), "box node has ┌ corner");
    assert!(output.contains('▼'), "arrow present");
}

#[test]
fn spec_node_mixed_shapes_lr() {
    let input = "graph LR\n    A(Round) --> B{Diamond}\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('╭'), "round node has ╭");
    assert!(output.contains('╱'), "diamond node has ╱");
    assert!(output.contains('>'), "arrow present");
}

// =============================================================================
// Edges — Arrow (-->)
// =============================================================================

#[test]
fn spec_edge_td_arrow() {
    let input = "graph TD\n    A --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('┬'), "parent bottom has ┬");
    assert!(output.contains('▼'), "target has ▼");
}

#[test]
fn spec_edge_lr_arrow() {
    let input = "graph LR\n    A --> B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let arrow_line = lines[1];
    assert!(arrow_line.contains("──"), "horizontal line between nodes");
    assert!(arrow_line.contains('>'), "arrow head at target");
}

// =============================================================================
// Edges — Open Link (---)
// =============================================================================

#[test]
fn spec_edge_td_open_link() {
    let input = "graph TD\n    A --- B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('┬'), "parent bottom has ┬");
    assert!(!output.contains('▼'), "no ▼ for open link");
}

#[test]
fn spec_edge_lr_open_link() {
    let input = "graph LR\n    A --- B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let conn_line = lines[1];
    assert!(conn_line.contains("──"), "horizontal line");
    assert!(!conn_line.contains('>'), "no arrow head for open link");
}

// =============================================================================
// TD Layout
// =============================================================================

#[test]
fn spec_td_linear_chain() {
    let input = "graph TD\n    A[Start] --> B[End]\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───────┐
│ Start │
└───┬───┘
    │
    ▼
 ┌─────┐
 │ End │
 └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_td_three_node_chain() {
    let input = "graph TD\n    A --> B\n    B --> C\n";
    let output = ma::render(input).unwrap();

    let a_pos = output.find("│ A │").unwrap();
    let b_pos = output.find("│ B │").unwrap();
    let c_pos = output.find("│ C │").unwrap();
    assert!(a_pos < b_pos, "A before B");
    assert!(b_pos < c_pos, "B before C");

    let arrow_count = output.matches('▼').count();
    assert_eq!(arrow_count, 2, "two arrows in chain");
}

#[test]
fn spec_td_fan_out() {
    let input = "graph TD\n    A --> B\n    A --> C\n";
    let output = ma::render(input).unwrap();
    let expected = concat!(
        "    ┌───┐\n",
        "    │ A │\n",
        "    └─┬─┘\n",
        "  ┌───┴───┐\n",
        "  ▼       ▼\n",
        "┌───┐   ┌───┐\n",
        "│ B │   │ C │\n",
        "└───┘   └───┘",
    );
    assert_eq!(output, expected);
}

#[test]
fn spec_td_fan_out_structure() {
    let input = "graph TD\n    A --> B\n    A --> C\n";
    let output = ma::render(input).unwrap();

    assert!(output.contains('┴'), "fan-out bar has ┴ at parent center");
    assert_eq!(output.matches('▼').count(), 2, "▼ at each child");

    let b_line = output.lines().find(|l| l.contains("│ B │")).unwrap();
    let c_line = output.lines().find(|l| l.contains("│ C │")).unwrap();
    assert_eq!(
        output.lines().position(|l| l == b_line),
        output.lines().position(|l| l == c_line),
        "B and C on same row"
    );
}

#[test]
fn spec_td_fan_in() {
    let input = "graph TD\n    A --> C\n    B --> C\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐   ┌───┐
│ A │   │ B │
└─┬─┘   └─┬─┘
  └───┬───┘
      ▼
    ┌───┐
    │ C │
    └───┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_td_fan_in_structure() {
    let input = "graph TD\n    A --> C\n    B --> C\n";
    let output = ma::render(input).unwrap();

    let a_line = output.lines().find(|l| l.contains("│ A │")).unwrap();
    let b_line = output.lines().find(|l| l.contains("│ B │")).unwrap();
    assert_eq!(
        output.lines().position(|l| l == a_line),
        output.lines().position(|l| l == b_line),
        "A and B on same row"
    );

    assert!(output.contains('└'), "merge bar has └");
    assert!(output.contains('┘'), "merge bar has ┘");
    assert_eq!(output.matches('▼').count(), 1, "single ▼ at child");
}

// =============================================================================
// LR Layout
// =============================================================================

#[test]
fn spec_lr_linear_chain() {
    let input = "graph LR\n    A[Start] --> B[End]\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───────┐     ┌─────┐
│ Start │────>│ End │
└───────┘     └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_lr_open_link() {
    let input = "graph LR\n    A --- B\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐     ┌───┐
│ A │─────│ B │
└───┘     └───┘";
    assert_eq!(output, expected);
}

// =============================================================================
// Edge Labels
// =============================================================================

#[test]
fn spec_td_edge_label_arrow() {
    let input = "graph TD\n    A -->|yes| B\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐
│ A │
└─┬─┘
 yes
  ▼
┌───┐
│ B │
└───┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_td_edge_label_open_link() {
    let input = "graph TD\n    A ---|label| B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("label"), "label text rendered");
    assert!(!output.contains('▼'), "no arrow for open link");
}

#[test]
fn spec_lr_edge_label() {
    let input = "graph LR\n    A -->|yes| B\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐ yes ┌───┐
│ A │────>│ B │
└───┘     └───┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_lr_edge_label_long() {
    let input = "graph LR\n    A -->|long label| B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines[0].contains("long label"), "long label rendered");
    assert!(lines[1].contains('>'), "arrow present");
}

#[test]
fn spec_edge_no_label() {
    let input = "graph TD\n    A --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('│'), "connector line present when no label");
}

#[test]
fn spec_fan_out_label_parsed_but_not_drawn() {
    let input = "graph TD\n    A -->|yes| B\n    A -->|no| C\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "parent rendered");
    assert!(output.contains("│ B │"), "child B rendered");
    assert!(output.contains("│ C │"), "child C rendered");
}

// =============================================================================
// Edge Labels — Alternative syntax (-- text -->)
// =============================================================================

#[test]
fn spec_alt_label_arrow_same_as_pipe() {
    let pipe = ma::render("graph TD\n    A -->|yes| B\n").unwrap();
    let alt = ma::render("graph TD\n    A -- yes --> B\n").unwrap();
    assert_eq!(pipe, alt, "-- text --> produces same output as -->|text|");
}

#[test]
fn spec_alt_label_open_link_same_as_pipe() {
    let pipe = ma::render("graph TD\n    A ---|label| B\n").unwrap();
    let alt = ma::render("graph TD\n    A -- label --- B\n").unwrap();
    assert_eq!(pipe, alt, "-- text --- produces same output as ---|text|");
}

#[test]
fn spec_alt_label_lr_arrow() {
    let pipe = ma::render("graph LR\n    A -->|yes| B\n").unwrap();
    let alt = ma::render("graph LR\n    A -- yes --> B\n").unwrap();
    assert_eq!(pipe, alt, "LR: -- text --> same as -->|text|");
}

#[test]
fn spec_alt_label_with_spaces() {
    let input = "graph LR\n    A -- hello world --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("hello world"), "label with spaces rendered");
}

// =============================================================================
// Edges — Dotted Arrow (-.->)
// =============================================================================

#[test]
fn spec_edge_td_dotted_arrow() {
    let input = "graph TD\n    A -.-> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('┬'), "parent bottom has ┬");
    assert!(output.contains('┊'), "dotted vertical connector ┊");
    assert!(output.contains('▼'), "arrow head ▼");
}

#[test]
fn spec_edge_lr_dotted_arrow() {
    let input = "graph LR\n    A -.-> B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let arrow_line = lines[1];
    assert!(arrow_line.contains('╌'), "dotted horizontal connector ╌");
    assert!(arrow_line.contains('>'), "arrow head at target");
}

#[test]
fn spec_edge_td_dotted_link() {
    let input = "graph TD\n    A -.- B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('┊'), "dotted vertical connector ┊");
    assert!(!output.contains('▼'), "no arrow for dotted link");
}

#[test]
fn spec_edge_lr_dotted_link() {
    let input = "graph LR\n    A -.- B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let conn_line = lines[1];
    assert!(conn_line.contains('╌'), "dotted horizontal connector");
    assert!(!conn_line.contains('>'), "no arrow head for dotted link");
}

// =============================================================================
// Edges — Thick Arrow (==>)
// =============================================================================

#[test]
fn spec_edge_td_thick_arrow() {
    let input = "graph TD\n    A ==> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('┬'), "parent bottom has ┬");
    assert!(output.contains('║'), "thick vertical connector ║");
    assert!(output.contains('▼'), "arrow head ▼");
}

#[test]
fn spec_edge_lr_thick_arrow() {
    let input = "graph LR\n    A ==> B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let arrow_line = lines[1];
    assert!(arrow_line.contains('═'), "thick horizontal connector ═");
    assert!(arrow_line.contains('>'), "arrow head at target");
}

#[test]
fn spec_edge_td_thick_link() {
    let input = "graph TD\n    A === B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains('║'), "thick vertical connector ║");
    assert!(!output.contains('▼'), "no arrow for thick link");
}

#[test]
fn spec_edge_lr_thick_link() {
    let input = "graph LR\n    A === B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let conn_line = lines[1];
    assert!(conn_line.contains('═'), "thick horizontal connector");
    assert!(!conn_line.contains('>'), "no arrow head for thick link");
}

// =============================================================================
// Edge Labels with dotted/thick edges
// =============================================================================

#[test]
fn spec_td_dotted_edge_label() {
    let input = "graph TD\n    A -.->|yes| B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("yes"), "label text rendered");
    assert!(output.contains('▼'), "arrow head present");
}

#[test]
fn spec_lr_dotted_edge_label() {
    let input = "graph LR\n    A -.->|yes| B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines[0].contains("yes"), "label rendered above edge");
    assert!(lines[1].contains('>'), "arrow present");
    assert!(lines[1].contains('╌'), "dotted connector used");
}

#[test]
fn spec_td_thick_edge_label() {
    let input = "graph TD\n    A ==>|go| B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("go"), "label text rendered");
    assert!(output.contains('▼'), "arrow head present");
}

#[test]
fn spec_lr_thick_edge_label() {
    let input = "graph LR\n    A ==>|go| B\n";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines[0].contains("go"), "label rendered above edge");
    assert!(lines[1].contains('>'), "arrow present");
    assert!(lines[1].contains('═'), "thick connector used");
}

// =============================================================================
// Subgraphs
// =============================================================================

#[test]
fn spec_subgraph_single_node() {
    let input = "graph TD\n    subgraph Group\n        A\n    end\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌─ Group ─┐
│ ┌───┐   │
│ │ A │   │
│ └───┘   │
└─────────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_subgraph_with_edge() {
    let input = "graph TD\n    subgraph Backend\n        A[API] --> B[DB]\n    end\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌─ Backend ─┐
│ ┌─────┐   │
│ │ API │   │
│ └──┬──┘   │
│    │      │
│    ▼      │
│ ┌────┐    │
│ │ DB │    │
│ └────┘    │
└───────────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_subgraph_border_contains_nodes() {
    let input = "graph TD\n    subgraph S\n        A --> B\n    end\n";
    let output = ma::render(input).unwrap();

    let lines: Vec<&str> = output.lines().collect();
    let first = lines[0];
    let last = lines[lines.len() - 1];

    assert!(first.starts_with('┌'), "top-left corner on first line");
    assert!(first.contains("─ S "), "title on top border");
    assert!(first.ends_with('┐'), "top-right corner on first line");
    assert!(last.starts_with('└'), "bottom-left corner on last line");
    assert!(last.ends_with('┘'), "bottom-right corner on last line");
}

#[test]
fn spec_subgraph_nodes_accessible() {
    let input = "graph TD\n    subgraph Backend\n        A[API] --> B[DB]\n    end\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ API │"), "node A rendered inside subgraph");
    assert!(output.contains("│ DB │"), "node B rendered inside subgraph");
    assert!(output.contains('▼'), "edge arrow rendered");
}

#[test]
fn spec_subgraph_cross_boundary_edge() {
    let input = "\
graph TD
    C[Client]
    subgraph Backend
        A[API] --> B[DB]
    end
    C --> A
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ Client │"), "external node rendered");
    assert!(output.contains("│ API │"), "subgraph node A rendered");
    assert!(output.contains("│ DB │"), "subgraph node B rendered");
    assert!(output.contains("┌─ Backend"), "subgraph title intact");

    let title_line = output.lines().find(|l| l.contains("Backend")).unwrap();
    assert!(
        !title_line.contains('▼'),
        "arrow should not overwrite subgraph title"
    );
}

// =============================================================================
// Dispatch — graph input does not break sequence diagrams
// =============================================================================

#[test]
fn spec_sequence_diagram_still_works() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("Alice"), "sequence diagram renders");
    assert!(output.contains("Bob"));
    assert!(output.contains("Hello"));
}
