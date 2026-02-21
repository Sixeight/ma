use std::collections::HashMap;

use crate::display_width::display_width;
use crate::graph_ast::{Direction, EdgeType, NodeShape};
use crate::graph_layout::*;

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

    fn set_merge(&mut self, row: usize, col: usize, ch: char) {
        if row < self.height && col < self.width {
            let existing = self.cells[row][col];
            let merged = merge_box_drawing(existing, ch);
            self.set(row, col, merged);
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

pub fn render(layout: &GraphLayout) -> String {
    match layout.direction {
        Direction::TopDown => render_td(layout),
        Direction::LeftRight => render_lr(layout),
    }
}

fn render_td(layout: &GraphLayout) -> String {
    let mut grid = Grid::new(layout.width, layout.height);
    let node_map: HashMap<&str, &NodeLayout> =
        layout.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for sg in &layout.subgraphs {
        draw_subgraph(&mut grid, sg);
    }

    for node in &layout.nodes {
        draw_node(&mut grid, node);
    }

    for edge in &layout.edges {
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_td_edge(&mut grid, from, to, edge, layout);
    }

    grid.render()
}

fn render_lr(layout: &GraphLayout) -> String {
    let mut grid = Grid::new(layout.width, layout.height);
    let node_map: HashMap<&str, &NodeLayout> =
        layout.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for sg in &layout.subgraphs {
        draw_subgraph(&mut grid, sg);
    }

    for node in &layout.nodes {
        draw_node(&mut grid, node);
    }

    for edge in &layout.edges {
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_lr_edge(&mut grid, from, to, edge);
    }

    grid.render()
}

fn draw_node(grid: &mut Grid, node: &NodeLayout) {
    match node.shape {
        NodeShape::Box => draw_box(grid, node.x, node.y, node.width, &node.label),
        NodeShape::Round | NodeShape::Circle => {
            draw_round(grid, node.x, node.y, node.width, &node.label)
        }
        NodeShape::Diamond => draw_diamond(grid, node.x, node.y, node.width, &node.label),
    }
}

fn draw_subgraph(grid: &mut Grid, sg: &SubgraphLayout) {
    let x = sg.x;
    let y = sg.y;
    let w = sg.width;
    let h = sg.height;

    grid.set(y, x, '┌');
    grid.set(y, x + 1, '─');
    grid.set(y, x + 2, ' ');
    grid.write_str(y, x + 3, &sg.label);
    grid.set(y, x + 3 + display_width(&sg.label), ' ');
    for col in (x + 4 + display_width(&sg.label))..(x + w - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + w - 1, '┐');

    for row in (y + 1)..(y + h - 1) {
        grid.set(row, x, '│');
        grid.set(row, x + w - 1, '│');
    }

    grid.set(y + h - 1, x, '└');
    for col in (x + 1)..(x + w - 1) {
        grid.set(y + h - 1, col, '─');
    }
    grid.set(y + h - 1, x + w - 1, '┘');
}

fn draw_box(grid: &mut Grid, x: usize, y: usize, width: usize, label: &str) {
    grid.set(y, x, '┌');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + width - 1, '┐');

    grid.set(y + 1, x, '│');
    grid.write_str(y + 1, x + 2, label);
    grid.set(y + 1, x + width - 1, '│');

    grid.set(y + 2, x, '└');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y + 2, col, '─');
    }
    grid.set(y + 2, x + width - 1, '┘');
}

fn draw_round(grid: &mut Grid, x: usize, y: usize, width: usize, label: &str) {
    grid.set(y, x, '╭');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + width - 1, '╮');

    grid.set(y + 1, x, '│');
    let inner = width - 2;
    let pad_left = (inner - display_width(label)) / 2;
    grid.write_str(y + 1, x + 1 + pad_left, label);
    grid.set(y + 1, x + width - 1, '│');

    grid.set(y + 2, x, '╰');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y + 2, col, '─');
    }
    grid.set(y + 2, x + width - 1, '╯');
}

fn draw_diamond(grid: &mut Grid, x: usize, y: usize, width: usize, label: &str) {
    grid.set(y, x, '╱');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + width - 1, '╲');

    grid.set(y + 1, x, '│');
    grid.write_str(y + 1, x + 2, label);
    grid.set(y + 1, x + width - 1, '│');

    grid.set(y + 2, x, '╲');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y + 2, col, '─');
    }
    grid.set(y + 2, x + width - 1, '╱');
}

const DIR_L: u8 = 1;
const DIR_R: u8 = 2;
const DIR_U: u8 = 4;
const DIR_D: u8 = 8;

fn box_connections(ch: char) -> u8 {
    match ch {
        '─' | '═' | '╌' => DIR_L | DIR_R,
        '│' | '║' | '┊' => DIR_U | DIR_D,
        '┌' => DIR_R | DIR_D,
        '┐' => DIR_L | DIR_D,
        '└' => DIR_R | DIR_U,
        '┘' => DIR_L | DIR_U,
        '┬' => DIR_L | DIR_R | DIR_D,
        '┴' => DIR_L | DIR_R | DIR_U,
        '├' => DIR_U | DIR_D | DIR_R,
        '┤' => DIR_U | DIR_D | DIR_L,
        '┼' => DIR_L | DIR_R | DIR_U | DIR_D,
        _ => 0,
    }
}

fn connections_to_char(conn: u8) -> Option<char> {
    match conn {
        c if c == DIR_L | DIR_R => Some('─'),
        c if c == DIR_U | DIR_D => Some('│'),
        c if c == DIR_R | DIR_D => Some('┌'),
        c if c == DIR_L | DIR_D => Some('┐'),
        c if c == DIR_R | DIR_U => Some('└'),
        c if c == DIR_L | DIR_U => Some('┘'),
        c if c == DIR_L | DIR_R | DIR_D => Some('┬'),
        c if c == DIR_L | DIR_R | DIR_U => Some('┴'),
        c if c == DIR_U | DIR_D | DIR_R => Some('├'),
        c if c == DIR_U | DIR_D | DIR_L => Some('┤'),
        c if c == DIR_L | DIR_R | DIR_U | DIR_D => Some('┼'),
        _ => None,
    }
}

fn merge_box_drawing(existing: char, new_char: char) -> char {
    let ec = box_connections(existing);
    let nc = box_connections(new_char);
    if ec == 0 {
        return new_char;
    }
    connections_to_char(ec | nc).unwrap_or(new_char)
}

fn td_vertical_connector(edge_type: EdgeType) -> char {
    match edge_type {
        EdgeType::DottedArrow | EdgeType::DottedLink => '┊',
        EdgeType::ThickArrow | EdgeType::ThickLink => '║',
        _ => '│',
    }
}

fn has_arrow_head(edge_type: EdgeType) -> bool {
    matches!(
        edge_type,
        EdgeType::Arrow | EdgeType::DottedArrow | EdgeType::ThickArrow
    )
}

fn is_subgraph_border_row(layout: &GraphLayout, row: usize) -> bool {
    layout
        .subgraphs
        .iter()
        .any(|sg| row == sg.y || row == sg.y + sg.height - 1)
}

fn draw_td_edge(
    grid: &mut Grid,
    from: &NodeLayout,
    to: &NodeLayout,
    edge: &EdgeLayout,
    layout: &GraphLayout,
) {
    let edge_type = edge.edge_type;
    let from_cx = from.center_x;
    let to_cx = to.center_x;
    let bottom_row = from.y + from.height - 1;
    let from_below = from.y + from.height;
    let to_above = to.y - 1;

    grid.set(bottom_row, from_cx, '┬');

    let sibling_count = layout.edges.iter().filter(|e| e.from_id == from.id).count();
    let parent_count = layout.edges.iter().filter(|e| e.to_id == to.id).count();

    if sibling_count > 1 {
        let child_centers: Vec<usize> = layout
            .edges
            .iter()
            .filter(|e| e.from_id == from.id)
            .filter_map(|e| layout.nodes.iter().find(|n| n.id == e.to_id))
            .map(|n| n.center_x)
            .collect();
        let min_cx = *child_centers.iter().min().unwrap();
        let max_cx = *child_centers.iter().max().unwrap();

        grid.set(from_below, min_cx, '┌');
        for col in (min_cx + 1)..max_cx {
            grid.set(from_below, col, '─');
        }
        grid.set(from_below, max_cx, '┐');
        grid.set(from_below, from_cx, '┴');

        if has_arrow_head(edge_type) {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, td_vertical_connector(edge_type));
        }
    } else if parent_count > 1 {
        let parent_centers: Vec<usize> = layout
            .edges
            .iter()
            .filter(|e| e.to_id == to.id)
            .filter_map(|e| layout.nodes.iter().find(|n| n.id == e.from_id))
            .map(|n| n.center_x)
            .collect();
        let min_cx = *parent_centers.iter().min().unwrap();
        let max_cx = *parent_centers.iter().max().unwrap();

        grid.set(from_below, min_cx, '└');
        for col in (min_cx + 1)..max_cx {
            grid.set(from_below, col, '─');
        }
        grid.set(from_below, max_cx, '┘');
        grid.set(from_below, to_cx, '┬');

        if has_arrow_head(edge_type) {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, td_vertical_connector(edge_type));
        }
    } else {
        let vert = td_vertical_connector(edge_type);
        if let Some(ref label) = edge.label {
            let label_col = from_cx.saturating_sub(display_width(label) / 2);
            grid.write_str(from_below, label_col, label);
        } else {
            for row in from_below..to_above {
                if !is_subgraph_border_row(layout, row) {
                    grid.set(row, from_cx, vert);
                }
            }
        }
        if !is_subgraph_border_row(layout, to_above) {
            if has_arrow_head(edge_type) {
                grid.set(to_above, to_cx, '▼');
            } else {
                grid.set(to_above, to_cx, vert);
            }
        }
    }
}

fn lr_horizontal_connector(edge_type: EdgeType) -> char {
    match edge_type {
        EdgeType::DottedArrow | EdgeType::DottedLink => '╌',
        EdgeType::ThickArrow | EdgeType::ThickLink => '═',
        _ => '─',
    }
}

fn draw_lr_edge(
    grid: &mut Grid,
    from: &NodeLayout,
    to: &NodeLayout,
    edge: &EdgeLayout,
) {
    let from_right = from.x + from.width;
    let to_left = to.x;
    let horiz = lr_horizontal_connector(edge.edge_type);

    if from.center_y == to.center_y {
        // Straight horizontal
        let row = from.center_y;
        for col in from_right..to_left {
            grid.set_merge(row, col, horiz);
        }
        if has_arrow_head(edge.edge_type) {
            grid.set(row, to_left - 1, '>');
        }
        if let Some(ref label) = edge.label {
            let gap = to_left - from_right;
            let label_col = from_right + (gap.saturating_sub(display_width(label))) / 2;
            if row > 0 {
                grid.write_str(row - 1, label_col, label);
            }
        }
    } else {
        // L-shaped routing: horizontal → corner → vertical → corner → horizontal
        let mid_col = from_right + (to_left - from_right) / 2;
        let vert = td_vertical_connector(edge.edge_type);

        // Horizontal from source to midpoint
        for col in from_right..mid_col {
            grid.set(from.center_y, col, horiz);
        }

        // Corners and vertical segment
        if from.center_y < to.center_y {
            grid.set_merge(from.center_y, mid_col, '┐');
            for row in (from.center_y + 1)..to.center_y {
                grid.set_merge(row, mid_col, vert);
            }
            grid.set_merge(to.center_y, mid_col, '└');
        } else {
            grid.set_merge(from.center_y, mid_col, '┘');
            for row in (to.center_y + 1)..from.center_y {
                grid.set_merge(row, mid_col, vert);
            }
            grid.set_merge(to.center_y, mid_col, '┌');
        }

        // Horizontal from midpoint to target
        for col in (mid_col + 1)..to_left {
            grid.set(to.center_y, col, horiz);
        }
        if has_arrow_head(edge.edge_type) {
            grid.set(to.center_y, to_left - 1, '>');
        }

        // Label on the source-side horizontal segment
        if let Some(ref label) = edge.label {
            let gap = mid_col.saturating_sub(from_right);
            if gap > 0 {
                let label_col = from_right + (gap.saturating_sub(display_width(label))) / 2;
                if from.center_y > 0 {
                    grid.write_str(from.center_y - 1, label_col, label);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_parser::parse_graph;
    use pretty_assertions::assert_eq;

    fn render_input(input: &str) -> String {
        let diagram = parse_graph(input).unwrap();
        let layout = crate::graph_layout::compute(&diagram).unwrap();
        render(&layout)
    }

    #[test]
    fn render_round_node() {
        let output = render_input("graph TD\n    A(Hello)\n");
        let expected = "\
╭───────╮
│ Hello │
╰───────╯";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_diamond_node() {
        let output = render_input("graph TD\n    A{Hello}\n");
        let expected = "\
╱───────╲
│ Hello │
╲───────╱";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_circle_node() {
        let output = render_input("graph TD\n    A((Hello))\n");
        let expected = "\
╭───────────╮
│   Hello   │
╰───────────╯";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_single_node() {
        let output = render_input("graph TD\n    A[Hello]\n");
        let expected = "\
┌───────┐
│ Hello │
└───────┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_linear_chain() {
        let output = render_input("graph TD\n    A[Start] --> B[End]\n");
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
    fn render_lr_linear_chain() {
        let output = render_input("graph LR\n    A[Start] --> B[End]\n");
        let expected = "\
┌───────┐     ┌─────┐
│ Start │────>│ End │
└───────┘     └─────┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_fan_out() {
        let output = render_input("graph TD\n    A --> B\n    A --> C\n");
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
    fn render_td_fan_in() {
        let output = render_input("graph TD\n    A --> C\n    B --> C\n");
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
    fn render_td_edge_label() {
        let output = render_input("graph TD\n    A -->|yes| B\n");
        let expected = "\
┌───┐
│ A │
└─┬─┘
 yes
  ▼
┌───┐
│ B │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_open_link() {
        let output = render_input("graph TD\n    A --- B\n");
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
    fn render_lr_edge_label() {
        let output = render_input("graph LR\n    A -->|yes| B\n");
        let expected = "\
┌───┐ yes ┌───┐
│ A │────>│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_dotted_arrow() {
        let output = render_input("graph TD\n    A -.-> B\n");
        let expected = "\
┌───┐
│ A │
└─┬─┘
  ┊
  ▼
┌───┐
│ B │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_dotted_link() {
        let output = render_input("graph TD\n    A -.- B\n");
        let expected = "\
┌───┐
│ A │
└─┬─┘
  ┊
  ┊
┌───┐
│ B │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_thick_arrow() {
        let output = render_input("graph TD\n    A ==> B\n");
        let expected = "\
┌───┐
│ A │
└─┬─┘
  ║
  ▼
┌───┐
│ B │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_thick_link() {
        let output = render_input("graph TD\n    A === B\n");
        let expected = "\
┌───┐
│ A │
└─┬─┘
  ║
  ║
┌───┐
│ B │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_lr_dotted_arrow() {
        let output = render_input("graph LR\n    A -.-> B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │╌╌╌╌>│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_lr_dotted_link() {
        let output = render_input("graph LR\n    A -.- B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │╌╌╌╌╌│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_lr_thick_arrow() {
        let output = render_input("graph LR\n    A ==> B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │════>│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_lr_thick_link() {
        let output = render_input("graph LR\n    A === B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │═════│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_lr_open_link() {
        let output = render_input("graph LR\n    A --- B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │─────│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_subgraph_single_node() {
        let output = render_input("graph TD\n    subgraph Group\n        A\n    end\n");
        assert!(output.contains("┌─ Group"), "top border with title");
        assert!(output.contains("│ A │"), "node inside subgraph");
        assert!(output.contains('└'), "bottom border");
    }

    #[test]
    fn render_td_subgraph_with_edge() {
        let output = render_input(
            "graph TD\n    subgraph Backend\n        A[API] --> B[DB]\n    end\n",
        );
        assert!(output.contains("┌─ Backend"), "top border with title");
        assert!(output.contains("│ API │"), "node A");
        assert!(output.contains("│ DB │"), "node B");
        assert!(output.contains('▼'), "arrow");

        let lines: Vec<&str> = output.lines().collect();
        let first_line = lines[0];
        let last_line = lines[lines.len() - 1];
        assert!(first_line.contains('┌'), "first line has top-left corner");
        assert!(first_line.contains('┐'), "first line has top-right corner");
        assert!(last_line.contains('└'), "last line has bottom-left corner");
        assert!(last_line.contains('┘'), "last line has bottom-right corner");
    }

    #[test]
    fn render_lr_fan_out_edges_reach_targets() {
        let output = render_input("graph LR\n    A --> B\n    A --> C\n");
        assert!(
            output.contains('>'),
            "should have at least one arrow head"
        );
        let lines: Vec<&str> = output.lines().collect();
        // B and C should both appear
        assert!(output.contains("B"), "B should be rendered");
        assert!(output.contains("C"), "C should be rendered");
        // Both B and C should have an incoming '>' on their line
        let b_line = lines.iter().find(|l| l.contains("│ B │")).expect("B node line");
        let c_line = lines.iter().find(|l| l.contains("│ C │")).expect("C node line");
        assert!(b_line.contains('>'), "B should have incoming arrow: {b_line}");
        assert!(c_line.contains('>'), "C should have incoming arrow: {c_line}");
    }

    #[test]
    fn render_lr_fan_out_has_vertical_routing() {
        let output = render_input("graph LR\n    A --> B\n    A --> C\n");
        // L-shaped routing should produce corner characters
        let has_corner = output.contains('┐')
            || output.contains('┘')
            || output.contains('└')
            || output.contains('┌');
        assert!(has_corner, "L-shaped routing should have corners:\n{output}");
    }
}
