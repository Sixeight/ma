use crate::ast::*;
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
            self.cells[row][col] = ch;
        }
    }

    fn write_str(&mut self, row: usize, col: usize, s: &str) {
        for (i, ch) in s.chars().enumerate() {
            self.set(row, col + i, ch);
        }
    }

    fn to_string(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                let line: String = row.iter().collect();
                line.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn render(layout: &Layout) -> String {
    let rows_per_message = 3;
    let box_height = 3;
    let height = box_height + layout.rows.len() * rows_per_message + box_height;
    let mut grid = Grid::new(layout.total_width, height);

    draw_participant_boxes(&mut grid, layout, 0, true);

    let body_start = box_height;
    for (i, row) in layout.rows.iter().enumerate() {
        let y = body_start + i * rows_per_message;
        let row_activations = layout
            .activations
            .get(i)
            .cloned()
            .unwrap_or_else(|| vec![false; layout.participants.len()]);
        match row {
            Row::Message(msg) => {
                draw_lifelines(&mut grid, layout, y, rows_per_message, &row_activations);
                draw_message(&mut grid, layout, msg, y, &row_activations);
            }
        }
    }

    let bottom_y = body_start + layout.rows.len() * rows_per_message;
    draw_participant_boxes(&mut grid, layout, bottom_y, false);

    grid.to_string()
}

fn draw_participant_boxes(grid: &mut Grid, layout: &Layout, y: usize, is_top: bool) {
    for p in &layout.participants {
        grid.set(y, p.box_left, BOX_TL);
        for col in (p.box_left + 1)..p.box_right {
            grid.set(y, col, BOX_H);
        }
        grid.set(y, p.box_right, BOX_TR);

        grid.set(y + 1, p.box_left, BOX_V);
        grid.write_str(y + 1, p.box_left + 2, &p.name);
        grid.set(y + 1, p.box_right, BOX_V);

        grid.set(y + 2, p.box_left, BOX_BL);
        for col in (p.box_left + 1)..p.box_right {
            grid.set(y + 2, col, BOX_H);
        }
        grid.set(y + 2, p.box_right, BOX_BR);

        if is_top {
            grid.set(y + 2, p.center_col, BOX_TD);
        } else {
            grid.set(y, p.center_col, BOX_TU);
        }
    }
}

fn draw_lifelines(
    grid: &mut Grid,
    layout: &Layout,
    y: usize,
    count: usize,
    activations: &[bool],
) {
    for (i, p) in layout.participants.iter().enumerate() {
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
    let (left_col, right_col) = if msg.from_col < msg.to_col {
        (msg.from_col, msg.to_col)
    } else {
        (msg.to_col, msg.from_col)
    };

    let text_col = left_col + 2;
    grid.write_str(y, text_col, &msg.text);

    let arrow_y = y + 1;

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

    let left_ch = if left_idx.map_or(false, |i| activations.get(i).copied().unwrap_or(false)) {
        HEAVY_V
    } else {
        BOX_V
    };
    let right_ch = if right_idx.map_or(false, |i| activations.get(i).copied().unwrap_or(false)) {
        HEAVY_V
    } else {
        BOX_V
    };

    grid.set(arrow_y, left_col, left_ch);
    grid.set(arrow_y, right_col, right_ch);
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn grid_basic_operations() {
        let mut grid = Grid::new(10, 3);
        grid.write_str(1, 2, "hello");
        let output = grid.to_string();
        assert!(output.contains("hello"));
    }

    #[test]
    fn grid_set_character() {
        let mut grid = Grid::new(5, 2);
        grid.set(0, 2, 'X');
        let output = grid.to_string();
        assert!(output.contains("X"));
    }

    #[test]
    fn grid_trims_trailing_spaces() {
        let mut grid = Grid::new(10, 2);
        grid.write_str(0, 0, "hi");
        let output = grid.to_string();
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
}
