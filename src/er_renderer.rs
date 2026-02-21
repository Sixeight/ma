use std::collections::HashMap;

use crate::er_layout::*;

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

pub fn render(layout: &ErLayout) -> String {
    let mut grid = Grid::new(layout.width, layout.height);

    let node_map: HashMap<&str, &ErNodeLayout> = layout
        .nodes
        .iter()
        .map(|n| (n.name.as_str(), n))
        .collect();

    for node in &layout.nodes {
        draw_box(&mut grid, node);
    }

    for edge in &layout.edges {
        if let (Some(from), Some(to)) = (node_map.get(edge.from.as_str()), node_map.get(edge.to.as_str())) {
            draw_er_edge(&mut grid, from, to, &edge.label);
        }
    }

    grid.to_string()
}

fn draw_box(grid: &mut Grid, node: &ErNodeLayout) {
    let x = node.x;
    let y = node.y;
    let w = node.width;

    grid.set(y, x, '┌');
    for col in (x + 1)..(x + w - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + w - 1, '┐');

    grid.set(y + 1, x, '│');
    grid.write_str(y + 1, x + 2, &node.name);
    grid.set(y + 1, x + w - 1, '│');

    grid.set(y + 2, x, '└');
    for col in (x + 1)..(x + w - 1) {
        grid.set(y + 2, col, '─');
    }
    grid.set(y + 2, x + w - 1, '┘');
}

fn draw_er_edge(grid: &mut Grid, from: &ErNodeLayout, to: &ErNodeLayout, label: &str) {
    let from_right = from.x + from.width;
    let to_left = to.x;
    let row = from.center_y;

    for col in from_right..to_left {
        grid.set(row, col, '─');
    }

    let gap = to_left - from_right;
    if gap > label.len() {
        let label_col = from_right + (gap - label.len()) / 2;
        grid.write_str(row, label_col, label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::er_ast::*;
    use crate::er_layout;
    use pretty_assertions::assert_eq;

    #[test]
    fn render_single_relationship() {
        let diagram = ErDiagram {
            entities: vec!["A".into(), "B".into()],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                left_card: Cardinality::ExactlyOne,
                right_card: Cardinality::ExactlyOne,
                label: "r1".into(),
            }],
        };
        let layout = er_layout::compute(&diagram).unwrap();
        let output = render(&layout);
        let expected = "\
┌───┐      ┌───┐
│ A │──r1──│ B │
└───┘      └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_chain() {
        let diagram = ErDiagram {
            entities: vec!["CUSTOMER".into(), "ORDER".into(), "LINE-ITEM".into()],
            relationships: vec![
                Relationship {
                    from: "CUSTOMER".into(),
                    to: "ORDER".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::ZeroOrMany,
                    label: "places".into(),
                },
                Relationship {
                    from: "ORDER".into(),
                    to: "LINE-ITEM".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::OneOrMany,
                    label: "contains".into(),
                },
            ],
        };
        let layout = er_layout::compute(&diagram).unwrap();
        let output = render(&layout);
        assert!(output.contains("CUSTOMER"), "should contain CUSTOMER");
        assert!(output.contains("ORDER"), "should contain ORDER");
        assert!(output.contains("LINE-ITEM"), "should contain LINE-ITEM");
        assert!(output.contains("places"), "should contain places label");
        assert!(output.contains("contains"), "should contain contains label");
    }
}
