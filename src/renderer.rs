use crate::ast::*;
use crate::display_width::{display_width, line_count, split_br};
use crate::layout::*;

const BOX_TL: char = '┌';
const BOX_TR: char = '┐';
const BOX_BL: char = '└';
const BOX_BR: char = '┘';
const BOX_H: char = '─';
const BOX_V: char = '│';
const BOX_TD: char = '┬';
const BOX_TU: char = '┴';
const ARROW_R: char = '>';
const ARROW_L: char = '<';
const HEAVY_V: char = '┃';
const SELF_LOOP_ARM: usize = 4;

struct Grid {
    cells: Vec<Vec<char>>,
    width: usize,
    height: usize,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![' '; width]; height],
            width,
            height,
        }
    }

    fn set(&mut self, row: usize, col: usize, ch: char) {
        if row < self.height && col < self.width {
            if self.cells[row][col] == '\0' && col > 0 && self.cells[row][col - 1] != '\0' {
                self.cells[row][col - 1] = ' ';
            }
            self.cells[row][col] = ch;
        }
    }

    fn write_str(&mut self, row: usize, col: usize, s: &str) {
        let mut offset = 0;
        for ch in s.chars() {
            self.set(row, col + offset, ch);
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
            for j in 1..w {
                self.set(row, col + offset + j, '\0');
            }
            offset += w;
        }
    }

    fn render(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                let line: String = row.iter().filter(|&&ch| ch != '\0').collect();
                line.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn row_height(row: &Row) -> usize {
    match row {
        Row::Message(m) => 2 + line_count(&m.text),
        Row::Note(n) => 2 + line_count(&n.text),
        Row::BlockStart(_) | Row::BlockEnd(_) | Row::BlockDivider(_) | Row::Destroy(_) => 1,
    }
}

pub fn render(layout: &Layout) -> String {
    let box_height = layout
        .participants
        .iter()
        .map(|p| p.box_height)
        .max()
        .unwrap_or(3);
    let body_height: usize = layout.rows.iter().map(row_height).sum();
    let height = box_height + body_height + box_height;
    let mut grid = Grid::new(layout.total_width, height);

    draw_participant_boxes_filtered(&mut grid, layout, 0, true, &[]);

    let body_start = box_height;
    let mut y = body_start;
    let mut active_frames: Vec<&BlockRow> = Vec::new();
    let mut alive = vec![true; layout.participants.len()];
    for (i, row) in layout.rows.iter().enumerate() {
        let row_activations = layout
            .activations
            .get(i)
            .cloned()
            .unwrap_or_else(|| vec![false; layout.participants.len()]);
        let h = row_height(row);
        match row {
            Row::Message(msg) => {
                draw_lifelines_filtered(&mut grid, layout, y, h, &row_activations, &alive);
                draw_message(&mut grid, layout, msg, y, &row_activations);
                draw_frame_sides(&mut grid, layout, &active_frames, y, h);
            }
            Row::Note(note) => {
                draw_lifelines_filtered(&mut grid, layout, y, h, &row_activations, &alive);
                draw_note(&mut grid, note, y);
                draw_frame_sides(&mut grid, layout, &active_frames, y, h);
            }
            Row::BlockStart(block) => {
                draw_block_start(&mut grid, layout, block, y);
                active_frames.push(block);
            }
            Row::BlockEnd(block) => {
                active_frames.retain(|f| f.frame_left != block.frame_left || f.frame_right != block.frame_right);
                draw_block_end(&mut grid, layout, block, y);
            }
            Row::BlockDivider(block) => {
                draw_block_divider(&mut grid, layout, block, y);
            }
            Row::Destroy(destroy) => {
                draw_destroy(&mut grid, destroy, y);
                alive[destroy.participant_idx] = false;
            }
        }
        y += h;
    }

    let bottom_y = body_start + body_height;
    draw_participant_boxes_filtered(&mut grid, layout, bottom_y, false, &layout.destroyed);

    grid.render()
}

fn draw_participant_boxes_filtered(
    grid: &mut Grid,
    layout: &Layout,
    y: usize,
    is_top: bool,
    skip: &[bool],
) {
    let max_box_height = layout
        .participants
        .iter()
        .map(|p| p.box_height)
        .max()
        .unwrap_or(3);

    for (i, p) in layout.participants.iter().enumerate() {
        if skip.get(i).copied().unwrap_or(false) {
            continue;
        }
        grid.set(y, p.box_left, BOX_TL);
        for col in (p.box_left + 1)..p.box_right {
            grid.set(y, col, BOX_H);
        }
        grid.set(y, p.box_right, BOX_TR);

        let lines = split_br(&p.name);
        for (li, line) in lines.iter().enumerate() {
            let row = y + 1 + li;
            grid.set(row, p.box_left, BOX_V);
            grid.write_str(row, p.box_left + 2, line);
            grid.set(row, p.box_right, BOX_V);
        }
        // Fill remaining rows if this box is shorter than the max
        for li in lines.len()..(max_box_height - 2) {
            let row = y + 1 + li;
            grid.set(row, p.box_left, BOX_V);
            grid.set(row, p.box_right, BOX_V);
        }

        let bottom = y + max_box_height - 1;
        grid.set(bottom, p.box_left, BOX_BL);
        for col in (p.box_left + 1)..p.box_right {
            grid.set(bottom, col, BOX_H);
        }
        grid.set(bottom, p.box_right, BOX_BR);

        if is_top {
            grid.set(bottom, p.center_col, BOX_TD);
        } else {
            grid.set(y, p.center_col, BOX_TU);
        }
    }
}

fn draw_lifelines_filtered(
    grid: &mut Grid,
    layout: &Layout,
    y: usize,
    count: usize,
    activations: &[bool],
    alive: &[bool],
) {
    for (i, p) in layout.participants.iter().enumerate() {
        if !alive.get(i).copied().unwrap_or(true) {
            continue;
        }
        let ch = if activations.get(i).copied().unwrap_or(false) {
            HEAVY_V
        } else {
            BOX_V
        };
        for dy in 0..count {
            grid.set(y + dy, p.center_col, ch);
        }
    }
}

fn draw_message(
    grid: &mut Grid,
    layout: &Layout,
    msg: &MessageRow,
    y: usize,
    activations: &[bool],
) {
    if msg.from_col == msg.to_col {
        draw_self_message(grid, layout, msg, y, activations);
        return;
    }

    let (left_col, right_col) = if msg.from_col < msg.to_col {
        (msg.from_col, msg.to_col)
    } else {
        (msg.to_col, msg.from_col)
    };

    let text_col = left_col + 2;
    let lines = split_br(&msg.text);
    for (i, line) in lines.iter().enumerate() {
        grid.write_str(y + i, text_col, line);
    }

    let arrow_y = y + lines.len();

    match msg.arrow.line_style {
        LineStyle::Solid => {
            for col in (left_col + 1)..right_col {
                grid.set(arrow_y, col, BOX_H);
            }
        }
        LineStyle::Dotted => {
            for col in (left_col + 1)..right_col {
                let offset = col - (left_col + 1);
                if offset % 2 == 0 {
                    grid.set(arrow_y, col, BOX_H);
                } else {
                    grid.set(arrow_y, col, ' ');
                }
            }
        }
    }

    match msg.direction {
        Direction::LeftToRight => {
            grid.set(arrow_y, right_col - 1, arrow_head_char(&msg.arrow));
        }
        Direction::RightToLeft => {
            grid.set(arrow_y, left_col + 1, reverse_arrow_head_char(&msg.arrow));
            if right_col >= 2 {
                grid.set(arrow_y, right_col - 1, BOX_H);
            }
        }
    }

    let left_idx = layout
        .participants
        .iter()
        .position(|p| p.center_col == left_col);
    let right_idx = layout
        .participants
        .iter()
        .position(|p| p.center_col == right_col);

    let left_ch = if left_idx.is_some_and(|i| activations.get(i).copied().unwrap_or(false)) {
        HEAVY_V
    } else {
        BOX_V
    };
    let right_ch = if right_idx.is_some_and(|i| activations.get(i).copied().unwrap_or(false)) {
        HEAVY_V
    } else {
        BOX_V
    };

    grid.set(arrow_y, left_col, left_ch);
    grid.set(arrow_y, right_col, right_ch);
}

fn draw_self_message(
    grid: &mut Grid,
    layout: &Layout,
    msg: &MessageRow,
    y: usize,
    activations: &[bool],
) {
    let center = msg.from_col;
    let arm_end = center + SELF_LOOP_ARM;
    let lines = split_br(&msg.text);
    let text_rows = lines.len();

    // text lines
    for (i, line) in lines.iter().enumerate() {
        grid.write_str(y + i, center + 2, line);
    }

    // outgoing arm ──┐
    let arm_y = y + text_rows;
    for col in (center + 1)..arm_end {
        grid.set(arm_y, col, BOX_H);
    }
    grid.set(arm_y, arm_end, BOX_TR);

    // return arm <─┘
    let return_y = arm_y + 1;
    grid.set(return_y, center + 1, reverse_arrow_head_char(&msg.arrow));
    for col in (center + 2)..arm_end {
        grid.set(return_y, col, BOX_H);
    }
    grid.set(return_y, arm_end, BOX_BR);

    // Restore lifeline at center
    let idx = layout
        .participants
        .iter()
        .position(|p| p.center_col == center);
    let ch = if idx.is_some_and(|i| activations.get(i).copied().unwrap_or(false)) {
        HEAVY_V
    } else {
        BOX_V
    };
    let h = 2 + text_rows;
    for dy in 0..h {
        grid.set(y + dy, center, ch);
    }
}

fn draw_note(grid: &mut Grid, note: &NoteRow, y: usize) {
    let left = note.box_left;
    let right = note.box_right;
    let lines = split_br(&note.text);

    grid.set(y, left, BOX_TL);
    for col in (left + 1)..right {
        grid.set(y, col, BOX_H);
    }
    grid.set(y, right, BOX_TR);

    for (i, line) in lines.iter().enumerate() {
        let row = y + 1 + i;
        grid.set(row, left, BOX_V);
        for col in (left + 1)..right {
            grid.set(row, col, ' ');
        }
        grid.write_str(row, left + 2, line);
        grid.set(row, right, BOX_V);
    }

    let bottom = y + 1 + lines.len();
    grid.set(bottom, left, BOX_BL);
    for col in (left + 1)..right {
        grid.set(bottom, col, BOX_H);
    }
    grid.set(bottom, right, BOX_BR);
}

const CROSS: char = '┼';

fn draw_block_start(grid: &mut Grid, layout: &Layout, block: &BlockRow, y: usize) {
    grid.set(y, block.frame_left, BOX_TL);
    for col in (block.frame_left + 1)..block.frame_right {
        grid.set(y, col, BOX_H);
    }
    grid.set(y, block.frame_right, BOX_TR);

    // Write label
    grid.write_str(y, block.frame_left + 2, &block.label);

    // Draw ┼ at lifeline intersections
    for p in &layout.participants {
        if p.center_col > block.frame_left && p.center_col < block.frame_right {
            // Only draw ┼ if it's not covered by the label text
            let label_end = block.frame_left + 2 + display_width(&block.label);
            if p.center_col > label_end {
                grid.set(y, p.center_col, CROSS);
            }
        }
    }
}

fn draw_block_end(grid: &mut Grid, layout: &Layout, block: &BlockRow, y: usize) {
    grid.set(y, block.frame_left, BOX_BL);
    for col in (block.frame_left + 1)..block.frame_right {
        grid.set(y, col, BOX_H);
    }
    grid.set(y, block.frame_right, BOX_BR);

    // Draw ┼ at lifeline intersections
    for p in &layout.participants {
        if p.center_col > block.frame_left && p.center_col < block.frame_right {
            grid.set(y, p.center_col, CROSS);
        }
    }
}

const BOX_DIVIDER_L: char = '├';
const BOX_DIVIDER_R: char = '┤';

fn draw_block_divider(grid: &mut Grid, layout: &Layout, block: &BlockRow, y: usize) {
    grid.set(y, block.frame_left, BOX_DIVIDER_L);
    for col in (block.frame_left + 1)..block.frame_right {
        grid.set(y, col, BOX_H);
    }
    grid.set(y, block.frame_right, BOX_DIVIDER_R);

    // Write label
    grid.write_str(y, block.frame_left + 2, &block.label);

    // Draw ┼ at lifeline intersections
    for p in &layout.participants {
        if p.center_col > block.frame_left && p.center_col < block.frame_right {
            let label_end = block.frame_left + 2 + display_width(&block.label);
            if p.center_col > label_end {
                grid.set(y, p.center_col, CROSS);
            }
        }
    }
}

fn draw_frame_sides(
    grid: &mut Grid,
    _layout: &Layout,
    active_frames: &[&BlockRow],
    y: usize,
    height: usize,
) {
    for frame in active_frames {
        for dy in 0..height {
            grid.set(y + dy, frame.frame_left, BOX_V);
            grid.set(y + dy, frame.frame_right, BOX_V);
        }
    }
}

fn arrow_head_char(arrow: &Arrow) -> char {
    match arrow.head {
        ArrowHead::None | ArrowHead::Arrowhead => ARROW_R,
        ArrowHead::Cross => 'x',
        ArrowHead::Open => ARROW_R,
    }
}

fn reverse_arrow_head_char(arrow: &Arrow) -> char {
    match arrow.head {
        ArrowHead::None | ArrowHead::Arrowhead => ARROW_L,
        ArrowHead::Cross => 'x',
        ArrowHead::Open => ARROW_L,
    }
}

fn draw_destroy(grid: &mut Grid, destroy: &DestroyRow, y: usize) {
    grid.set(y, destroy.col, 'X');
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn grid_basic_operations() {
        let mut grid = Grid::new(10, 3);
        grid.write_str(1, 2, "hello");
        let output = grid.render();
        assert!(output.contains("hello"));
    }

    #[test]
    fn grid_set_character() {
        let mut grid = Grid::new(5, 2);
        grid.set(0, 2, 'X');
        let output = grid.render();
        assert!(output.contains("X"));
    }

    #[test]
    fn grid_write_wide_chars_correct_offset() {
        let mut grid = Grid::new(10, 1);
        grid.write_str(0, 0, "テス");
        grid.set(0, 4, 'C');
        let output = grid.render();
        assert_eq!(output, "テスC");
    }

    #[test]
    fn grid_set_overwrites_wide_char_continuation() {
        let mut grid = Grid::new(10, 1);
        grid.write_str(0, 0, "テスト");
        // Overwrite continuation marker of ス (at col 3) with │
        grid.set(0, 3, '│');
        let output = grid.render();
        // ス's base at col 2 should be cleared to space
        assert_eq!(output, "テ │ト");
    }

    #[test]
    fn grid_trims_trailing_spaces() {
        let mut grid = Grid::new(10, 2);
        grid.write_str(0, 0, "hi");
        let output = grid.render();
        let first_line = output.lines().next().unwrap();
        assert_eq!(first_line, "hi");
    }

    #[test]
    fn render_two_participants_basic() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi!\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);

        assert!(output.contains("Alice"), "output should contain Alice");
        assert!(output.contains("Bob"), "output should contain Bob");
        assert!(output.contains("Hello"), "output should contain Hello");
        assert!(output.contains("Hi!"), "output should contain Hi!");
        assert!(output.contains("┌"), "output should contain box drawing chars");
        assert!(output.contains("│"), "output should contain lifeline");
    }

    #[test]
    fn render_has_top_and_bottom_boxes() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);

        let alice_count = output.matches("Alice").count();
        assert_eq!(alice_count, 2, "Alice should appear in top and bottom boxes");

        let bob_count = output.matches("Bob").count();
        assert_eq!(bob_count, 2, "Bob should appear in top and bottom boxes");
    }

    #[test]
    fn render_arrow_direction() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);

        assert!(output.contains(">│") || output.contains(">"), "should have right arrow");
    }

    #[test]
    fn render_activation_uses_heavy_line() {
        let input = "sequenceDiagram\n    Alice->>+Bob: Hello\n    Bob-->>-Alice: Hi!\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);

        assert!(output.contains('┃'), "active lifeline should use heavy vertical");
    }

    #[test]
    fn render_inactive_uses_normal_line() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);

        let body = output.lines().skip(3).take(3).collect::<Vec<_>>().join("\n");
        assert!(!body.contains('┃'), "inactive lifeline should not use heavy vertical");
    }

    #[test]
    fn render_multiline_note() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Note right of Bob: Line1<br/>Line2\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);
        assert!(output.contains("Line1"), "output should contain Line1");
        assert!(output.contains("Line2"), "output should contain Line2");
        let lines: Vec<&str> = output.lines().collect();
        let l1 = lines.iter().position(|l| l.contains("Line1")).unwrap();
        let l2 = lines.iter().position(|l| l.contains("Line2")).unwrap();
        assert_eq!(l2, l1 + 1, "Line2 should be on the line after Line1");
    }

    #[test]
    fn render_multiline_message() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello<br/>World\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);
        // Message text should appear on two lines
        assert!(output.contains("Hello"), "output should contain Hello");
        assert!(output.contains("World"), "output should contain World");
        // They should be on separate lines
        let lines: Vec<&str> = output.lines().collect();
        let hello_line = lines.iter().position(|l| l.contains("Hello")).unwrap();
        let world_line = lines.iter().position(|l| l.contains("World")).unwrap();
        assert_eq!(
            world_line,
            hello_line + 1,
            "World should be on the line after Hello"
        );
    }

    #[test]
    fn render_participant_name_with_br_tag() {
        let input = "sequenceDiagram\n    participant A as PlaceInfo<br/>Details\n    A->>A: test\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);
        // <br/> should NOT appear literally in the output
        assert!(
            !output.contains("<br/>"),
            "output should not contain literal <br/>: {output}"
        );
        // Both lines of the name should be rendered
        assert!(output.contains("PlaceInfo"), "should contain PlaceInfo");
        assert!(output.contains("Details"), "should contain Details");
    }

    #[test]
    fn render_self_message_as_loop() {
        let input = "sequenceDiagram\n    A->>B: Hello\n    B->>B: self\n";
        let diagram = crate::parser::parse_diagram(input).unwrap();
        let layout = crate::layout::compute(&diagram).unwrap();
        let output = render(&layout);
        assert!(output.contains("self"), "should contain self-message text");
        assert!(output.contains("──┐"), "self-message should have loop out");
        assert!(output.contains("┘"), "self-message should have return corner");
    }
}
