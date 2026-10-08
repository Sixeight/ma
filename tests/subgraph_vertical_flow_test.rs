use ma::render;

#[test]
fn case_a_start_decision_processing_merge_end_vertical_flow() {
    let input = r#"graph TD
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
    let output = render(input).unwrap();

    let lines: Vec<&str> = output.lines().collect();

    let start_row = lines
        .iter()
        .position(|line| line.contains("Start"))
        .unwrap();
    let decision_row = lines
        .iter()
        .position(|line| line.contains("Choose"))
        .unwrap();
    let processing_row = lines
        .iter()
        .position(|line| line.contains("Proc") || line.contains("ssing"))
        .unwrap();
    let merge_row = lines
        .iter()
        .position(|line| line.contains("Merge"))
        .unwrap();
    let end_row = lines.iter().position(|line| line.contains("End")).unwrap();

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

    assert!(output.contains("Yes"), "edge label Yes present");
    assert!(output.contains("No"), "edge label No present");
    assert!(output.contains("Maybe"), "edge label Maybe present");

    let step1_row = lines
        .iter()
        .position(|line| line.contains("Step 1"))
        .unwrap();
    let step2_row = lines
        .iter()
        .position(|line| line.contains("Step 2"))
        .unwrap();
    let step3_row = lines
        .iter()
        .position(|line| line.contains("Step 3"))
        .unwrap();

    let edge_chars = vec!['│', '▼', '├', '┤', '┬', '┴', '└', '┌', '┐', '╱', '╲'];
    let edge_rows: Vec<usize> = ((decision_row + 1)..step1_row.min(step2_row).min(step3_row))
        .filter(|&row_idx| {
            let line = lines[row_idx];
            !line.contains("Yes")
                && !line.contains("No")
                && !line.contains("Maybe")
                && line.chars().any(|c| edge_chars.contains(&c))
        })
        .collect();

    assert!(
        !edge_rows.is_empty(),
        "edges from Decision to Processing steps must be drawn"
    );

    for (step_name, step_row) in [
        ("Step 1", step1_row),
        ("Step 2", step2_row),
        ("Step 3", step3_row),
    ] {
        let has_edge_to_merge = (step_row..merge_row).any(|row_idx| {
            let line = lines[row_idx];
            line.chars()
                .any(|c| c == '│' || c == '▼' || c == '└' || c == '┴')
        });
        assert!(
            has_edge_to_merge,
            "{} must have edge drawn to Merge",
            step_name
        );
    }
}

#[test]
fn case_b_subgraph_to_external_node_forward_edge() {
    let input = r#"graph TD
    subgraph Process
        A --> B
    end
    B --> ErrorHandler
"#;
    let output = render(input).unwrap();

    let lines: Vec<&str> = output.lines().collect();

    let b_row = lines
        .iter()
        .position(|line| line.contains("│ B │"))
        .unwrap();
    let error_handler_row = lines
        .iter()
        .position(|line| line.contains("ErrorHandler"))
        .unwrap();

    assert!(
        b_row < error_handler_row,
        "B must render above ErrorHandler for forward edge, B row {}, ErrorHandler row {}",
        b_row,
        error_handler_row
    );

    let blank_rows_between = (b_row + 1..error_handler_row)
        .filter(|&row_idx| {
            let line = lines[row_idx];
            line.trim().is_empty() || line.chars().all(|c| c.is_whitespace())
        })
        .count();

    assert!(
        blank_rows_between <= 2,
        "no large gutter between B and ErrorHandler, found {} blank rows",
        blank_rows_between
    );

    let has_edge = (b_row..error_handler_row).any(|row_idx| {
        let line = lines[row_idx];
        line.chars()
            .any(|c| c == '│' || c == '▼' || c == '└' || c == '├')
    });
    assert!(has_edge, "edge from B to ErrorHandler must be drawn");
}
