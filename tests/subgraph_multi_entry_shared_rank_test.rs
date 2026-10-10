use ma::render;

const CASE_THREE_TOP_ENTRIES: &str = r#"graph TD
    S1[S1] -->|a| A
    S2[S2] -->|b| B
    S3[S3] -->|c| C
    subgraph Cluster
        A[AA]
        B[BB]
        C[CC]
    end
"#;

const CASE_LR_SIDE_ENTRIES: &str = r#"flowchart LR
    subgraph Processing
        A[Step 1]
        B[Step 2]
        C[Step 3]
    end
    D{Decision} -->|Yes| A
    D -->|No| B
    D -->|Maybe| C
"#;

const CASE_TD_OUTERS_SAME_SIDE: &str = r#"flowchart TD
    subgraph Cluster
        A[InA] --> B[InB]
    end
    A -->|s1| X[OutX]
    A -->|s2| Y[OutY]
    A -->|s3| Z[OutZ]
"#;

const CASE_TD_OUTERS_BOTH_SIDES: &str = r#"flowchart TD
    subgraph Cluster
        A[InA] --> B[InB]
        C[InC]
    end
    A --> L1[Left1]
    A --> L2[Left2]
    C --> R1[Right1]
    C --> R2[Right2]
"#;

const CASE_LR_OUTERS_SAME_SIDE: &str = r#"flowchart LR
    subgraph Cluster
        A[InA] --> B[InB]
    end
    A -->|s1| X[OutX]
    A -->|s2| Y[OutY]
    A -->|s3| Z[OutZ]
"#;

const CASE_LR_OUTERS_BOTH_SIDES: &str = r#"flowchart LR
    subgraph Cluster
        A[InA] --> B[InB]
        C[InC]
    end
    A --> U1[Up1]
    A --> U2[Up2]
    C --> D1[Down1]
    C --> D2[Down2]
"#;

const CASE_TD_OFFSET_AND_OUTERS: &str = r#"flowchart TD
    subgraph Cluster
        A[InA] --> B[InB]
    end
    W[West] --> A
    A -->|s1| X[OutX]
    A -->|s2| Y[OutY]
    A -->|s3| Z[OutZ]
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

fn glyph_at(lines: &[&str], row: usize, col: usize) -> Option<char> {
    lines.get(row).and_then(|line| line.chars().nth(col))
}

fn node_center_col(lines: &[&str], needle: &str) -> usize {
    char_pos(lines[row_with(lines, needle)], needle) + needle.chars().count() / 2
}

fn node_box_left(lines: &[&str], needle: &str) -> usize {
    let row = row_with(lines, needle);
    let text_col = char_pos(lines[row], needle);
    let chars: Vec<char> = lines[row].chars().collect();
    (0..text_col)
        .rev()
        .find(|&col| chars[col] == '│')
        .unwrap_or_else(|| panic!("box left of {needle}:\n{}", lines.join("\n")))
}

fn node_box_right(lines: &[&str], needle: &str) -> usize {
    let row = row_with(lines, needle);
    let left = node_box_left(lines, needle);
    let chars: Vec<char> = lines[row].chars().collect();
    ((left + 1)..chars.len())
        .find(|&col| chars[col] == '│')
        .unwrap_or_else(|| panic!("box right of {needle}:\n{}", lines.join("\n")))
}

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

fn is_vertical_connector(ch: char) -> bool {
    matches!(
        ch,
        '│' | '┊'
            | '║'
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

fn assert_title_at_home(lines: &[&str], title: &str) {
    let title_line = lines[row_with(lines, title)];
    let home = format!("┌─ {title} ─");
    assert!(
        title_line.contains(&home),
        "title must sit at ┌─ Title ─: {title_line:?}"
    );
    let title_at = title_line.find(title).expect("title text");
    for ch in title_line[title_at..title_at + title.len()].chars() {
        assert!(
            !matches!(ch, '┼' | '│' | '▼' | '┬' | '┴' | '>' | '─'),
            "entry glyph {ch:?} must not sit inside the title: {title_line:?}"
        );
    }
}

fn assert_arrowheads_off_frame(lines: &[&str], output: &str, title: &str) {
    let (top, bottom, left, right) = subgraph_frame(lines, title);
    for row in [top, bottom] {
        let line = lines[row];
        assert!(
            !line.contains('▼')
                && !line.contains('▲')
                && !line.contains('>')
                && !line.contains('<'),
            "arrowhead must not sit on a frame border or title row {row}: {line:?}\n{output}"
        );
    }
    for row in top + 1..bottom {
        assert!(
            glyph_at(lines, row, left) != Some('>') && glyph_at(lines, row, right) != Some('>'),
            "arrowhead must not sit on a side border at row {row}:\n{output}"
        );
    }
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

fn assert_label_once_on_vertical(output: &str, label: &str) {
    assert_eq!(
        output.matches(label).count(),
        1,
        "{label} must appear exactly once:\n{output}"
    );
    let lines: Vec<&str> = output.lines().collect();
    let row = row_with(&lines, label);
    let start = char_pos(lines[row], label);
    let end = start + label.chars().count();
    let chars: Vec<char> = lines[row].chars().collect();
    let left = start.checked_sub(1).and_then(|col| chars.get(col).copied());
    let right = chars.get(end).copied();
    assert!(
        !matches!(left, Some('─' | '╌')),
        "{label} must sit on a vertical segment, left {left:?}:\n{output}"
    );
    assert!(
        !matches!(right, Some('─' | '╌')),
        "{label} must sit on a vertical segment, right {right:?}:\n{output}"
    );
}

fn assert_label_once(output: &str, label: &str) {
    assert_eq!(
        output.matches(label).count(),
        1,
        "{label} must appear exactly once:\n{output}"
    );
}

fn assert_top_entry_own_column(lines: &[&str], output: &str, title: &str, step: &str) {
    let (title_row, _bottom, left, right) = subgraph_frame(lines, title);
    let col = node_center_col(lines, step);
    assert!(
        col > left && col < right,
        "{step} center {col} must sit inside the frame [{left},{right}]:\n{output}"
    );
    let title_line = lines[title_row];
    let title_at = char_pos(title_line, title);
    let reserved_end = title_at + title.chars().count() + 2;
    assert!(
        col >= reserved_end,
        "{step} must cross after the title reserved end {reserved_end}, col {col}: {title_line:?}"
    );
    let ch = glyph_at(lines, title_row, col);
    assert!(
        matches!(ch, Some('┼' | '┬' | '│')),
        "{step} must cross the top border at its own column {col}, got {ch:?}: {title_line:?}"
    );
}

fn assert_straight_stem_inside(lines: &[&str], output: &str, title: &str, step: &str) {
    let (title_row, _, _, _) = subgraph_frame(lines, title);
    let step_row = row_with(lines, step);
    let col = node_center_col(lines, step);
    let box_top = (title_row..step_row)
        .rev()
        .find(|&row| glyph_at(lines, row, node_box_left(lines, step)) == Some('┌'))
        .unwrap_or_else(|| panic!("{step} box top:\n{output}"));
    let arrow_row = (title_row + 1..box_top)
        .rev()
        .find(|&row| glyph_at(lines, row, col) == Some('▼'))
        .unwrap_or_else(|| panic!("▼ above {step} at col {col}:\n{output}"));
    assert_eq!(
        arrow_row + 1,
        box_top,
        "▼ must sit on its own row directly above {step}:\n{output}"
    );
    for row in (title_row + 1)..arrow_row {
        let ch = glyph_at(lines, row, col);
        assert!(
            ch.is_some_and(|c| is_vertical_connector(c) || c.is_ascii_alphanumeric()),
            "stem to {step} must stay on column {col} inside the frame, row {row} got {ch:?}:\n{output}"
        );
        let left = glyph_at(lines, row, col.saturating_sub(1));
        let right = glyph_at(lines, row, col + 1);
        if ch == Some('│') || ch == Some('┊') {
            assert!(
                left != Some('─') && right != Some('─'),
                "vertical stem to {step} must not share row {row} with a crossing ─ unless ┼:\n{output}"
            );
        }
    }
}

fn assert_unique_top_crossings(lines: &[&str], output: &str, title: &str, steps: &[&str]) {
    let (title_row, _, left, right) = subgraph_frame(lines, title);
    let mut cols = Vec::new();
    for step in steps {
        let col = node_center_col(lines, step);
        cols.push(col);
        assert_top_entry_own_column(lines, output, title, step);
        assert_straight_stem_inside(lines, output, title, step);
    }
    cols.sort();
    cols.dedup();
    assert_eq!(
        cols.len(),
        steps.len(),
        "each top entry must keep its own crossing column:\n{output}"
    );
    let crossings = (left + 1..right)
        .filter(|&col| matches!(glyph_at(lines, title_row, col), Some('┼' | '┬' | '│' | '┴')))
        .count();
    assert_eq!(
        crossings,
        steps.len(),
        "top border must carry exactly one crossing per entry:\n{output}"
    );
}

fn assert_outside_frame(lines: &[&str], needle: &str, left: usize, right: usize) {
    let col = char_pos(lines[row_with(lines, needle)], needle);
    assert!(
        col + needle.chars().count() <= left || col >= right,
        "{needle} must sit outside the subgraph border [{left},{right}]:\n{}",
        lines.join("\n")
    );
}

fn assert_even_box_gaps(lines: &[&str], output: &str, names: &[&str]) {
    let mut lefts: Vec<usize> = names
        .iter()
        .map(|name| node_box_left(lines, name))
        .collect();
    lefts.sort();
    let rights: Vec<usize> = names
        .iter()
        .map(|name| node_box_right(lines, name))
        .collect();
    let mut ordered_rights = Vec::new();
    for left in &lefts {
        let name = names
            .iter()
            .find(|name| node_box_left(lines, name) == *left)
            .unwrap();
        ordered_rights.push(node_box_right(lines, name));
    }
    let _ = rights;
    let gaps: Vec<usize> = lefts
        .windows(2)
        .enumerate()
        .map(|(i, pair)| pair[1] - ordered_rights[i] - 1)
        .collect();
    assert!(
        gaps.windows(2).all(|pair| pair[0] == pair[1]),
        "outer boxes must keep even spacing {gaps:?}:\n{output}"
    );
    assert!(
        gaps.iter().all(|gap| *gap >= 2),
        "outer boxes must keep at least normal node spacing {gaps:?}:\n{output}"
    );
}

fn assert_no_edge_along_border(lines: &[&str], output: &str, title: &str) {
    let (top, bottom, left, right) = subgraph_frame(lines, title);
    for row in top + 1..bottom {
        for col in [left, right] {
            let ch = glyph_at(lines, row, col);
            assert!(
                !matches!(ch, Some('─' | '╌')),
                "edge must not run along the side border at ({row},{col}):\n{output}"
            );
        }
    }
}

fn assert_node_box_intact(lines: &[&str], output: &str, needle: &str) {
    let text_row = row_with(lines, needle);
    let left = node_box_left(lines, needle);
    let right = node_box_right(lines, needle);
    let top = text_row - 1;
    let bottom = text_row + 1;
    assert_eq!(
        glyph_at(lines, top, left),
        Some('┌'),
        "{needle} top-left must stay a box corner:\n{output}"
    );
    assert_eq!(
        glyph_at(lines, top, right),
        Some('┐'),
        "{needle} top-right must stay a box corner:\n{output}"
    );
    assert_eq!(
        glyph_at(lines, bottom, left),
        Some('└'),
        "{needle} bottom-left must stay a box corner:\n{output}"
    );
    assert_eq!(
        glyph_at(lines, bottom, right),
        Some('┘'),
        "{needle} bottom-right must stay a box corner:\n{output}"
    );
    for col in left + 1..right {
        let top_ch = glyph_at(lines, top, col);
        assert!(
            matches!(top_ch, Some('─' | '┬' | '┴')),
            "{needle} top border overwritten at col {col} with {top_ch:?}:\n{output}"
        );
        let bottom_ch = glyph_at(lines, bottom, col);
        assert!(
            matches!(bottom_ch, Some('─' | '┬' | '┴')),
            "{needle} bottom border overwritten at col {col} with {bottom_ch:?}:\n{output}"
        );
    }
}

fn assert_lr_side_entries(lines: &[&str], output: &str, title: &str, steps: &[&str]) {
    let (top, bottom, left, right) = subgraph_frame(lines, title);
    let mut rows = Vec::new();
    for step in steps {
        let step_row = row_with(lines, step);
        rows.push(step_row);
        let crossing = glyph_at(lines, step_row, left);
        assert!(
            matches!(crossing, Some('┼' | '├' | '─')),
            "{step} must cross the left border on its own row, got {crossing:?}:\n{output}"
        );
        let arrow_col = (left + 1..right)
            .find(|&col| glyph_at(lines, step_row, col) == Some('>'))
            .unwrap_or_else(|| panic!("arrow into {step}:\n{output}"));
        assert!(
            arrow_col > left && arrow_col < right,
            "arrow into {step} must sit inside the frame, not on the border:\n{output}"
        );
        assert_eq!(
            arrow_col + 1,
            node_box_left(lines, step),
            "> must sit directly left of {step}:\n{output}"
        );
    }
    rows.sort();
    rows.dedup();
    assert_eq!(
        rows.len(),
        steps.len(),
        "each side entry must keep its own crossing row:\n{output}"
    );
}

#[test]
fn three_top_entries_use_own_columns_and_straight_stems() {
    let output = render(CASE_THREE_TOP_ENTRIES).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert_title_at_home(&lines, "Cluster");
    assert_unique_top_crossings(&lines, &output, "Cluster", &["AA", "BB", "CC"]);
    for label in ["a", "b", "c"] {
        assert_label_once_on_vertical(&output, label);
    }
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for step in ["AA", "BB", "CC", "S1", "S2", "S3"] {
        assert_node_box_intact(&lines, &output, step);
    }
}

#[test]
fn lr_side_entries_cross_once_per_row_with_inside_arrows() {
    let output = render(CASE_LR_SIDE_ENTRIES).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert_title_at_home(&lines, "Processing");
    assert_lr_side_entries(
        &lines,
        &output,
        "Processing",
        &["Step 1", "Step 2", "Step 3"],
    );
    let left = subgraph_frame(&lines, "Processing").2;
    for label in ["Yes", "No", "Maybe"] {
        assert_label_once_on_vertical(&output, label);
        let row = row_with(&lines, label);
        let start = char_pos(lines[row], label);
        let end = start + label.chars().count();
        assert!(
            start != left && end != left,
            "{label} must not sit on or against the left border:\n{output}"
        );
    }
    assert_arrowheads_off_frame(&lines, &output, "Processing");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Processing");
    assert_no_box_overlap(&output);
}

#[test]
fn td_three_outers_same_side_keep_spacing() {
    let output = render(CASE_TD_OUTERS_SAME_SIDE).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let (title_row, bottom, left, right) = subgraph_frame(&lines, "Cluster");

    assert_title_at_home(&lines, "Cluster");
    let inb_row = row_with(&lines, "InB");
    for name in ["OutX", "OutY", "OutZ"] {
        assert_eq!(
            row_with(&lines, name),
            inb_row,
            "{name} must share InB's rank:\n{output}"
        );
        assert_outside_frame(&lines, name, left, right);
        assert!(
            title_row < row_with(&lines, name) && row_with(&lines, name) < bottom,
            "{name} must sit beside the frame:\n{output}"
        );
    }
    assert_even_box_gaps(&lines, &output, &["OutX", "OutY", "OutZ"]);
    for label in ["s1", "s2", "s3"] {
        assert_label_once(&output, label);
    }
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for name in ["InA", "InB", "OutX", "OutY", "OutZ"] {
        assert_node_box_intact(&lines, &output, name);
    }
}

#[test]
fn td_outers_split_to_both_sides() {
    let output = render(CASE_TD_OUTERS_BOTH_SIDES).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let (title_row, bottom, left, right) = subgraph_frame(&lines, "Cluster");

    assert_title_at_home(&lines, "Cluster");
    let inb_row = row_with(&lines, "InB");
    for name in ["Left1", "Left2", "Right1", "Right2"] {
        assert_eq!(
            row_with(&lines, name),
            inb_row,
            "{name} must share InB's rank:\n{output}"
        );
        assert_outside_frame(&lines, name, left, right);
        assert!(
            title_row < row_with(&lines, name) && row_with(&lines, name) < bottom,
            "{name} must sit beside the frame:\n{output}"
        );
    }
    let left1 = node_box_right(&lines, "Left1");
    let left2 = node_box_right(&lines, "Left2");
    let right1 = node_box_left(&lines, "Right1");
    let right2 = node_box_left(&lines, "Right2");
    assert!(
        left1 < left && left2 < left,
        "Left1 and Left2 must sit left of the frame:\n{output}"
    );
    assert!(
        right1 > right && right2 > right,
        "Right1 and Right2 must sit right of the frame:\n{output}"
    );
    assert_even_box_gaps(&lines, &output, &["Left1", "Left2"]);
    assert_even_box_gaps(&lines, &output, &["Right1", "Right2"]);
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for name in ["InA", "InB", "InC", "Left1", "Left2", "Right1", "Right2"] {
        assert_node_box_intact(&lines, &output, name);
    }
}

#[test]
fn lr_three_outers_same_side_keep_spacing() {
    let output = render(CASE_LR_OUTERS_SAME_SIDE).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let (title_row, bottom, left, right) = subgraph_frame(&lines, "Cluster");

    assert_title_at_home(&lines, "Cluster");
    let inb_left = node_box_left(&lines, "InB");
    for name in ["OutX", "OutY", "OutZ"] {
        assert_outside_frame(&lines, name, left, right);
        let row = row_with(&lines, name);
        assert!(
            row < title_row || row > bottom,
            "{name} must sit above or below the frame, not inside it:\n{output}"
        );
        let name_left = node_box_left(&lines, name);
        assert!(
            name_left.abs_diff(inb_left) <= 2,
            "{name} must share InB's rank column:\n{output}"
        );
    }
    let rows = ["OutX", "OutY", "OutZ"].map(|name| row_with(&lines, name));
    let mut sorted = rows;
    sorted.sort();
    let gaps = [sorted[1] - sorted[0], sorted[2] - sorted[1]];
    assert_eq!(
        gaps[0], gaps[1],
        "outer boxes must keep even vertical spacing:\n{output}"
    );
    for label in ["s1", "s2", "s3"] {
        assert_label_once(&output, label);
    }
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for name in ["InA", "InB", "OutX", "OutY", "OutZ"] {
        assert_node_box_intact(&lines, &output, name);
    }
}

#[test]
fn lr_outers_split_above_and_below() {
    let output = render(CASE_LR_OUTERS_BOTH_SIDES).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let (title_row, bottom, left, right) = subgraph_frame(&lines, "Cluster");

    assert_title_at_home(&lines, "Cluster");
    for name in ["Up1", "Up2", "Down1", "Down2"] {
        assert_outside_frame(&lines, name, left, right);
    }
    for name in ["Up1", "Up2"] {
        assert!(
            row_with(&lines, name) < title_row,
            "{name} must sit above the frame:\n{output}"
        );
    }
    for name in ["Down1", "Down2"] {
        assert!(
            row_with(&lines, name) > bottom,
            "{name} must sit below the frame:\n{output}"
        );
    }
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for name in ["InA", "InB", "InC", "Up1", "Up2", "Down1", "Down2"] {
        assert_node_box_intact(&lines, &output, name);
    }
}

#[test]
fn td_offset_top_entry_plus_three_outers() {
    let output = render(CASE_TD_OFFSET_AND_OUTERS).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let (_title_row, _bottom, left, right) = subgraph_frame(&lines, "Cluster");

    assert_title_at_home(&lines, "Cluster");
    assert_top_entry_own_column(&lines, &output, "Cluster", "InA");
    assert_straight_stem_inside(&lines, &output, "Cluster", "InA");
    for name in ["OutX", "OutY", "OutZ"] {
        assert_outside_frame(&lines, name, left, right);
    }
    assert_even_box_gaps(&lines, &output, &["OutX", "OutY", "OutZ"]);
    for label in ["s1", "s2", "s3"] {
        assert_label_once(&output, label);
    }
    assert_arrowheads_off_frame(&lines, &output, "Cluster");
    assert_arrowheads_isolated(&lines, &output);
    assert_no_edge_along_border(&lines, &output, "Cluster");
    assert_no_box_overlap(&output);
    for name in ["West", "InA", "InB", "OutX", "OutY", "OutZ"] {
        assert_node_box_intact(&lines, &output, name);
    }
}
