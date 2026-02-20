use std::collections::HashMap;

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

    for node in &layout.nodes {
        draw_node(&mut grid, node);
    }

    for edge in &layout.edges {
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_td_edge(&mut grid, from, to, edge, layout);
    }

    grid.to_string()
}

fn render_lr(layout: &GraphLayout) -> String {
    let mut grid = Grid::new(layout.width, layout.height);
    let node_map: HashMap<&str, &NodeLayout> =
        layout.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for node in &layout.nodes {
        draw_node(&mut grid, node);
    }

    for edge in &layout.edges {
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_lr_edge(&mut grid, from, to, edge);
    }

    grid.to_string()
}

fn draw_node(grid: &mut Grid, node: &NodeLayout) {
    match node.shape {
        NodeShape::Box => draw_box(grid, node.x, node.y, node.width, &node.label),
        _ => draw_box(grid, node.x, node.y, node.width, &node.label),
    }
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

        if edge_type == EdgeType::Arrow {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, '│');
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

        if edge_type == EdgeType::Arrow {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, '│');
        }
    } else {
        if let Some(ref label) = edge.label {
            let label_col = from_cx.saturating_sub(label.len() / 2);
            grid.write_str(from_below, label_col, label);
        } else {
            for row in from_below..to_above {
                grid.set(row, from_cx, '│');
            }
        }
        if edge_type == EdgeType::Arrow {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, '│');
        }
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
    let row = from.center_y;

    for col in from_right..to_left {
        grid.set(row, col, '─');
    }

    if edge.edge_type == EdgeType::Arrow {
        grid.set(row, to_left - 1, '>');
    }

    if let Some(ref label) = edge.label {
        let gap = to_left - from_right;
        let label_col = from_right + (gap.saturating_sub(label.len())) / 2;
        if row > 0 {
            grid.write_str(row - 1, label_col, label);
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
    fn render_lr_open_link() {
        let output = render_input("graph LR\n    A --- B\n");
        let expected = "\
┌───┐     ┌───┐
│ A │─────│ B │
└───┘     └───┘";
        assert_eq!(output, expected);
    }
}
