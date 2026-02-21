use std::collections::HashMap;

use crate::display_width::display_width;
use crate::graph_ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphLayout {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub subgraphs: Vec<SubgraphLayout>,
    pub width: usize,
    pub height: usize,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubgraphLayout {
    pub label: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub center_x: usize,
    pub center_y: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLayout {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

pub fn compute(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    if diagram.nodes.is_empty() {
        return Err("no nodes found".to_string());
    }

    let ranks = assign_ranks(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);

    let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
    for node in &diagram.nodes {
        let rank = ranks[&node.id];
        ranks_nodes[rank].push(node);
    }

    let mut node_layouts = match diagram.direction {
        Direction::TopDown => layout_td(&ranks_nodes),
        Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &diagram.edges),
    };

    let edges: Vec<EdgeLayout> = diagram
        .edges
        .iter()
        .map(|e| EdgeLayout {
            from_id: e.from.clone(),
            to_id: e.to.clone(),
            edge_type: e.edge_type,
            label: e.label.clone(),
        })
        .collect();

    let subgraphs = compute_subgraph_layouts(&diagram.subgraphs, &mut node_layouts);

    let mut width = node_layouts.iter().map(|n| n.x + n.width).max().unwrap_or(0);
    let mut height = node_layouts.iter().map(|n| n.y + n.height).max().unwrap_or(0);
    for sg in &subgraphs {
        width = width.max(sg.x + sg.width);
        height = height.max(sg.y + sg.height);
    }

    Ok(GraphLayout {
        nodes: node_layouts,
        edges,
        subgraphs,
        width,
        height,
        direction: diagram.direction.clone(),
    })
}

fn assign_ranks(diagram: &GraphDiagram) -> HashMap<String, usize> {
    let mut in_edges: HashMap<String, Vec<String>> = HashMap::new();
    for node in &diagram.nodes {
        in_edges.entry(node.id.clone()).or_default();
    }
    for edge in &diagram.edges {
        in_edges
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut ranks: HashMap<String, usize> = HashMap::new();

    for node in &diagram.nodes {
        if !ranks.contains_key(&node.id) {
            compute_rank(&node.id, &in_edges, &mut ranks);
        }
    }

    ranks
}

fn compute_rank(
    id: &str,
    in_edges: &HashMap<String, Vec<String>>,
    ranks: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&r) = ranks.get(id) {
        return r;
    }

    let predecessors = in_edges.get(id).cloned().unwrap_or_default();
    if predecessors.is_empty() {
        ranks.insert(id.to_string(), 0);
        return 0;
    }

    let max_pred = predecessors
        .iter()
        .map(|p| compute_rank(p, in_edges, ranks))
        .max()
        .unwrap_or(0);
    let rank = max_pred + 1;
    ranks.insert(id.to_string(), rank);
    rank
}

const BOX_HEIGHT: usize = 3;
const TD_RANK_SPACING: usize = 2;
const TD_NODE_GAP: usize = 3;
const LR_GAP: usize = 5;
const LR_NODE_VERTICAL_GAP: usize = 2;

fn layout_td(ranks_nodes: &[Vec<&NodeDecl>]) -> Vec<NodeLayout> {
    let mut layouts = Vec::new();

    let mut rank_widths: Vec<usize> = Vec::new();
    for rank_nodes in ranks_nodes {
        let total: usize = rank_nodes
            .iter()
            .map(|n| box_width(&n.label, n.shape))
            .sum::<usize>()
            + if rank_nodes.len() > 1 {
                (rank_nodes.len() - 1) * TD_NODE_GAP
            } else {
                0
            };
        rank_widths.push(total);
    }
    let max_width = *rank_widths.iter().max().unwrap_or(&0);

    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        let rank_total = rank_widths[rank];
        let base_x = if max_width > rank_total {
            (max_width - rank_total) / 2
        } else {
            0
        };

        let y = rank * (BOX_HEIGHT + TD_RANK_SPACING);
        let mut x = base_x;

        for node in rank_nodes {
            let w = box_width(&node.label, node.shape);
            layouts.push(NodeLayout {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape,
                x,
                y,
                width: w,
                height: BOX_HEIGHT,
                center_x: x + w / 2,
                center_y: y + 1,
            });
            x += w + TD_NODE_GAP;
        }
    }

    layouts
}

fn layout_lr(
    ranks_nodes: &[Vec<&NodeDecl>],
    ranks: &HashMap<String, usize>,
    edges: &[Edge],
) -> Vec<NodeLayout> {
    let mut layouts = Vec::new();
    let mut rank_x = 0;

    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        let rank_max_width = rank_nodes
            .iter()
            .map(|n| box_width(&n.label, n.shape))
            .max()
            .unwrap_or(0);
        let mut y = 0;

        for node in rank_nodes {
            let w = box_width(&node.label, node.shape);
            layouts.push(NodeLayout {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape,
                x: rank_x,
                y,
                width: w,
                height: BOX_HEIGHT,
                center_x: rank_x + w / 2,
                center_y: y + 1,
            });
            y += BOX_HEIGHT + LR_NODE_VERTICAL_GAP;
        }

        if rank + 1 < ranks_nodes.len() {
            let label_gap = edges
                .iter()
                .filter(|e| {
                    ranks.get(&e.from) == Some(&rank)
                        && ranks.get(&e.to) == Some(&(rank + 1))
                })
                .filter_map(|e| e.label.as_ref().map(|l| display_width(l) + 2))
                .max()
                .unwrap_or(0);
            let gap = LR_GAP.max(label_gap);
            rank_x += rank_max_width + gap;
        }
    }

    layouts
}

fn box_width(label: &str, shape: NodeShape) -> usize {
    let base = display_width(label) + 4;
    match shape {
        NodeShape::Circle => base + 4,
        _ => base,
    }
}

const SUBGRAPH_PAD_LEFT: usize = 2;
const SUBGRAPH_PAD_RIGHT: usize = 2;
const SUBGRAPH_PAD_TOP: usize = 1;
const SUBGRAPH_PAD_BOTTOM: usize = 1;
const SUBGRAPH_TITLE_DECOR: usize = 6;

fn compute_subgraph_layouts(
    subgraphs: &[Subgraph],
    node_layouts: &mut [NodeLayout],
) -> Vec<SubgraphLayout> {
    let mut sg_layouts = Vec::new();

    for sg in subgraphs {
        let contained: Vec<usize> = node_layouts
            .iter()
            .enumerate()
            .filter(|(_, n)| sg.node_ids.contains(&n.id))
            .map(|(i, _)| i)
            .collect();

        if contained.is_empty() {
            continue;
        }

        let min_x = contained.iter().map(|&i| node_layouts[i].x).min().unwrap();
        let min_y = contained.iter().map(|&i| node_layouts[i].y).min().unwrap();

        for &i in &contained {
            node_layouts[i].x += SUBGRAPH_PAD_LEFT;
            node_layouts[i].y += SUBGRAPH_PAD_TOP;
            node_layouts[i].center_x += SUBGRAPH_PAD_LEFT;
            node_layouts[i].center_y += SUBGRAPH_PAD_TOP;
        }

        let max_right = contained
            .iter()
            .map(|&i| node_layouts[i].x + node_layouts[i].width)
            .max()
            .unwrap();
        let max_bottom = contained
            .iter()
            .map(|&i| node_layouts[i].y + node_layouts[i].height)
            .max()
            .unwrap();

        let content_width = max_right - min_x + SUBGRAPH_PAD_RIGHT;
        let title_width = display_width(&sg.label) + SUBGRAPH_TITLE_DECOR;
        let width = content_width.max(title_width);
        let height = max_bottom - min_y + SUBGRAPH_PAD_BOTTOM;

        sg_layouts.push(SubgraphLayout {
            label: sg.label.clone(),
            x: min_x,
            y: min_y,
            width,
            height,
        });
    }

    sg_layouts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_parser::parse_graph;
    use pretty_assertions::assert_eq;

    #[test]
    fn rank_linear_chain() {
        let diagram = parse_graph("graph TD\n    A --> B\n    B --> C\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
    }

    #[test]
    fn rank_fan_out() {
        let diagram = parse_graph("graph TD\n    A --> B\n    A --> C\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 1);
    }

    #[test]
    fn rank_fan_in() {
        let diagram = parse_graph("graph TD\n    A --> C\n    B --> C\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 0);
        assert_eq!(ranks["C"], 1);
    }

    #[test]
    fn layout_td_two_nodes() {
        let diagram = parse_graph("graph TD\n    A[Start] --> B[End]\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.nodes.len(), 2);
        let a = &layout.nodes[0];
        let b = &layout.nodes[1];
        assert!(b.y > a.y, "B should be below A in TD");
        assert_eq!(a.center_x, b.center_x, "linear chain should be centered");
    }

    #[test]
    fn layout_lr_two_nodes() {
        let diagram = parse_graph("graph LR\n    A[Start] --> B[End]\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let a = &layout.nodes[0];
        let b = &layout.nodes[1];
        assert!(b.x > a.x, "B should be right of A in LR");
        assert_eq!(a.y, b.y, "single row in LR");
    }

    #[test]
    fn layout_td_fan_out_side_by_side() {
        let diagram = parse_graph("graph TD\n    A --> B\n    A --> C\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let a = layout.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.id == "B").unwrap();
        let c = layout.nodes.iter().find(|n| n.id == "C").unwrap();

        assert_eq!(b.y, c.y, "B and C on same rank");
        assert!(b.y > a.y, "children below parent");
        assert!(b.x < c.x, "B left of C");
    }

    #[test]
    fn layout_td_fan_in() {
        let diagram = parse_graph("graph TD\n    A --> C\n    B --> C\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let a = layout.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.id == "B").unwrap();
        let c = layout.nodes.iter().find(|n| n.id == "C").unwrap();

        assert_eq!(a.y, b.y, "A and B on same rank");
        assert!(c.y > a.y, "C below parents");
    }

    #[test]
    fn layout_box_dimensions() {
        let diagram = parse_graph("graph TD\n    A[Hello]\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let a = &layout.nodes[0];
        assert_eq!(a.width, "Hello".len() + 4);
        assert_eq!(a.height, 3);
    }

    #[test]
    fn layout_edges_preserved() {
        let diagram = parse_graph("graph TD\n    A --> B\n    A --- C\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.edges.len(), 2);
        assert_eq!(layout.edges[0].edge_type, EdgeType::Arrow);
        assert_eq!(layout.edges[1].edge_type, EdgeType::OpenLink);
    }

    #[test]
    fn layout_subgraph_basic() {
        let diagram =
            parse_graph("graph TD\n    subgraph Backend\n        A --> B\n    end\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.subgraphs.len(), 1);
        let sg = &layout.subgraphs[0];
        assert_eq!(sg.label, "Backend");

        let a = layout.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.id == "B").unwrap();

        // Subgraph bounding box must contain all its nodes
        assert!(sg.x <= a.x, "subgraph left <= node A x");
        assert!(sg.y <= a.y, "subgraph top <= node A y");
        assert!(sg.x + sg.width >= b.x + b.width, "subgraph right >= node B right");
        assert!(sg.y + sg.height >= b.y + b.height, "subgraph bottom >= node B bottom");
    }
}
