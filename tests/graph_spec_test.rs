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
    let expected = "  ─────\n ╱     ╲\n│ Hello │\n ╲     ╱\n  ─────";
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
fn spec_additional_node_shapes_render_distinctly() {
    let stadium = ma::render("graph TD\n    A([Stadium])\n").unwrap();
    assert!(stadium.contains("( Stadium )"));

    let subroutine = ma::render("graph TD\n    A[[Subroutine]]\n").unwrap();
    assert!(subroutine.contains("│║Subroutine  ║│"));

    let cylinder = ma::render("graph TD\n    A[(Database)]\n").unwrap();
    let cylinder_lines: Vec<_> = cylinder.lines().collect();
    assert!(cylinder_lines[1].starts_with('╰'), "top ellipse is visible");
    assert!(cylinder_lines[2].contains("│ Database │"));

    let hexagon = ma::render("graph TD\n    A{{Hexagon}}\n").unwrap();
    assert!(hexagon.contains('╱') && hexagon.contains('╲'));
}

#[test]
fn spec_chained_fan_out_preserves_each_edge_label() {
    let input = "graph LR\n    A -->|first| B & C -->|second| D\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("first"), "{output}");
    assert!(!output.contains("firstt"), "{output}");
    assert_eq!(output.matches("first").count(), 1, "{output}");
    assert_eq!(output.matches("second").count(), 2, "{output}");
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

#[test]
fn spec_invisible_edge_between_subgraphs_constrains_layout_without_rendering() {
    let input = "\
flowchart TB
  subgraph left
    A
  end
  subgraph right
    B
  end
  left ~~~ right
";
    let output = ma::render(input).unwrap();
    let expected = "\
┌─ left ─┐
│ ┌───┐  │
│ │ A │  │
│ └───┘  │
└────────┘



┌─ right ─┐
│ ┌───┐   │
│ │ B │   │
│ └───┘   │
└─────────┘";
    assert_eq!(output, expected);
}

// =============================================================================
// Self-loop
// =============================================================================

#[test]
fn spec_fan_in_different_ranks() {
    // D(rank 1) and H(rank 2) both point to E — they are at different y positions.
    // The renderer must NOT draw a merge bar (which assumes same y) for this case.
    let input = "\
graph TD
    A --> B
    B --> D
    D --> E
    G --> H
    H --> E
";
    let output = ma::render(input).unwrap();
    // E should be rendered
    assert!(output.contains("│ E │"), "E rendered");
    // Each line should be well-formed: no broken merge bars where ┬ appears
    // without matching └ or ┘ on the same line
    for (i, line) in output.lines().enumerate() {
        // A merge bar has └ and ┘ on the same line — that's fine.
        // But a lone ┬─┘ without └ on the same line indicates a broken merge bar.
        if line.contains('┘')
            && !line.contains('└')
            && !line.contains('┐')
            && !line.contains('┌')
            && !line.contains('├')
            && !line.contains('▼')
        {
            // This line has a dangling ┘ without a proper left end — broken merge bar
            panic!("broken merge bar at line {i}: {line}\nfull output:\n{output}");
        }
    }
}

#[test]
fn spec_fan_in_different_ranks_labeled_edge_connects() {
    // When a labeled edge crosses multiple ranks (fan-in from different ranks),
    // both labels should be rendered and the target should have an arrow.
    // Note: arrows from different parents may overlap at the same ▼ position.
    let input = "\
graph TD
    A -->|data| B
    B -->|lookup| C
    C -->|label1| E
    D -->|label2| E
";
    let output = ma::render(input).unwrap();
    // Both labels should appear
    assert!(output.contains("label1"), "label1 rendered");
    assert!(output.contains("label2"), "label2 rendered");
    // E should be rendered intact (no edges routing through it)
    assert!(output.contains("│ E │"), "E node rendered intact");
    // Intermediate nodes should not be corrupted by edge routing
    assert!(output.contains("│ B │"), "B node rendered intact");
    assert!(output.contains("│ C │"), "C node rendered intact");
}

#[test]
fn spec_self_loop_does_not_crash() {
    let input = "graph TD\n    A --> B\n    B -->|fallback| B\n    B --> C\n";
    let output = ma::render(input);
    assert!(output.is_ok(), "self-loop should not crash: {:?}", output.err());
    let rendered = output.unwrap();
    assert!(rendered.contains("│ A │"), "A rendered");
    // B's right border becomes ├ due to self-loop, so check for │ B ├
    assert!(rendered.contains("│ B ├"), "B rendered with self-loop");
    assert!(rendered.contains("│ C │"), "C rendered");
}

#[test]
fn spec_self_loop_td_visual() {
    let input = "graph TD\n    A -->|retry| A\n";
    let output = ma::render(input).unwrap();
    // Self-loop should show the loop arm to the right of the node
    assert!(output.contains("├"), "self-loop branches from right border");
    assert!(output.contains("◄"), "self-loop returns with ◄");
    assert!(output.contains("retry"), "self-loop label rendered");
}

#[test]
fn spec_self_loop_lr_visual() {
    let input = "graph LR\n    A -->|retry| A\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("├"), "self-loop branches from right border");
    assert!(output.contains("◄"), "self-loop returns with ◄");
    assert!(output.contains("retry"), "self-loop label rendered");
}

// =============================================================================
// Cycles — back edges
// =============================================================================

#[test]
fn spec_cycle_lr_diamond_terminates() {
    let input = "graph LR\n    A --> B\n    A --> C\n    B --> D\n    C --> D\n    D --> A\n";
    let output = ma::render(input).unwrap();
    for id in ["A", "B", "C", "D"] {
        assert!(output.contains(&format!("│ {id} ")), "{id} rendered");
    }
}

#[test]
fn spec_cycle_lr_two_nodes_ranks_forward() {
    let input = "graph LR\n    A --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    let first = output.lines().next().unwrap();
    assert!(!first.trim().is_empty(), "first rank is not empty: {output:?}");
    let a = output.find("│ A ").unwrap();
    let b = output.find("│ B ").unwrap();
    assert!(a < b, "A left of B, back edge B --> A goes backwards:\n{output}");
}

#[test]
fn spec_cycle_td_two_nodes_ranks_forward() {
    let input = "graph TD\n    A --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    let first = output.lines().next().unwrap();
    assert!(!first.trim().is_empty(), "first rank is not empty: {output:?}");
    let a = output.find("│ A ").unwrap();
    let b = output.find("│ B ").unwrap();
    assert!(a < b, "A above B, back edge B --> A goes backwards:\n{output}");
}

#[test]
fn spec_cycle_three_nodes_keeps_entry_node_first() {
    for input in [
        "graph LR\n    A --> B\n    B --> C\n    C --> A\n",
        "graph TD\n    A --> B\n    B --> C\n    C --> A\n",
    ] {
        let output = ma::render(input).unwrap();
        let a = output.find("│ A ").unwrap();
        let b = output.find("│ B ").unwrap();
        let c = output.find("│ C ").unwrap();
        assert!(a < b && b < c, "A, B, C in declaration order:\n{output}");
    }
}

#[test]
fn spec_cycle_lr_back_edge_visual() {
    let input = "graph LR\n    A --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    let expected = concat!(
        "┌───┐     ┌───┐\n",
        "│ A │────>│ B │\n",
        "└──┬┘     └─┬─┘\n",
        "   ▲─┐      └──┐\n",
        "     └─────────┘",
    );
    assert_eq!(output, expected);
}

#[test]
fn spec_cycle_td_back_edge_visual() {
    let input = "graph TD\n    A --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    let expected = concat!(
        "┌───┐\n",
        "│ A │\n",
        "└─┬┬┘\n",
        "  │▲──┐\n",
        "  ▼   │\n",
        "┌───┐ │\n",
        "│ B │ │\n",
        "└─┬─┘ │\n",
        "  └───┘",
    );
    assert_eq!(output, expected);
}

#[test]
fn spec_cycle_back_edge_label_rendered() {
    let lr = ma::render("graph LR\n    A --> B\n    B -->|retry| A\n").unwrap();
    assert!(lr.contains("retry"), "LR back edge label rendered:\n{lr}");
    let td = ma::render("graph TD\n    A --> B\n    B -->|retry| A\n").unwrap();
    assert!(td.contains("retry"), "TD back edge label rendered:\n{td}");
}

#[test]
fn spec_cycle_back_edge_does_not_cross_nodes() {
    let input = "graph LR\n    A --> B\n    B --> C\n    C --> A\n    C --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ B │"), "B rendered intact:\n{output}");
    assert!(output.contains("│ C │"), "C rendered intact:\n{output}");
    assert_eq!(output.matches('▲').count(), 2, "both back edges arrive");
}

#[test]
fn spec_cycle_back_edge_does_not_overwrite_sibling_nodes() {
    // The back edge B --> A must reach A without crossing C, which shares A's rank
    let input = "graph TD\n    A --> B\n    B --> A\n    C --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "A rendered intact:\n{output}");
    assert!(output.contains("│ C │"), "C rendered intact:\n{output}");
    assert!(output.contains('▲'), "back edge arrives:\n{output}");
}

#[test]
fn spec_cycle_back_edge_excluded_from_fan_out_bar() {
    // C has one forward child (D) and one back edge (A); the fan-out bar is for
    // forward children only, so a single forward child means no bar
    let input = "graph TD\n    A --> B\n    B --> C\n    C --> D\n    C --> A\n";
    let output = ma::render(input).unwrap();
    for id in ["A", "B", "C", "D"] {
        assert!(output.contains(&format!("│ {id} │")), "{id} intact:\n{output}");
    }
    assert!(output.contains('▲'), "back edge arrives:\n{output}");
}

#[test]
fn spec_cycle_back_edge_long_label_keeps_route_intact() {
    let input = "graph LR\n    A --> B\n    B -->|averyverylonglabel| A\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("averyverylonglabel"), "label not truncated:\n{output}");
    let route = output.lines().last().unwrap();
    assert!(route.contains('└') && route.contains('┘'), "route corners intact:\n{output}");
}

#[test]
fn spec_cycle_inside_subgraph_keeps_frame() {
    for input in [
        "graph TD\n    subgraph S\n        A --> B\n        B --> A\n    end\n",
        "graph LR\n    subgraph S\n        A --> B\n        B --> A\n    end\n",
    ] {
        let output = ma::render(input).unwrap();
        assert!(output.contains("┌─ S "), "subgraph title intact:\n{output}");
        assert!(output.contains("│ A │"), "A intact:\n{output}");
        assert!(output.contains("│ B │"), "B intact:\n{output}");
        assert!(output.contains('▲'), "back edge arrives:\n{output}");
    }
}

#[test]
fn spec_cycle_back_edge_across_subgraphs_keeps_frames() {
    let input = "graph LR\n    subgraph One\n        A --> B\n    end\n    subgraph Two\n        C --> D\n    end\n    B --> C\n    D --> A\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("┌─ One "), "One title intact:\n{output}");
    assert!(output.contains("┌─ Two "), "Two title intact:\n{output}");
    for id in ["A", "B", "C", "D"] {
        assert!(output.contains(&format!("│ {id} │")), "{id} intact:\n{output}");
    }
}

#[test]
fn spec_cycle_back_edge_types() {
    let dotted = ma::render("graph LR\n    A --> B\n    B -.-> A\n").unwrap();
    assert!(dotted.contains('╌') || dotted.contains('┊'), "dotted glyphs:\n{dotted}");
    let thick = ma::render("graph LR\n    A --> B\n    B ==> A\n").unwrap();
    assert!(thick.contains('═') || thick.contains('║'), "thick glyphs:\n{thick}");
    let open = ma::render("graph LR\n    A --> B\n    B --- A\n").unwrap();
    assert!(!open.contains('▲'), "open link has no arrow head:\n{open}");
}

#[test]
fn spec_cycle_respects_max_width() {
    let input = "graph LR\n    A --> B\n    A --> C\n    B --> D\n    C --> D\n    D --> A\n";
    for max in [80, 30, 20] {
        let output = ma::render_with_options(input, Some(max)).unwrap();
        let widest = output.lines().map(ma::display_width::display_width).max().unwrap_or(0);
        assert!(widest <= max, "output fits {max} columns, got {widest}:\n{output}");
        assert!(output.contains('▲'), "back edge still drawn at {max}:\n{output}");
    }
}

#[test]
fn spec_left_right_reflows_when_max_width_requires_it() {
    let input = "flowchart LR\n    A -->|abcdefghij| B\n    B --> C\n";
    let output = ma::render_with_options(input, Some(27)).unwrap();
    let widest = output
        .lines()
        .map(ma::display_width::display_width)
        .max()
        .unwrap_or(0);

    assert!(
        widest <= 27,
        "output fits 27 columns, got {widest}:\n{output}"
    );
    assert!(
        output.contains('▼'),
        "narrow LR graph should reflow vertically:\n{output}"
    );
}

#[test]
fn spec_subgraphs_stack_when_max_width_requires_it() {
    let input = "graph LR\n    subgraph One\n        A --> B\n    end\n    subgraph Two\n        C --> D\n    end\n    B --> C\n";
    let output = ma::render_with_options(input, Some(19)).unwrap();
    let widest = output
        .lines()
        .map(ma::display_width::display_width)
        .max()
        .unwrap_or(0);
    let one_row = output
        .lines()
        .position(|line| line.contains("One"))
        .unwrap();
    let two_row = output
        .lines()
        .position(|line| line.contains("Two"))
        .unwrap();

    assert!(
        widest <= 19,
        "output fits 19 columns, got {widest}:\n{output}"
    );
    assert!(
        two_row > one_row,
        "subgraphs should stack vertically:\n{output}"
    );
    assert!(
        output.contains('▼'),
        "cross-subgraph edge should be routed vertically:\n{output}"
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

// =============================================================================
// Multi-target edges (A --> B & C)
// =============================================================================

#[test]
fn spec_multi_target_same_as_separate() {
    let separate = ma::render("graph TD\n    A --> B\n    A --> C\n").unwrap();
    let multi = ma::render("graph TD\n    A --> B & C\n").unwrap();
    assert_eq!(separate, multi, "A --> B & C same as two separate edges");
}

#[test]
fn spec_multi_target_three() {
    let separate = ma::render("graph TD\n    A --> B\n    A --> C\n    A --> D\n").unwrap();
    let multi = ma::render("graph TD\n    A --> B & C & D\n").unwrap();
    assert_eq!(separate, multi, "A --> B & C & D expands to three edges");
}

#[test]
fn spec_multi_target_lr() {
    let separate = ma::render("graph LR\n    A --> B\n    A --> C\n").unwrap();
    let multi = ma::render("graph LR\n    A --> B & C\n").unwrap();
    assert_eq!(separate, multi, "multi-target works in LR layout");
}

#[test]
fn spec_multi_target_with_label() {
    let input = "graph TD\n    A -->|yes| B & C\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ B │"), "B rendered");
    assert!(output.contains("│ C │"), "C rendered");
}

// =============================================================================
// Cross-rank fan-in: gutter column routing
// =============================================================================

#[test]
fn spec_cross_rank_fan_in_gutter_routing() {
    // B(rank 1) and F(rank 4) both → G — different ranks.
    // B's center_x overlaps with wide intermediate node E.
    // Routing should go via gutter column (right of all intermediate nodes).
    let input = "\
graph TD
    A -->|data| B
    B -->|label_b| G
    C --> D
    D --> E[WideNodeName123456]
    E --> F
    F -->|label_f| G
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("label_b"), "label_b rendered");
    assert!(output.contains("│ G │"), "G node intact");
    // Gutter routing: ┐ at route_start row, ┘ at to_above row
    let lines: Vec<&str> = output.lines().collect();
    let has_top_corner = lines.iter().any(|l| l.contains('┐') && !l.contains("┌─"));
    let has_bottom_corner = lines.iter().any(|l| l.contains('┘') && !l.contains("└─"));
    assert!(has_top_corner, "gutter top corner ┐ exists:\n{output}");
    assert!(has_bottom_corner, "gutter bottom corner ┘ exists:\n{output}");
}

// =============================================================================
// Style directives (ignored)
// =============================================================================

#[test]
fn spec_style_directive_ignored() {
    let input = "\
graph TD
    A --> B
    style A fill:#f9f,stroke:#333
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "A rendered");
    assert!(output.contains("│ B │"), "B rendered");
    assert!(!output.contains("style"), "style directive not rendered as node");
    assert!(!output.contains("fill"), "style properties not rendered");
}

#[test]
fn spec_style_directive_with_stroke_dasharray() {
    let input = "\
graph LR
    A --> B
    style B stroke-dasharray: 5 5,stroke:#f66
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "A rendered");
    assert!(output.contains("│ B │"), "B rendered");
    assert!(!output.contains("stroke"), "stroke not rendered as node");
}

#[test]
fn spec_multiple_style_directives() {
    let input = "\
graph TD
    A --> B
    B --> C
    style A fill:#f9f
    style B fill:#bbf
    style C fill:#bfb
";
    let output = ma::render(input).unwrap();
    assert_eq!(output.matches("│ A │").count(), 1, "exactly one A");
    assert_eq!(output.matches("│ B │").count(), 1, "exactly one B");
    assert_eq!(output.matches("│ C │").count(), 1, "exactly one C");
    assert!(!output.contains("style"), "no style nodes");
}

#[test]
fn spec_classdef_directive_ignored() {
    let input = "\
graph TD
    A --> B
    classDef highlight fill:#f9f,stroke:#333
    class A highlight
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "A rendered");
    assert!(output.contains("│ B │"), "B rendered");
    assert!(!output.contains("classDef"), "classDef not rendered as node");
    assert!(!output.contains("class"), "class not rendered as node");
    assert!(!output.contains("highlight"), "class name not rendered");
}

#[test]
fn spec_linkstyle_directive_ignored() {
    let input = "\
graph LR
    A --> B
    linkStyle 0 stroke:#f66
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "A rendered");
    assert!(output.contains("│ B │"), "B rendered");
    assert!(!output.contains("linkStyle"), "linkStyle not rendered as node");
}

// =============================================================================
// Cycles — back edge routing under stress
// =============================================================================

#[test]
fn spec_cycle_lr_back_edge_arrow_survives_a_second_back_edge_leaving_the_target() {
    // B receives the back edge C --> B and is the source of the back edge
    // B --> A. Both routes touch B's bottom centre, so the outgoing route must
    // not take the cell the incoming arrow head sits in.
    let input = "graph LR\n    A --> B\n    B --> C\n    C --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    assert_eq!(
        output.matches('▲').count(),
        2,
        "both back edges keep their arrow head:\n{output}"
    );
}

#[test]
fn spec_cycle_td_back_edge_arrow_survives_a_self_loop_on_the_target() {
    // A is the target of the back edge B --> A and carries a self-loop. The
    // self-loop is drawn last and must not take the back edge's arrow head.
    let input = "graph TD\n    A --> B\n    B --> A\n    A --> A\n";
    let output = ma::render(input).unwrap();
    assert_eq!(
        output.matches('▲').count(),
        1,
        "back edge keeps its arrow head next to the self-loop:\n{output}"
    );
    assert_eq!(
        output.matches('◄').count(),
        1,
        "self-loop keeps its own arrow head:\n{output}"
    );
}

#[test]
fn spec_cycle_td_back_edge_clears_a_taller_node_in_the_target_rank() {
    // A shares its rank with a diamond, which is two rows taller. The back edge
    // B --> A enters through the gap below the whole rank, so the diamond's
    // body has to stay untouched.
    let input = "graph TD\n    S --> A\n    S --> D{Decide}\n    A --> B\n    B --> A\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ Decide │"), "diamond body intact:\n{output}");
    assert!(output.contains(" ╲      ╱"), "diamond lower slant intact:\n{output}");
    assert!(output.contains("│ A │"), "A intact:\n{output}");
    assert_eq!(output.matches('▲').count(), 1, "back edge arrives:\n{output}");
}

#[test]
fn spec_cycle_disconnected_components_each_draw_their_own_back_edge() {
    let input = "graph TD\n    A --> B\n    B --> A\n    C --> D\n    D --> C\n";
    let output = ma::render(input).unwrap();
    for id in ["A", "B", "C", "D"] {
        assert!(output.contains(&format!("│ {id} │")), "{id} intact:\n{output}");
    }
    assert_eq!(output.matches('▲').count(), 2, "both cycles close:\n{output}");
}

#[test]
fn spec_cycle_duplicate_edges_keep_ranks_forward() {
    let input = "graph LR\n    A --> B\n    A --> B\n    B --> A\n    B --> A\n";
    let output = ma::render(input).unwrap();
    let a = output.find("│ A │").unwrap();
    let b = output.find("│ B │").unwrap();
    assert!(a < b, "parallel edges do not push B ahead of A:\n{output}");
    assert!(output.contains('▲'), "back edge drawn:\n{output}");
}

#[test]
fn spec_cycle_lr_back_edges_from_the_last_rank_all_arrive() {
    // Three back edges leave the same node, so they share one rank gap and the
    // lanes have to wrap without eating each other's arrow heads.
    let input =
        "graph LR\n    A --> B\n    B --> C\n    C --> D\n    D --> A\n    D --> B\n    D --> C\n";
    let output = ma::render(input).unwrap();
    for id in ["A", "B", "C", "D"] {
        assert!(output.contains(&format!("│ {id} │")), "{id} intact:\n{output}");
    }
    assert_eq!(output.matches('▲').count(), 3, "three back edges arrive:\n{output}");
}

#[test]
fn spec_cycle_long_chain_renders_every_node() {
    let mut input = String::from("graph TD\n");
    for i in 1..=60 {
        input.push_str(&format!("    N{i} --> N{}\n", i + 1));
    }
    input.push_str("    N61 --> N1\n");
    let output = ma::render(&input).unwrap();
    for i in 1..=61 {
        assert!(output.contains(&format!("│ N{i} │")), "N{i} rendered");
    }
    assert_eq!(output.matches('▲').count(), 1, "the back edge closes the chain");
}

#[test]
fn spec_fan_in_diamond_unchanged_by_back_edge_filtering() {
    // The fan-out and fan-in bars now count forward children and parents only.
    // With no back edge present the drawing must be exactly what it was.
    let input = "graph TD\n    A --> B\n    A --> C\n    B --> D\n    C --> D\n";
    let output = ma::render(input).unwrap();
    let expected = concat!(
        "    ┌───┐\n",
        "    │ A │\n",
        "    └─┬─┘\n",
        "  ┌───┴───┐\n",
        "  ▼       ▼\n",
        "┌───┐   ┌───┐\n",
        "│ B │   │ C │\n",
        "└─┬─┘   └─┬─┘\n",
        "  └───┬───┘\n",
        "      ▼\n",
        "    ┌───┐\n",
        "    │ D │\n",
        "    └───┘",
    );
    assert_eq!(output, expected);
}
