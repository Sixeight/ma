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
    let byte = line
        .find(needle)
        .unwrap_or_else(|| panic!("expected {needle:?} in {line:?}"));
    line[..byte].chars().count()
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
    assert_inherited_lr_members_packed(&with_dir, "Cluster", "│ A │", "│ B │");
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

const MERMAID_DOC: &str = "\
flowchart LR
    subgraph subgraph1
        direction TB
        top1[top] --> bottom1[bottom]
    end
    subgraph subgraph2
        direction TB
        top2[top] --> bottom2[bottom]
    end
    outside --> subgraph1
    outside ---> top2
";

#[test]
fn mermaid_doc_subgraph_direction_limitation() {
    let output = render(MERMAID_DOC).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let top1_row = row_with(&lines, "│ top │");
    let bottom1_row = row_with(&lines, "│ bottom │");
    assert!(
        top1_row < bottom1_row,
        "subgraph1 keeps TB because the outside link targets the subgraph id:\n{output}"
    );

    let sg2 = title_frame(&lines, "subgraph2");
    let top2_row = lines
        .iter()
        .enumerate()
        .find(|(i, line)| *i > sg2.0 && line.contains("│ top │"))
        .map(|(i, _)| i)
        .expect("top2");
    let bottom2_row = lines
        .iter()
        .enumerate()
        .find(|(i, line)| *i > sg2.0 && line.contains("│ bottom │"))
        .map(|(i, _)| i)
        .expect("bottom2");
    assert_eq!(
        top2_row, bottom2_row,
        "inherited subgraph2 must keep LR rank packing:\n{output}"
    );
    let top2_col = col_with(lines[top2_row], "│ top │");
    let bottom2_col = col_with(lines[bottom2_row], "│ bottom │");
    assert!(
        top2_col < bottom2_col,
        "subgraph2 members stay in flow order:\n{output}"
    );
    assert!(
        bottom2_col - top2_col <= 16,
        "inherited members must not sit farther than normal LR rank spacing:\n{output}"
    );
    assert!(
        sg2.1 - sg2.0 <= 6,
        "inherited subgraph2 must not grow empty interior:\n{output}"
    );

    let outside = node_box(&lines, "outside");
    let sg1 = title_frame(&lines, "subgraph1");
    assert!(
        outside.3 < sg1.2 && outside.3 < sg2.2,
        "outside must sit on the source side of both frames:\n{output}"
    );
    assert_entry_from_facing_side(&lines, &output, outside, sg1);
    assert_entry_from_facing_side(&lines, &output, outside, sg2);
    assert_arrows_inside_frames(&lines, &output, &[sg1, sg2]);
    assert_no_edge_along_frame(&lines, &output, sg1);
    assert_no_edge_along_frame(&lines, &output, sg2);
    assert_eq!(output, mermaid_doc_golden());
}

fn mermaid_doc_golden() -> String {
    [
        "                ┌─ subgraph1 ─┐",
        "                │   ┌─────┐   │",
        "                │   │ top │   │",
        "                │   └──┬──┘   │",
        "┌─────────┐     │      │      │",
        "│ outside │───┬─┼>     ▼      │",
        "└─────────┘   │ │ ┌────────┐  │",
        "              │ │ │ bottom │  │",
        "              │ │ └────────┘  │",
        "              │ └─────────────┘",
        "              │",
        "              │ ┌─ subgraph2 ────────────┐",
        "              │ │ ┌─────┐     ┌────────┐ │",
        "              └─┼>│ top │────>│ bottom │ │",
        "                │ └─────┘     └────────┘ │",
        "                └────────────────────────┘",
    ]
    .join("\n")
}

fn assert_inherited_lr_members_packed(output: &str, frame: &str, left: &str, right: &str) {
    let lines: Vec<&str> = output.lines().collect();
    let box_frame = title_frame(&lines, frame);
    let left_row = row_with(&lines, left);
    let right_row = row_with(&lines, right);
    assert_eq!(
        left_row, right_row,
        "inherited LR members must share a rank:\n{output}"
    );
    let left_col = col_with(lines[left_row], left);
    let right_col = col_with(lines[right_row], right);
    assert!(
        left_col < right_col,
        "inherited LR members must stay in flow order:\n{output}"
    );
    assert!(
        right_col - left_col <= 16,
        "inherited LR gap exceeds normal rank spacing:\n{output}"
    );
    assert!(
        box_frame.1 - box_frame.0 <= 6,
        "inherited LR frame must not grow empty interior:\n{output}"
    );
}

fn title_frame(lines: &[&str], title: &str) -> (usize, usize, usize, usize) {
    let title_row = row_with(lines, title);
    let chars: Vec<char> = lines[title_row].chars().collect();
    let title_at = col_with(lines[title_row], title);
    let left = (0..=title_at)
        .rev()
        .find(|&col| chars[col] == '┌')
        .unwrap_or_else(|| panic!("┌ before {title}:\n{}", lines.join("\n")));
    let right = (title_at + title.chars().count()..chars.len())
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

fn node_box(lines: &[&str], needle: &str) -> (usize, usize, usize, usize) {
    let row = row_with(lines, needle);
    let text = col_with(lines[row], needle);
    let chars: Vec<char> = lines[row].chars().collect();
    let left = (0..text)
        .rev()
        .find(|&col| chars[col] == '│')
        .unwrap_or_else(|| panic!("box left of {needle}"));
    let right = ((left + 1)..chars.len())
        .find(|&col| chars[col] == '│')
        .unwrap_or_else(|| panic!("box right of {needle}"));
    (row - 1, row + 1, left, right)
}

fn glyph(lines: &[&str], row: usize, col: usize) -> Option<char> {
    lines.get(row).and_then(|line| line.chars().nth(col))
}

fn is_arrow(ch: char) -> bool {
    matches!(ch, '▲' | '▼' | '<' | '>')
}

fn is_edge(ch: char) -> bool {
    matches!(
        ch,
        '─' | '│'
            | '┌'
            | '┐'
            | '└'
            | '┘'
            | '├'
            | '┤'
            | '┬'
            | '┴'
            | '┼'
            | '▲'
            | '▼'
            | '<'
            | '>'
    )
}

fn assert_arrows_inside_frames(
    lines: &[&str],
    output: &str,
    frames: &[(usize, usize, usize, usize)],
) {
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if !is_arrow(ch) {
                continue;
            }
            let on_border = frames.iter().any(|&(top, bottom, left, right)| {
                (row == top || row == bottom || col == left || col == right)
                    && row >= top
                    && row <= bottom
                    && col >= left
                    && col <= right
            });
            assert!(
                !on_border,
                "arrowhead {ch} at ({row},{col}) sits on a frame border:\n{output}"
            );
            let under_or_on = frames.iter().any(|&(top, bottom, left, right)| {
                col >= left && col <= right && (row == top || row == bottom || row == bottom + 1)
            });
            if under_or_on && (ch == '▲' || ch == '▼') {
                let inside = frames.iter().any(|&(top, bottom, left, right)| {
                    row > top && row < bottom && col > left && col < right
                });
                assert!(
                    inside,
                    "arrowhead {ch} at ({row},{col}) sits under or on a frame border:\n{output}"
                );
            }
        }
    }
}

fn assert_entry_from_facing_side(
    lines: &[&str],
    output: &str,
    source: (usize, usize, usize, usize),
    frame: (usize, usize, usize, usize),
) {
    let (top, bottom, left, right) = frame;
    let src_cx = (source.2 + source.3) / 2;
    let src_cy = (source.0 + source.1) / 2;
    let mid_x = (left + right) / 2;
    let mid_y = (top + bottom) / 2;
    let crossings: Vec<(usize, usize, char)> = (top..=bottom)
        .flat_map(|row| {
            [left, right]
                .into_iter()
                .filter_map(move |col| glyph(lines, row, col).map(|ch| (row, col, ch)))
        })
        .chain((left..=right).flat_map(|col| {
            [top, bottom]
                .into_iter()
                .filter_map(move |row| glyph(lines, row, col).map(|ch| (row, col, ch)))
        }))
        .filter(|(_, _, ch)| matches!(ch, '├' | '┤' | '┬' | '┴' | '┼' | '▲' | '▼' | '<' | '>'))
        .collect();
    assert_eq!(
        crossings.len(),
        1,
        "edge must cross the frame border once, found {crossings:?}:\n{output}"
    );
    let (crow, ccol, _) = crossings[0];
    let facing = if src_cx < mid_x && (mid_x - src_cx) >= src_cy.abs_diff(mid_y) {
        ccol == left
    } else if src_cx > mid_x && (src_cx - mid_x) >= src_cy.abs_diff(mid_y) {
        ccol == right
    } else if src_cy < mid_y {
        crow == top
    } else {
        crow == bottom
    };
    assert!(
        facing,
        "entry at ({crow},{ccol}) does not face the source:\n{output}"
    );
}

fn assert_no_edge_along_frame(lines: &[&str], output: &str, frame: (usize, usize, usize, usize)) {
    let (top, bottom, left, right) = frame;
    for row in [top, bottom] {
        if left > 0 && is_edge(glyph(lines, row, left - 1).unwrap_or(' ')) {
            panic!(
                "edge runs along the frame border at ({row},{}):\n{output}",
                left - 1
            );
        }
        if is_edge(glyph(lines, row, right + 1).unwrap_or(' ')) {
            panic!(
                "edge runs along the frame border at ({row},{}):\n{output}",
                right + 1
            );
        }
    }
    for col in [left, right] {
        if top > 0 && is_edge(glyph(lines, top - 1, col).unwrap_or(' ')) {
            panic!(
                "edge runs along the frame border at ({}, {col}):\n{output}",
                top - 1
            );
        }
        if is_edge(glyph(lines, bottom + 1, col).unwrap_or(' ')) {
            panic!(
                "edge runs along the frame border at ({}, {col}):\n{output}",
                bottom + 1
            );
        }
    }
}
