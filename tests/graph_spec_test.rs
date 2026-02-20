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
