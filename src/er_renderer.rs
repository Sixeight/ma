use std::collections::HashMap;

use crate::display_width::display_width;
use crate::er_ast::Cardinality;
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
            draw_er_edge(&mut grid, from, to, &edge.label, edge.left_card, edge.right_card);
        }
    }

    grid.render()
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

    if node.attributes.is_empty() {
        grid.set(y + 2, x, '└');
        for col in (x + 1)..(x + w - 1) {
            grid.set(y + 2, col, '─');
        }
        grid.set(y + 2, x + w - 1, '┘');
    } else {
        // Separator line
        let sep_y = y + 2;
        grid.set(sep_y, x, '├');
        for col in (x + 1)..(x + w - 1) {
            grid.set(sep_y, col, '─');
        }
        grid.set(sep_y, x + w - 1, '┤');

        // Attribute rows
        for (i, attr) in node.attributes.iter().enumerate() {
            let row = sep_y + 1 + i;
            grid.set(row, x, '│');
            let text = if let Some(ref key) = attr.key {
                format!("{} {} {}", attr.attr_type, attr.name, key)
            } else {
                format!("{} {}", attr.attr_type, attr.name)
            };
            grid.write_str(row, x + 2, &text);
            grid.set(row, x + w - 1, '│');
        }

        // Bottom border
        let bottom_y = sep_y + 1 + node.attributes.len();
        grid.set(bottom_y, x, '└');
        for col in (x + 1)..(x + w - 1) {
            grid.set(bottom_y, col, '─');
        }
        grid.set(bottom_y, x + w - 1, '┘');
    }
}

fn draw_er_edge(
    grid: &mut Grid,
    from: &ErNodeLayout,
    to: &ErNodeLayout,
    label: &str,
    left_card: Cardinality,
    right_card: Cardinality,
) {
    let from_right = from.x + from.width;
    let to_left = to.x;
    let row = from.center_y;

    for col in from_right..to_left {
        grid.set(row, col, '─');
    }

    let left_sym = left_cardinality_str(left_card);
    grid.write_str(row, from_right, left_sym);

    let right_sym = right_cardinality_str(right_card);
    if to_left >= 2 {
        grid.write_str(row, to_left - 2, right_sym);
    }

    let gap = to_left - from_right;
    if gap > display_width(label) {
        let label_col = from_right + (gap - display_width(label)) / 2;
        grid.write_str(row, label_col, label);
    }
}

fn left_cardinality_str(card: Cardinality) -> &'static str {
    match card {
        Cardinality::ExactlyOne => "||",
        Cardinality::ZeroOrOne => "o|",
        Cardinality::OneOrMany => "}|",
        Cardinality::ZeroOrMany => "}o",
    }
}

fn right_cardinality_str(card: Cardinality) -> &'static str {
    match card {
        Cardinality::ExactlyOne => "||",
        Cardinality::ZeroOrOne => "|o",
        Cardinality::OneOrMany => "|{",
        Cardinality::ZeroOrMany => "o{",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::er_ast::*;
    use crate::er_layout;
    use pretty_assertions::assert_eq;

    fn entity(name: &str) -> Entity {
        Entity { name: name.to_string(), attributes: Vec::new() }
    }

    #[test]
    fn render_single_relationship() {
        let diagram = ErDiagram {
            entities: vec![entity("A"), entity("B")],
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
┌───┐          ┌───┐
│ A │||──r1──||│ B │
└───┘          └───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_chain() {
        let diagram = ErDiagram {
            entities: vec![entity("CUSTOMER"), entity("ORDER"), entity("LINE-ITEM")],
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
