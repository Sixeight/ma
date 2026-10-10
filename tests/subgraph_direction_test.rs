use ma::render;

fn lr_cluster() -> &'static str {
    "\
┌─ Cluster ───────┐
│ ┌───┐     ┌───┐ │
│ │ A │────>│ B │ │
│ └───┘     └───┘ │
└─────────────────┘"
}

fn td_cluster() -> &'static str {
    "\
┌─ Cluster ─┐
│   ┌───┐   │
│   │ A │   │
│   └─┬─┘   │
│     │     │
│     ▼     │
│   ┌───┐   │
│   │ B │   │
│   └───┘   │
└───────────┘"
}

fn lr_cluster_three() -> &'static str {
    "\
┌─ Cluster ─────────────────┐
│ ┌───┐     ┌───┐     ┌───┐ │
│ │ A │────>│ B │────>│ C │ │
│ └───┘     └───┘     └───┘ │
└───────────────────────────┘"
}

fn nested_lr_inner() -> &'static str {
    "\
┌─ Outer ─────────────┐
│ ┌─ Inner ─────────┐ │
│ │ ┌───┐     ┌───┐ │ │
│ │ │ A │────>│ B │ │ │
│ │ └───┘     └───┘ │ │
│ └─────────────────┘ │
└─────────────────────┘"
}

fn row_with(lines: &[&str], needle: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("expected {needle:?} in render:\n{}", lines.join("\n")))
}

fn col_with(line: &str, needle: &str) -> usize {
    line.find(needle)
        .unwrap_or_else(|| panic!("expected {needle:?} in {line:?}"))
}

#[test]
fn isolated_lr_subgraph_inside_td_flowchart() {
    let output = render(
        "flowchart TD\n    subgraph Cluster\n        direction LR\n        A --> B\n    end\n",
    )
    .unwrap();
    assert_eq!(output, lr_cluster());
}

#[test]
fn isolated_lr_chain_inside_td_flowchart() {
    let output = render(
        "flowchart TD\n    subgraph Cluster\n        direction LR\n        A --> B --> C\n    end\n",
    )
    .unwrap();
    assert_eq!(output, lr_cluster_three());
}

#[test]
fn isolated_tb_subgraph_inside_lr_flowchart() {
    let output = render(
        "flowchart LR\n    subgraph Cluster\n        direction TB\n        A --> B\n    end\n",
    )
    .unwrap();
    assert_eq!(output, td_cluster());
}

#[test]
fn nested_inner_honors_lr_inside_td_parent() {
    let output = render(
        "flowchart TD\n    subgraph Outer[Outer]\n        subgraph Inner[Inner]\n            direction LR\n            A --> B\n        end\n    end\n",
    )
    .unwrap();
    assert_eq!(output, nested_lr_inner());
}

#[test]
fn node_link_outside_ignores_subgraph_direction_td() {
    let with_dir = render(
        "flowchart TD\n    subgraph Cluster\n        direction LR\n        A --> B\n    end\n    Outside --> A\n",
    )
    .unwrap();
    let without_dir =
        render("flowchart TD\n    subgraph Cluster\n        A --> B\n    end\n    Outside --> A\n")
            .unwrap();
    assert_eq!(with_dir, without_dir);
    let lines: Vec<&str> = with_dir.lines().collect();
    assert!(
        row_with(&lines, "│ A │") < row_with(&lines, "│ B │"),
        "ignored LR must stay TD:\n{with_dir}"
    );
}

#[test]
fn node_link_outside_ignores_subgraph_direction_lr() {
    let with_dir = render(
        "flowchart LR\n    subgraph Cluster[Cluster]\n        direction TB\n        A --> B\n    end\n    outside --> A\n",
    )
    .unwrap();
    let without_dir = render(
        "flowchart LR\n    subgraph Cluster[Cluster]\n        A --> B\n    end\n    outside --> A\n",
    )
    .unwrap();
    assert_eq!(with_dir, without_dir);
}

#[test]
fn link_to_subgraph_id_keeps_lr_inside_td() {
    let output = render(
        "flowchart TD\n    Outside --> Cluster\n    subgraph Cluster[Cluster]\n        direction LR\n        A --> B\n    end\n",
    )
    .unwrap();
    assert_eq!(
        output,
        [
            "    ┌─────────┐",
            "    │ Outside │────┐",
            "    └─────────┘    │",
            "         ┌─────────┘",
            "         ▼",
            "┌─ Cluster ───────┐",
            "│ ┌───┐     ┌───┐ │",
            "│ │ A │────>│ B │ │",
            "│ └───┘     └───┘ │",
            "└─────────────────┘",
        ]
        .join("\n")
    );
}

#[test]
fn mermaid_doc_subgraph_direction_limitation() {
    let output = render(
        "flowchart LR\n    subgraph subgraph1\n        direction TB\n        top1[top] --> bottom1[bottom]\n    end\n    subgraph subgraph2\n        direction TB\n        top2[top] --> bottom2[bottom]\n    end\n    outside --> subgraph1\n    outside ---> top2\n",
    )
    .unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let top1_row = row_with(&lines, "│ top │");
    let bottom1_row = row_with(&lines, "│ bottom │");
    assert!(
        top1_row < bottom1_row,
        "subgraph1 keeps TB because the outside link targets the subgraph id:\n{output}"
    );

    let sg2_title = row_with(&lines, "subgraph2");
    let top2_row = lines
        .iter()
        .enumerate()
        .find(|(i, line)| *i > sg2_title && line.contains("│ top │"))
        .map(|(i, _)| i)
        .expect("top2");
    let bottom2_row = lines
        .iter()
        .enumerate()
        .find(|(i, line)| *i > sg2_title && line.contains("│ bottom │"))
        .map(|(i, _)| i)
        .expect("bottom2");
    assert!(
        col_with(lines[top2_row], "│ top │") < col_with(lines[bottom2_row], "│ bottom │"),
        "subgraph2 inherits LR because outside links to top2:\n{output}"
    );
}
