use ma::render;

const CASE_A: &str = r#"graph TD
    Start --> Decision{Choose}
    subgraph Processing
        Process1[Step 1]
        Process2[Step 2]
        Process3[Step 3]
    end
    Decision -->|Yes| Process1
    Decision -->|No| Process2
    Decision -->|Maybe| Process3
    Process1 --> Merge
    Process2 --> Merge
    Process3 --> Merge
    Merge --> End
"#;

const CASE_B: &str = r#"graph TD
    subgraph Process
        A --> B
    end
    B --> ErrorHandler
"#;

fn row_with(lines: &[&str], needle: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("expected {needle:?} in render:\n{}", lines.join("\n")))
}

fn char_pos(line: &str, needle: &str) -> usize {
    let byte = line
        .find(needle)
        .unwrap_or_else(|| panic!("expected {needle:?} in {line:?}"));
    line[..byte].chars().count()
}

fn is_vertical_connector(ch: char) -> bool {
    matches!(
        ch,
        '│' | '┊' | '║' | '▼' | '┼' | '┬' | '┴' | '├' | '┤' | '┌' | '┐' | '└' | '┘'
    )
}

fn column_has_vertical(line: &str, col: usize) -> bool {
    line.chars().nth(col).is_some_and(is_vertical_connector)
}

/// True when `col` has a vertical connector on every non-empty row from `start`
/// through `end` inclusive. Blank rows in that span fail the check.
fn column_connected(lines: &[&str], col: usize, start: usize, end: usize) -> bool {
    if start > end {
        return false;
    }
    (start..=end).all(|row| column_has_vertical(lines[row], col))
}

#[test]
fn case_a_start_decision_processing_merge_end_vertical_flow() {
    let output = render(CASE_A).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let start_row = row_with(&lines, "Start");
    let decision_row = row_with(&lines, "Choose");
    let processing_row = row_with(&lines, "Processing");
    let merge_row = row_with(&lines, "Merge");
    let end_row = row_with(&lines, "End");

    assert!(start_row < decision_row, "Start must render above Decision");
    assert!(
        decision_row < processing_row,
        "Decision must render above Processing subgraph"
    );
    assert!(
        processing_row < merge_row,
        "Processing subgraph must render above Merge"
    );
    assert!(merge_row < end_row, "Merge must render above End");

    let title_line = lines[processing_row];
    assert!(
        title_line.contains("Processing"),
        "subgraph title must be intact on one row, got {title_line:?}"
    );
    let title_at = title_line
        .find("Processing")
        .expect("Processing on title row");
    let title_end = title_at + "Processing".len();
    let title_span = &title_line[title_at..title_end];
    assert_eq!(
        title_span, "Processing",
        "entry edges must not cut the subgraph title: {title_line:?}"
    );

    assert!(output.contains("Yes"), "edge label Yes present");
    assert!(output.contains("No"), "edge label No present");
    assert!(output.contains("Maybe"), "edge label Maybe present");

    for label in ["Yes", "No", "Maybe"] {
        let label_row = row_with(&lines, label);
        assert!(
            decision_row < label_row && label_row < merge_row,
            "{label} must sit on the Decision-to-step edge, row {label_row}"
        );
        assert!(
            !lines[label_row].contains("Processing"),
            "{label} must not share the subgraph title row"
        );
    }

    let step1_row = row_with(&lines, "Step 1");
    let bottom_row = ((processing_row + 1)..merge_row)
        .rev()
        .find(|&row| {
            let trimmed = lines[row].trim();
            trimmed.starts_with('└') && trimmed.ends_with('┘')
        })
        .expect("subgraph bottom border");
    assert!(
        step1_row < bottom_row && bottom_row < merge_row,
        "subgraph bottom must sit between the steps and Merge"
    );
    assert!(
        !lines[bottom_row].contains("Step"),
        "exit edges must not sit on the subgraph bottom border: {:?}",
        lines[bottom_row]
    );

    let merge_arrow_row = (bottom_row..merge_row)
        .rev()
        .find(|&row| lines[row].contains('▼'))
        .expect("arrow into Merge");
    let merge_col = char_pos(lines[merge_arrow_row], "▼");
    assert!(
        column_connected(&lines, merge_col, bottom_row, merge_arrow_row),
        "Merge edge must be a continuous vertical from the subgraph border to the arrow:\n{output}"
    );

    for (label, step) in [("Yes", "Step 1"), ("No", "Step 2"), ("Maybe", "Step 3")] {
        let label_row = row_with(&lines, label);
        let step_row = row_with(&lines, step);
        let col = char_pos(lines[label_row], label) + label.chars().count() / 2;
        assert!(
            (label_row..step_row).any(|row| column_has_vertical(lines[row], col)
                || lines[row].chars().nth(col) == Some('▼')),
            "{label} edge must connect to {step}:\n{output}"
        );
    }
}

#[test]
fn case_b_subgraph_to_external_node_forward_edge() {
    let output = render(CASE_B).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let b_row = row_with(&lines, "│ B │");
    let error_handler_row = row_with(&lines, "ErrorHandler");
    assert!(
        b_row < error_handler_row,
        "B must render above ErrorHandler, B row {b_row}, ErrorHandler row {error_handler_row}"
    );

    let blank_rows_between = (b_row + 1..error_handler_row)
        .filter(|&row| lines[row].trim().is_empty())
        .count();
    assert!(
        blank_rows_between == 0,
        "no stray blank rows between B and ErrorHandler, found {blank_rows_between}:\n{output}"
    );

    let arrow_row = (b_row..error_handler_row)
        .rev()
        .find(|&row| lines[row].contains('▼'))
        .expect("arrow into ErrorHandler");
    let col = char_pos(lines[arrow_row], "▼");
    let bottom_row = (b_row..error_handler_row)
        .rev()
        .find(|&row| lines[row].trim_start().starts_with('└'))
        .expect("subgraph bottom border");
    assert!(
        column_connected(&lines, col, bottom_row, arrow_row),
        "B to ErrorHandler must be a continuous vertical, no gutter detour:\n{output}"
    );
}
