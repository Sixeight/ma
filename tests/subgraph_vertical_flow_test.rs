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

    assert_title_at_home(&lines, "Processing");
    assert_entries_cross_after_title(&lines, "Processing");
    for step in ["Step 1", "Step 2", "Step 3"] {
        assert_stem_misses_title(&lines, "Processing", step);
    }

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

fn assert_title_at_home(lines: &[&str], title: &str) {
    assert_title_intact(lines, title);
    let title_line = lines[row_with(lines, title)];
    let home = format!("┌─ {title} ─");
    assert!(
        title_line.contains(&home),
        "title must sit at its normal ┌─ Title ─ position: {title_line:?}"
    );
    let title_at = title_line.find(title).expect("title text");
    for ch in title_line[title_at..title_at + title.len()].chars() {
        assert!(
            !matches!(ch, '┼' | '│' | '▼' | '┬' | '┴'),
            "entry glyph {ch:?} must not sit inside the title: {title_line:?}"
        );
    }
}

fn assert_entries_cross_after_title(lines: &[&str], title: &str) {
    let title_line = lines[row_with(lines, title)];
    let title_at = title_line.find(title).expect("title text");
    let after = &title_line[title_at + title.len()..];
    assert!(
        after.contains('┼') || after.contains('┬') || after.contains('│'),
        "entry edges must cross the top border after the title: {title_line:?}"
    );
}

fn title_reserved_cols(title_line: &str, title: &str) -> (usize, usize) {
    let start = char_pos(title_line, title);
    (start.saturating_sub(1), start + title.chars().count() + 2)
}

fn assert_stem_misses_title(lines: &[&str], title: &str, step: &str) {
    let title_line = lines[row_with(lines, title)];
    let step_row = row_with(lines, step);
    let col = char_pos(lines[step_row], step) + step.chars().count() / 2;
    let (reserved_start, reserved_end) = title_reserved_cols(title_line, title);
    assert!(
        col < reserved_start || col >= reserved_end,
        "{step} stem col {col} must miss title reserved [{reserved_start},{reserved_end}): {title_line:?}"
    );
    let ch = title_line.chars().nth(col);
    assert!(
        matches!(ch, Some('┼' | '┬' | '│')),
        "{step} must cross the top border at col {col}, got {ch:?}: {title_line:?}"
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

fn assert_outside_subgraph_box(
    lines: &[&str],
    needle: &str,
    title_row: usize,
    bottom_row: usize,
    left: usize,
    right: usize,
) {
    let col = node_col(lines, needle);
    let row = row_with(lines, needle);
    let inside_x = col > left && col + needle.chars().count() <= right;
    let inside_y = row > title_row && row < bottom_row;
    assert!(
        !inside_x || !inside_y,
        "{needle} must sit outside the subgraph box:\n{}",
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

fn glyph_at(lines: &[&str], row: usize, col: usize) -> Option<char> {
    lines.get(row).and_then(|line| line.chars().nth(col))
}

fn label_touches_edge_path(lines: &[&str], label: &str) -> bool {
    let row = row_with(lines, label);
    let start = char_pos(lines[row], label);
    let width = label.chars().count();
    (start..start + width).any(|col| {
        [
            (row.wrapping_sub(1), col),
            (row + 1, col),
            (row, col.wrapping_sub(1)),
            (row, col + 1),
        ]
        .into_iter()
        .any(|(r, c)| glyph_at(lines, r, c).is_some_and(is_line_glyph))
    })
}

fn assert_label_on_edge(output: &str, label: &str, above: &str, below: &str) {
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        output.matches(label).count(),
        1,
        "{label} must appear exactly once:\n{output}"
    );
    let label_row = row_with(&lines, label);
    let above_row = row_with(&lines, above);
    let below_row = row_with(&lines, below);
    let (start, end) = if above_row < below_row {
        (above_row, below_row)
    } else {
        (below_row, above_row)
    };
    assert!(
        start < label_row && label_row < end,
        "{label} must sit on the {above}-{below} edge, row {label_row}:\n{output}"
    );
    assert!(
        label_touches_edge_path(&lines, label),
        "{label} must sit on its edge path:\n{output}"
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

    assert_label_on_edge(&output, "go", "InA", "OutX");
    assert_label_on_edge(&output, "back", "OutX", "InB");

    let b_cx = char_pos(lines[b_row], "InB") + "InB".chars().count() / 2;
    let arrow_row = (0..b_row)
        .rev()
        .find(|&row| glyph_at(&lines, row, b_cx) == Some('▼'))
        .unwrap_or_else(|| panic!("arrow above InB:\n{output}"));
    let inb_box_top = (0..=b_row)
        .rev()
        .find(|&row| glyph_at(&lines, row, char_pos(lines[b_row], "InB") - 2) == Some('┌'))
        .unwrap_or_else(|| panic!("InB box top:\n{output}"));
    assert_eq!(
        arrow_row + 1,
        inb_box_top,
        "▼ must sit on its own row directly above InB:\n{output}"
    );
    let arrow_neighbors = [
        glyph_at(&lines, arrow_row, b_cx.saturating_sub(1)),
        glyph_at(&lines, arrow_row, b_cx + 1),
    ];
    assert!(
        arrow_neighbors.iter().all(|ch| *ch != Some('─')),
        "▼ must not share its row with the horizontal run:\n{output}"
    );
    let turn = glyph_at(&lines, arrow_row.saturating_sub(1), b_cx);
    assert!(
        matches!(turn, Some('┌' | '┐')),
        "horizontal run must end in a corner above ▼, got {turn:?}:\n{output}"
    );
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
    assert_label_on_edge(&output, "side", "InA", "OutX");

    let a_cx = char_pos(lines[a_row], "InA") + "InA".chars().count() / 2;
    let stem_row = (a_row..b_row)
        .find(|&row| glyph_at(&lines, row, a_cx) == Some('┬'))
        .unwrap_or_else(|| panic!("InA bottom stem:\n{output}"));
    let fork = glyph_at(&lines, stem_row + 1, a_cx);
    assert!(
        matches!(fork, Some('├' | '┬' | '┼')),
        "down/right split under InA must be ├/┬/┼, got {fork:?}:\n{output}"
    );
    assert_no_box_overlap(&output);
}

const CASE_LEFT_EDGE_ENTRIES: &str = r#"graph TD
    Src --> A
    Src --> B
    subgraph Leftish
        A[AA]
        B[BB]
    end
"#;

#[test]
fn top_entries_near_the_left_edge_keep_title_at_home() {
    let output = render(CASE_LEFT_EDGE_ENTRIES).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert_title_at_home(&lines, "Leftish");
    assert_entries_cross_after_title(&lines, "Leftish");
    for step in ["AA", "BB"] {
        assert_stem_misses_title(&lines, "Leftish", step);
    }

    let src_row = row_with(&lines, "Src");
    let title_row = row_with(&lines, "Leftish");
    let aa_row = row_with(&lines, "AA");
    let bb_row = row_with(&lines, "BB");
    assert!(src_row < title_row, "Src must sit above the subgraph");
    assert!(
        title_row < aa_row && title_row < bb_row,
        "AA and BB must sit below the title:\n{output}"
    );

    let (title_row, bottom_row, left, right) = subgraph_frame(&lines, "Leftish");
    assert!(title_row < aa_row && aa_row < bottom_row);
    assert!(title_row < bb_row && bb_row < bottom_row);
    assert_inside_frame(&lines, "AA", left, right);
    assert_inside_frame(&lines, "BB", left, right);

    for step in ["AA", "BB"] {
        let step_row = row_with(&lines, step);
        let col = char_pos(lines[step_row], step) + step.chars().count() / 2;
        assert!(
            (title_row..step_row).any(|row| column_has_vertical(lines[row], col)
                || lines[row].chars().nth(col) == Some('▼')),
            "{step} must keep a top-entry stem:\n{output}"
        );
    }
}

const CASE_LR_INTERLEAVE: &str = r#"flowchart LR
    subgraph G[Group]
        A[InA]
        B[InB]
    end
    A -->|go| X[OutX]
    X -->|back| B
"#;

const CASE_LR_SHARE_COL: &str = r#"flowchart LR
    subgraph G[Group]
        A[InA] --> B[InB]
    end
    A -->|side| X[OutX]
"#;

fn is_horizontal_connector(ch: char) -> bool {
    matches!(
        ch,
        '─' | '╌' | '>' | '<' | '┼' | '┬' | '┴' | '├' | '┤' | '┌' | '┐' | '└' | '┘'
    )
}

fn row_has_horizontal(line: &str, col: usize) -> bool {
    line.chars().nth(col).is_some_and(is_horizontal_connector)
}

fn assert_label_on_lr_edge(output: &str, label: &str, left: &str, right: &str) {
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        output.matches(label).count(),
        1,
        "{label} must appear exactly once:\n{output}"
    );
    let label_col = node_col(&lines, label);
    let left_col = node_col(&lines, left);
    let right_col = node_col(&lines, right);
    let (start, end) = if left_col < right_col {
        (left_col, right_col)
    } else {
        (right_col, left_col)
    };
    assert!(
        start < label_col && label_col < end,
        "{label} must sit on the {left}-{right} edge, col {label_col}:\n{output}"
    );
    assert!(
        label_touches_edge_path(&lines, label),
        "{label} must sit on its edge path:\n{output}"
    );
}

fn assert_arrowheads_isolated(lines: &[&str], output: &str) {
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == '>' || ch == '<' {
                let above = glyph_at(lines, row.wrapping_sub(1), col);
                let below = glyph_at(lines, row + 1, col);
                assert!(
                    !above.is_some_and(is_vertical_connector)
                        && !below.is_some_and(is_vertical_connector),
                    "{ch} at ({row},{col}) shares a column with a perpendicular segment:\n{output}"
                );
            }
            if ch == '▼' || ch == '▲' {
                let left = glyph_at(lines, row, col.saturating_sub(1));
                let right = glyph_at(lines, row, col + 1);
                assert!(
                    left != Some('─') && right != Some('─'),
                    "{ch} at ({row},{col}) shares a row with a perpendicular segment:\n{output}"
                );
            }
        }
    }
}

#[test]
fn outer_node_ranks_between_lr_subgraph_members() {
    let output = render(CASE_LR_INTERLEAVE).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let a_col = node_col(&lines, "InA");
    let x_col = node_col(&lines, "OutX");
    let b_col = node_col(&lines, "InB");
    assert!(
        a_col < x_col && x_col < b_col,
        "OutX must rank between InA and InB, cols InA={a_col} OutX={x_col} InB={b_col}:\n{output}"
    );

    assert_title_at_home(&lines, "Group");
    let (title_row, bottom_row, left, right) = subgraph_frame(&lines, "Group");
    let a_row = row_with(&lines, "InA");
    let b_row = row_with(&lines, "InB");
    let x_row = row_with(&lines, "OutX");
    assert!(title_row < a_row && a_row < bottom_row);
    assert!(title_row < b_row && b_row < bottom_row);
    assert_inside_frame(&lines, "InA", left, right);
    assert_inside_frame(&lines, "InB", left, right);
    assert_outside_subgraph_box(&lines, "OutX", title_row, bottom_row, left, right);
    assert!(
        left < x_col && x_col < right,
        "OutX must sit above or below the subgraph, not left or right of it:\n{output}"
    );
    assert!(
        x_row < title_row || x_row > bottom_row,
        "OutX must sit outside the subgraph band:\n{output}"
    );

    assert_label_on_lr_edge(&output, "go", "InA", "OutX");
    assert_label_on_lr_edge(&output, "back", "OutX", "InB");

    let b_left = char_pos(lines[b_row], "InB") - 2;
    let arrow_col = (0..b_left)
        .rev()
        .find(|&col| glyph_at(&lines, b_row, col) == Some('>'))
        .unwrap_or_else(|| panic!("arrow left of InB:\n{output}"));
    assert_eq!(
        arrow_col + 1,
        b_left,
        "> must sit on its own column directly left of InB:\n{output}"
    );
    let arrow_neighbors = [
        glyph_at(&lines, b_row.wrapping_sub(1), arrow_col),
        glyph_at(&lines, b_row + 1, arrow_col),
    ];
    assert!(
        arrow_neighbors
            .iter()
            .all(|ch| !ch.is_some_and(is_vertical_connector)),
        "> must not share its column with a vertical run:\n{output}"
    );
    let turn = glyph_at(&lines, b_row, arrow_col.saturating_sub(1));
    assert!(
        matches!(turn, Some('┌' | '└')),
        "vertical run must end in a corner left of >, got {turn:?}:\n{output}"
    );
    assert_arrowheads_isolated(&lines, &output);
    assert_no_box_overlap(&output);
}

#[test]
fn outer_node_shares_a_column_with_an_inner_node() {
    let output = render(CASE_LR_SHARE_COL).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let b_col = node_col(&lines, "InB");
    let x_col = node_col(&lines, "OutX");
    assert_eq!(b_col, x_col, "OutX must share InB's column:\n{output}");

    assert_title_at_home(&lines, "Group");
    let (title_row, bottom_row, left, right) = subgraph_frame(&lines, "Group");
    assert_inside_frame(&lines, "InA", left, right);
    assert_inside_frame(&lines, "InB", left, right);
    assert_outside_subgraph_box(&lines, "OutX", title_row, bottom_row, left, right);

    let a_col = node_col(&lines, "InA");
    let b_left = node_col(&lines, "InB");
    assert!(a_col < b_left, "InA must stay left of InB:\n{output}");
    assert_label_on_lr_edge(&output, "side", "InA", "OutX");

    let a_row = row_with(&lines, "InA");
    let a_right = char_pos(lines[a_row], "InA") + "InA".chars().count() + 2;
    let stem_col = (a_right..b_left)
        .find(|&col| {
            glyph_at(&lines, a_row, col) == Some('┬') || glyph_at(&lines, a_row, col) == Some('┤')
        })
        .unwrap_or_else(|| panic!("InA right stem:\n{output}"));
    let fork = glyph_at(&lines, a_row, stem_col);
    assert!(
        matches!(fork, Some('┬' | '┤' | '┼')),
        "right/down split after InA must be ┬/┤/┼, got {fork:?}:\n{output}"
    );
    assert!(
        (0..lines.len()).any(|row| row_has_horizontal(lines[row], stem_col)
            || glyph_at(&lines, row, stem_col) == Some('▼')
            || glyph_at(&lines, row, stem_col) == Some('│')),
        "split after InA must continue toward OutX:\n{output}"
    );
    assert_arrowheads_isolated(&lines, &output);
    assert_no_box_overlap(&output);
}

const CASE_NESTED_TD: &str = r#"flowchart TD
    Start -->|enter| InB
    subgraph Outer[Outer]
        subgraph Inner[Inner]
            InB
        end
        InB -->|leave| OutC
        OutC
    end
"#;

const CASE_NESTED_LR: &str = r#"flowchart LR
    Start -->|enter| InB
    subgraph Outer[Outer]
        subgraph Inner[Inner]
            InB
        end
        InB -->|leave| OutC
        OutC
    end
"#;

fn title_frame(lines: &[&str], title: &str) -> (usize, usize, usize, usize) {
    let title_row = row_with(lines, title);
    let chars: Vec<char> = lines[title_row].chars().collect();
    let title_chars: Vec<char> = title.chars().collect();
    let title_at = chars
        .windows(title_chars.len())
        .position(|window| window == title_chars.as_slice())
        .unwrap_or_else(|| panic!("title {title} columns:\n{}", lines.join("\n")));
    let left = (0..=title_at)
        .rev()
        .find(|&col| chars[col] == '┌')
        .unwrap_or_else(|| panic!("┌ before {title}:\n{}", lines.join("\n")));
    let right = (title_at + title_chars.len()..chars.len())
        .find(|&col| matches!(chars[col], '┐' | '┤'))
        .unwrap_or_else(|| panic!("┐ after {title}:\n{}", lines.join("\n")));
    let bottom_row = ((title_row + 1)..lines.len())
        .find(|&row| {
            let end: Vec<char> = lines[row].chars().collect();
            end.get(left) == Some(&'└') && end.get(right) == Some(&'┘')
        })
        .unwrap_or_else(|| panic!("bottom of {title}:\n{}", lines.join("\n")));
    (title_row, bottom_row, left, right)
}

fn assert_inside_box(
    lines: &[&str],
    needle: &str,
    title_row: usize,
    bottom_row: usize,
    left: usize,
    right: usize,
) {
    let row = row_with(lines, needle);
    let col = node_col(lines, needle);
    assert!(
        row > title_row && row < bottom_row && col > left && col + needle.chars().count() <= right,
        "{needle} must sit inside the box [{left},{right}] x [{title_row},{bottom_row}]:\n{}",
        lines.join("\n")
    );
}

fn assert_nested_frames(lines: &[&str], output: &str) {
    assert_title_at_home(lines, "Outer");
    assert_title_at_home(lines, "Inner");

    let (outer_top, outer_bottom, outer_left, outer_right) = title_frame(lines, "Outer");
    let (inner_top, inner_bottom, inner_left, inner_right) = title_frame(lines, "Inner");

    assert_ne!(
        outer_top, inner_top,
        "each title must occupy its own row:\n{output}"
    );
    assert!(
        !lines[outer_top].contains("Inner"),
        "Inner title must not sit on the Outer title row:\n{output}"
    );
    assert!(
        !lines[inner_top].contains("Outer"),
        "Outer title must not sit on the Inner title row:\n{output}"
    );

    assert!(
        inner_top >= outer_top + 2
            && inner_bottom + 2 <= outer_bottom
            && inner_left >= outer_left + 2
            && inner_right + 2 <= outer_right,
        "Inner frame must sit fully inside Outer with at least one cell of padding:\n{output}"
    );

    let outer_border = perimeter(outer_top, outer_bottom, outer_left, outer_right);
    let inner_border = perimeter(inner_top, inner_bottom, inner_left, inner_right);
    assert!(
        outer_border.is_disjoint(&inner_border),
        "frames must not share border cells:\n{output}"
    );

    assert_outside_subgraph_box(
        lines,
        "Start",
        outer_top,
        outer_bottom,
        outer_left,
        outer_right,
    );
    assert_inside_box(
        lines,
        "InB",
        inner_top,
        inner_bottom,
        inner_left,
        inner_right,
    );
    assert_inside_box(
        lines,
        "OutC",
        outer_top,
        outer_bottom,
        outer_left,
        outer_right,
    );
    assert_outside_subgraph_box(
        lines,
        "OutC",
        inner_top,
        inner_bottom,
        inner_left,
        inner_right,
    );
}

fn perimeter(
    title_row: usize,
    bottom_row: usize,
    left: usize,
    right: usize,
) -> std::collections::HashSet<(usize, usize)> {
    let mut cells = std::collections::HashSet::new();
    for col in left..=right {
        cells.insert((title_row, col));
        cells.insert((bottom_row, col));
    }
    for row in title_row..=bottom_row {
        cells.insert((row, left));
        cells.insert((row, right));
    }
    cells
}

fn horizontal_border_crossings(lines: &[&str], row: usize, left: usize, right: usize) -> usize {
    (left + 1..right)
        .filter(|&col| {
            matches!(
                glyph_at(lines, row, col),
                Some('┼' | '┬' | '┴' | '├' | '┤' | '│' | '▼' | '┊')
            )
        })
        .count()
}

fn vertical_border_crossings(lines: &[&str], col: usize, top: usize, bottom: usize) -> usize {
    (top + 1..bottom)
        .filter(|&row| {
            matches!(
                glyph_at(lines, row, col),
                Some('┼' | '┬' | '┴' | '├' | '┤' | '─' | '>' | '╌')
            )
        })
        .count()
}

fn assert_td_nested_crossings(lines: &[&str], output: &str) {
    let (outer_top, outer_bottom, outer_left, outer_right) = title_frame(lines, "Outer");
    let (inner_top, inner_bottom, inner_left, inner_right) = title_frame(lines, "Inner");
    assert_eq!(
        horizontal_border_crossings(lines, outer_top, outer_left, outer_right),
        1,
        "Start→InB must cross the Outer top border once:\n{output}"
    );
    assert_eq!(
        horizontal_border_crossings(lines, inner_top, inner_left, inner_right),
        1,
        "Start→InB must cross the Inner top border once:\n{output}"
    );
    assert_eq!(
        horizontal_border_crossings(lines, inner_bottom, inner_left, inner_right),
        1,
        "InB→OutC must cross the Inner bottom border once:\n{output}"
    );
    assert_eq!(
        horizontal_border_crossings(lines, outer_bottom, outer_left, outer_right),
        0,
        "InB→OutC must stay inside Outer and not cross its bottom:\n{output}"
    );
}

fn assert_lr_nested_crossings(lines: &[&str], output: &str) {
    let (outer_top, outer_bottom, outer_left, outer_right) = title_frame(lines, "Outer");
    let (inner_top, inner_bottom, inner_left, inner_right) = title_frame(lines, "Inner");
    assert_eq!(
        vertical_border_crossings(lines, outer_left, outer_top, outer_bottom),
        1,
        "Start→InB must cross the Outer left border once:\n{output}"
    );
    assert_eq!(
        vertical_border_crossings(lines, inner_left, inner_top, inner_bottom),
        1,
        "Start→InB must cross the Inner left border once:\n{output}"
    );
    assert_eq!(
        vertical_border_crossings(lines, inner_right, inner_top, inner_bottom),
        1,
        "InB→OutC must cross the Inner right border once:\n{output}"
    );
    assert_eq!(
        vertical_border_crossings(lines, outer_right, outer_top, outer_bottom),
        0,
        "InB→OutC must stay inside Outer and not cross its right border:\n{output}"
    );
}

#[test]
fn nested_td_inner_frame_sits_inside_outer() {
    let output = render(CASE_NESTED_TD).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert_nested_frames(&lines, &output);
    assert_td_nested_crossings(&lines, &output);
    assert_label_on_edge(&output, "enter", "Start", "InB");
    assert_label_on_edge(&output, "leave", "InB", "OutC");
    assert_no_box_overlap(&output);

    let start_row = row_with(&lines, "Start");
    let inb_row = row_with(&lines, "InB");
    let outc_row = row_with(&lines, "OutC");
    assert!(
        start_row < inb_row && inb_row < outc_row,
        "Start must sit above InB, and InB above OutC:\n{output}"
    );
}

#[test]
fn nested_lr_inner_frame_sits_inside_outer() {
    let output = render(CASE_NESTED_LR).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert_nested_frames(&lines, &output);
    assert_lr_nested_crossings(&lines, &output);
    assert_label_on_lr_edge(&output, "enter", "Start", "InB");
    assert_label_on_lr_edge(&output, "leave", "InB", "OutC");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_box_overlap(&output);

    let start_col = node_col(&lines, "Start");
    let inb_col = node_col(&lines, "InB");
    let outc_col = node_col(&lines, "OutC");
    assert!(
        start_col < inb_col && inb_col < outc_col,
        "Start must sit left of InB, and InB left of OutC:\n{output}"
    );
}
