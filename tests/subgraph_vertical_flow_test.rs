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

const CASE_INTERLEAVE: &str = r#"flowchart TD
    subgraph G[Group]
        A[InA]
        B[InB]
    end
    A -->|go| X[OutX]
    X -->|back| B
"#;

const CASE_SHARE_ROW: &str = r#"flowchart TD
    subgraph G[Group]
        A[InA] --> B[InB]
    end
    A -->|side| X[OutX]
"#;

fn subgraph_frame(lines: &[&str], title: &str) -> (usize, usize, usize, usize) {
    let title_row = row_with(lines, title);
    let title_line = lines[title_row];
    let title_chars: Vec<char> = title_line.chars().collect();
    let left = title_chars
        .iter()
        .position(|ch| *ch == '┌')
        .unwrap_or_else(|| panic!("subgraph left border on title row:\n{}", lines.join("\n")));
    let right = title_chars
        .iter()
        .rposition(|ch| *ch == '┐')
        .unwrap_or_else(|| panic!("subgraph right border on title row:\n{}", lines.join("\n")));
    let bottom_row = ((title_row + 1)..lines.len())
        .find(|&row| {
            let chars: Vec<char> = lines[row].chars().collect();
            chars.get(left) == Some(&'└') && chars.get(right) == Some(&'┘')
        })
        .unwrap_or_else(|| {
            panic!(
                "subgraph bottom border under {title}:\n{}",
                lines.join("\n")
            )
        });
    (title_row, bottom_row, left, right)
}

fn assert_title_intact(lines: &[&str], title: &str) {
    let title_row = row_with(lines, title);
    let title_line = lines[title_row];
    let title_at = title_line.find(title).expect("title text");
    assert_eq!(
        &title_line[title_at..title_at + title.len()],
        title,
        "title text must sit on one row and stay intact: {title_line:?}"
    );
    assert_eq!(
        lines.iter().filter(|line| line.contains(title)).count(),
        1,
        "title must appear on exactly one row:\n{}",
        lines.join("\n")
    );
}

fn node_col(lines: &[&str], needle: &str) -> usize {
    let row = row_with(lines, needle);
    char_pos(lines[row], needle)
}

fn assert_outside_frame(lines: &[&str], needle: &str, left: usize, right: usize) {
    let col = node_col(lines, needle);
    assert!(
        col + needle.chars().count() <= left || col >= right,
        "{needle} must sit outside the subgraph border [{left},{right}]:\n{}",
        lines.join("\n")
    );
}

fn assert_inside_frame(lines: &[&str], needle: &str, left: usize, right: usize) {
    let col = node_col(lines, needle);
    assert!(
        col > left && col + needle.chars().count() <= right,
        "{needle} must sit inside the subgraph border [{left},{right}]:\n{}",
        lines.join("\n")
    );
}

fn is_line_glyph(ch: char) -> bool {
    matches!(
        ch,
        '│' | '┊'
            | '║'
            | '─'
            | '╌'
            | '▼'
            | '▲'
            | '┼'
            | '┬'
            | '┴'
            | '├'
            | '┤'
            | '┌'
            | '┐'
            | '└'
            | '┘'
            | '╭'
            | '╮'
            | '╯'
            | '╰'
    )
}

fn boxes_from_render(lines: &[&str]) -> Vec<(usize, usize, usize, usize)> {
    let mut boxes = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut col = 0;
        while col < chars.len() {
            if chars[col] == '┌'
                && let Some(right) = chars[col + 1..].iter().position(|&ch| ch == '┐')
            {
                let right = col + 1 + right;
                if let Some(bottom) = ((row + 1)..lines.len()).find(|&end| {
                    let end_chars: Vec<char> = lines[end].chars().collect();
                    end_chars.get(col) == Some(&'└') && end_chars.get(right) == Some(&'┘')
                }) {
                    boxes.push((row, bottom, col, right));
                    col = right + 1;
                    continue;
                }
            }
            col += 1;
        }
    }
    boxes
}

fn assert_no_box_overlap(output: &str) {
    let lines: Vec<&str> = output.lines().collect();
    let boxes = boxes_from_render(&lines);
    for (i, a) in boxes.iter().enumerate() {
        for b in boxes.iter().skip(i + 1) {
            let x_overlap = a.2 < b.3 && b.2 < a.3;
            let y_overlap = a.0 < b.1 && b.0 < a.1;
            let a_contains_b = a.0 <= b.0 && a.1 >= b.1 && a.2 <= b.2 && a.3 >= b.3;
            let b_contains_a = b.0 <= a.0 && b.1 >= a.1 && b.2 <= a.2 && b.3 >= a.3;
            assert!(
                !x_overlap || !y_overlap || a_contains_b || b_contains_a,
                "boxes overlap ({a:?} vs {b:?}):\n{output}"
            );
        }
    }
}

fn assert_label_on_edge(lines: &[&str], label: &str, above: &str, below: &str) {
    let label_row = row_with(lines, label);
    let above_row = row_with(lines, above);
    let below_row = row_with(lines, below);
    let (start, end) = if above_row < below_row {
        (above_row, below_row)
    } else {
        (below_row, above_row)
    };
    assert!(
        start < label_row && label_row < end,
        "{label} must sit on the {above}-{below} edge, row {label_row}:\n{}",
        lines.join("\n")
    );
    let col = char_pos(lines[label_row], label) + label.chars().count() / 2;
    let touches_line = ((start + 1)..end).any(|row| {
        lines[row]
            .chars()
            .nth(col)
            .is_some_and(|ch| is_line_glyph(ch) || ch.is_ascii_alphabetic())
    });
    assert!(
        touches_line,
        "{label} must sit on a drawn edge at column {col}:\n{}",
        lines.join("\n")
    );
}

#[test]
fn outer_node_ranks_between_subgraph_members() {
    let output = render(CASE_INTERLEAVE).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let a_row = row_with(&lines, "InA");
    let x_row = row_with(&lines, "OutX");
    let b_row = row_with(&lines, "InB");
    assert!(
        a_row < x_row && x_row < b_row,
        "OutX must rank between InA and InB, rows InA={a_row} OutX={x_row} InB={b_row}:\n{output}"
    );

    assert_title_intact(&lines, "Group");
    let (title_row, bottom_row, left, right) = subgraph_frame(&lines, "Group");
    assert!(title_row < a_row && a_row < bottom_row);
    assert!(title_row < b_row && b_row < bottom_row);
    assert_inside_frame(&lines, "InA", left, right);
    assert_inside_frame(&lines, "InB", left, right);
    assert_outside_frame(&lines, "OutX", left, right);
    assert!(
        title_row < x_row && x_row < bottom_row,
        "OutX must sit beside the subgraph, not above or below it:\n{output}"
    );

    assert_label_on_edge(&lines, "go", "InA", "OutX");
    assert_label_on_edge(&lines, "back", "OutX", "InB");
    assert_no_box_overlap(&output);
}

#[test]
fn outer_node_shares_a_row_with_an_inner_node() {
    let output = render(CASE_SHARE_ROW).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let b_row = row_with(&lines, "InB");
    let x_row = row_with(&lines, "OutX");
    assert_eq!(b_row, x_row, "OutX must share InB's row:\n{output}");

    assert_title_intact(&lines, "Group");
    let (_title_row, _bottom_row, left, right) = subgraph_frame(&lines, "Group");
    assert_inside_frame(&lines, "InA", left, right);
    assert_inside_frame(&lines, "InB", left, right);
    assert_outside_frame(&lines, "OutX", left, right);

    let a_row = row_with(&lines, "InA");
    assert!(a_row < b_row, "InA must stay above InB:\n{output}");
    assert_label_on_edge(&lines, "side", "InA", "OutX");
    assert_no_box_overlap(&output);
}
