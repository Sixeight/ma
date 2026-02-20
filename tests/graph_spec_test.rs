use pretty_assertions::assert_eq;

#[test]
fn graph_td_linear_chain() {
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
fn graph_lr_linear_chain() {
    let input = "graph LR\n    A[Start] --> B[End]\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───────┐     ┌─────┐
│ Start │────>│ End │
└───────┘     └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn graph_td_fan_out() {
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
fn graph_td_fan_in() {
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
fn graph_td_open_link() {
    let input = "graph TD\n    A --- B\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐
│ A │
└─┬─┘
  │
  │
┌───┐
│ B │
└───┘";
    assert_eq!(output, expected);
}

#[test]
fn graph_lr_open_link() {
    let input = "graph LR\n    A --- B\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───┐     ┌───┐
│ A │─────│ B │
└───┘     └───┘";
    assert_eq!(output, expected);
}

#[test]
fn graph_flowchart_keyword() {
    let input = "flowchart TD\n    A[Start] --> B[End]\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("Start"));
    assert!(output.contains("End"));
    assert!(output.contains("▼"));
}

#[test]
fn graph_node_labels() {
    let input = "graph TD\n    A[Hello World] --> B[Goodbye]\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("Hello World"));
    assert!(output.contains("Goodbye"));
}

#[test]
fn graph_three_node_chain() {
    let input = "graph TD\n    A --> B\n    B --> C\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"));
    assert!(output.contains("│ B │"));
    assert!(output.contains("│ C │"));
    let a_pos = output.find("│ A │").unwrap();
    let b_pos = output.find("│ B │").unwrap();
    let c_pos = output.find("│ C │").unwrap();
    assert!(a_pos < b_pos, "A before B");
    assert!(b_pos < c_pos, "B before C");
}

#[test]
fn graph_implicit_labels() {
    let input = "graph TD\n    A --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("│ A │"), "implicit label A");
    assert!(output.contains("│ B │"), "implicit label B");
}
