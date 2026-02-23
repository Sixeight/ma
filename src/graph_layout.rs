use std::collections::HashMap;

use crate::display_width::{display_width, line_count, multiline_width};
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

const SUBGRAPH_GAP: usize = 3;

pub fn compute(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    if diagram.nodes.is_empty() {
        return Err("no nodes found".to_string());
    }

    if !diagram.subgraphs.is_empty() {
        return layout_with_subgraphs(diagram);
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

fn layout_with_subgraphs(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    let node_to_subgraph: HashMap<String, usize> = diagram
        .subgraphs
        .iter()
        .enumerate()
        .flat_map(|(i, sg)| sg.node_ids.iter().map(move |id| (id.clone(), i)))
        .collect();

    // Build mini-diagrams for each subgraph
    let mut sg_groups: Vec<GraphDiagram> = Vec::new();
    for sg in &diagram.subgraphs {
        let nodes: Vec<NodeDecl> = diagram
            .nodes
            .iter()
            .filter(|n| sg.node_ids.contains(&n.id))
            .cloned()
            .collect();
        let edges: Vec<Edge> = diagram
            .edges
            .iter()
            .filter(|e| sg.node_ids.contains(&e.from) && sg.node_ids.contains(&e.to))
            .cloned()
            .collect();
        sg_groups.push(GraphDiagram {
            direction: diagram.direction.clone(),
            nodes,
            edges,
            subgraphs: vec![],
        });
    }

    // Collect bare nodes (not in any subgraph)
    let bare_nodes: Vec<&NodeDecl> = diagram
        .nodes
        .iter()
        .filter(|n| !node_to_subgraph.contains_key(&n.id))
        .collect();
    let bare_edges: Vec<&Edge> = diagram
        .edges
        .iter()
        .filter(|e| {
            !node_to_subgraph.contains_key(&e.from) && !node_to_subgraph.contains_key(&e.to)
        })
        .collect();

    // Layout each subgraph independently
    let mut all_nodes: Vec<NodeLayout> = Vec::new();
    let mut sg_layouts: Vec<SubgraphLayout> = Vec::new();
    let mut x_offset: usize = 0;

    for (i, sg_diagram) in sg_groups.iter().enumerate() {
        if sg_diagram.nodes.is_empty() {
            continue;
        }

        let ranks = assign_ranks(sg_diagram);
        let max_rank = *ranks.values().max().unwrap_or(&0);
        let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
        for node in &sg_diagram.nodes {
            let rank = ranks[&node.id];
            ranks_nodes[rank].push(node);
        }

        let mut node_layouts = match diagram.direction {
            Direction::TopDown => layout_td(&ranks_nodes),
            Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &sg_diagram.edges),
        };

        // Apply subgraph padding
        let sg = &diagram.subgraphs[i];
        for nl in &mut node_layouts {
            nl.x += x_offset + SUBGRAPH_PAD_LEFT;
            nl.y += SUBGRAPH_PAD_TOP;
            nl.center_x += x_offset + SUBGRAPH_PAD_LEFT;
            nl.center_y += SUBGRAPH_PAD_TOP;
        }

        let content_right = node_layouts
            .iter()
            .map(|n| n.x + n.width)
            .max()
            .unwrap_or(0);
        let content_bottom = node_layouts
            .iter()
            .map(|n| n.y + n.height)
            .max()
            .unwrap_or(0);

        let content_width = content_right - x_offset + SUBGRAPH_PAD_RIGHT;
        let title_width = display_width(&sg.label) + SUBGRAPH_TITLE_DECOR;
        let sg_width = content_width.max(title_width);
        let sg_height = content_bottom + SUBGRAPH_PAD_BOTTOM;

        sg_layouts.push(SubgraphLayout {
            label: sg.label.clone(),
            x: x_offset,
            y: 0,
            width: sg_width,
            height: sg_height,
        });

        all_nodes.extend(node_layouts);
        x_offset += sg_width + SUBGRAPH_GAP;
    }

    // Layout bare nodes
    if !bare_nodes.is_empty() {
        let bare_diagram = GraphDiagram {
            direction: diagram.direction.clone(),
            nodes: bare_nodes.into_iter().cloned().collect(),
            edges: bare_edges.into_iter().cloned().collect(),
            subgraphs: vec![],
        };
        let ranks = assign_ranks(&bare_diagram);
        let max_rank = *ranks.values().max().unwrap_or(&0);
        let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
        for node in &bare_diagram.nodes {
            let rank = ranks[&node.id];
            ranks_nodes[rank].push(node);
        }

        let mut node_layouts = match diagram.direction {
            Direction::TopDown => layout_td(&ranks_nodes),
            Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &bare_diagram.edges),
        };

        for nl in &mut node_layouts {
            nl.x += x_offset;
            nl.center_x += x_offset;
        }

        all_nodes.extend(node_layouts);
    }

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

    let mut width = all_nodes.iter().map(|n| n.x + n.width).max().unwrap_or(0);
    let mut height = all_nodes.iter().map(|n| n.y + n.height).max().unwrap_or(0);
    for sg in &sg_layouts {
        width = width.max(sg.x + sg.width);
        height = height.max(sg.y + sg.height);
    }

    Ok(GraphLayout {
        nodes: all_nodes,
        edges,
        subgraphs: sg_layouts,
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

pub fn compute_with_max_width(
    diagram: &GraphDiagram,
    max_width: usize,
) -> Result<GraphLayout, String> {
    let layout = compute(diagram)?;
    if layout.width <= max_width {
        return Ok(layout);
    }

    // Subgraph case: no gap reduction fallback (already laid out independently)
    if !diagram.subgraphs.is_empty() {
        return Err(format!("graph diagram too wide for {max_width} columns"));
    }

    // Try with progressively smaller gaps
    let ranks = assign_ranks(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);
    let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
    for node in &diagram.nodes {
        let rank = ranks[&node.id];
        ranks_nodes[rank].push(node);
    }

    for node_gap in (0..TD_NODE_GAP).rev() {
        for lr_gap in (1..LR_GAP).rev() {
            let mut node_layouts = match diagram.direction {
                Direction::TopDown => layout_td_with_gap(&ranks_nodes, node_gap),
                Direction::LeftRight => {
                    layout_lr_with_gap(&ranks_nodes, &ranks, &diagram.edges, lr_gap)
                }
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

            if width <= max_width {
                return Ok(GraphLayout {
                    nodes: node_layouts,
                    edges,
                    subgraphs,
                    width,
                    height,
                    direction: diagram.direction.clone(),
                });
            }
        }
    }

    Err(format!("graph diagram too wide for {max_width} columns"))
}

fn layout_td(ranks_nodes: &[Vec<&NodeDecl>]) -> Vec<NodeLayout> {
    layout_td_with_gap(ranks_nodes, TD_NODE_GAP)
}

fn layout_td_with_gap(ranks_nodes: &[Vec<&NodeDecl>], node_gap: usize) -> Vec<NodeLayout> {
    let mut layouts = Vec::new();

    let mut rank_widths: Vec<usize> = Vec::new();
    for rank_nodes in ranks_nodes {
        let total: usize = rank_nodes
            .iter()
            .map(|n| box_width(&n.label, n.shape))
            .sum::<usize>()
            + if rank_nodes.len() > 1 {
                (rank_nodes.len() - 1) * node_gap
            } else {
                0
            };
        rank_widths.push(total);
    }
    let max_width = *rank_widths.iter().max().unwrap_or(&0);

    let mut rank_heights: Vec<usize> = Vec::new();
    for rank_nodes in ranks_nodes {
        let max_h = rank_nodes
            .iter()
            .map(|n| box_height(&n.label))
            .max()
            .unwrap_or(BOX_HEIGHT);
        rank_heights.push(max_h);
    }

    let mut y = 0;
    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        let rank_total = rank_widths[rank];
        let base_x = if max_width > rank_total {
            (max_width - rank_total) / 2
        } else {
            0
        };

        let mut x = base_x;

        for node in rank_nodes {
            let w = box_width(&node.label, node.shape);
            let h = box_height(&node.label);
            layouts.push(NodeLayout {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape,
                x,
                y,
                width: w,
                height: h,
                center_x: x + w / 2,
                center_y: y + h / 2,
            });
            x += w + node_gap;
        }

        y += rank_heights[rank] + TD_RANK_SPACING;
    }

    layouts
}

fn layout_lr(
    ranks_nodes: &[Vec<&NodeDecl>],
    ranks: &HashMap<String, usize>,
    edges: &[Edge],
) -> Vec<NodeLayout> {
    layout_lr_with_gap(ranks_nodes, ranks, edges, LR_GAP)
}

fn layout_lr_with_gap(
    ranks_nodes: &[Vec<&NodeDecl>],
    ranks: &HashMap<String, usize>,
    edges: &[Edge],
    min_gap: usize,
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
            let h = box_height(&node.label);
            layouts.push(NodeLayout {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape,
                x: rank_x,
                y,
                width: w,
                height: h,
                center_x: rank_x + w / 2,
                center_y: y + h / 2,
            });
            y += h + LR_NODE_VERTICAL_GAP;
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
            let gap = min_gap.max(label_gap);
            rank_x += rank_max_width + gap;
        }
    }

    layouts
}

fn box_width(label: &str, shape: NodeShape) -> usize {
    let base = multiline_width(label) + 4;
    match shape {
        NodeShape::Circle => base + 4,
        _ => base,
    }
}

fn box_height(label: &str) -> usize {
    2 + line_count(label)
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
    fn layout_two_subgraphs_no_overlap() {
        let diagram = parse_graph(
            "graph TD\n    subgraph GroupA\n        A --> B\n        A --> C\n    end\n    subgraph GroupB\n        D --> E\n    end\n",
        )
        .unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.subgraphs.len(), 2);
        let sg_a = layout.subgraphs.iter().find(|s| s.label == "GroupA").unwrap();
        let sg_b = layout.subgraphs.iter().find(|s| s.label == "GroupB").unwrap();

        // Subgraph x-ranges must not overlap
        let a_right = sg_a.x + sg_a.width;
        let b_right = sg_b.x + sg_b.width;
        assert!(
            a_right <= sg_b.x || b_right <= sg_a.x,
            "subgraphs overlap: GroupA({}-{}), GroupB({}-{})",
            sg_a.x, a_right, sg_b.x, b_right
        );

        // Each node must be within its subgraph bounds
        for node_id in &["A", "B", "C"] {
            let n = layout.nodes.iter().find(|n| n.id == *node_id).unwrap();
            assert!(n.x >= sg_a.x, "{node_id} x < sg_a.x");
            assert!(n.x + n.width <= sg_a.x + sg_a.width, "{node_id} right > sg_a right");
            assert!(n.y >= sg_a.y, "{node_id} y < sg_a.y");
            assert!(n.y + n.height <= sg_a.y + sg_a.height, "{node_id} bottom > sg_a bottom");
        }
        for node_id in &["D", "E"] {
            let n = layout.nodes.iter().find(|n| n.id == *node_id).unwrap();
            assert!(n.x >= sg_b.x, "{node_id} x < sg_b.x");
            assert!(n.x + n.width <= sg_b.x + sg_b.width, "{node_id} right > sg_b right");
            assert!(n.y >= sg_b.y, "{node_id} y < sg_b.y");
            assert!(n.y + n.height <= sg_b.y + sg_b.height, "{node_id} bottom > sg_b bottom");
        }
    }

    #[test]
    fn layout_subgraph_with_bare_nodes() {
        let diagram = parse_graph(
            "graph TD\n    C\n    subgraph Backend\n        A --> B\n    end\n    C --> A\n",
        )
        .unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.subgraphs.len(), 1);
        let sg = &layout.subgraphs[0];
        assert_eq!(sg.label, "Backend");

        // C is a bare node, should not be inside the subgraph
        let c = layout.nodes.iter().find(|n| n.id == "C").unwrap();
        let a = layout.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.id == "B").unwrap();

        // A and B must be inside the subgraph
        assert!(a.x >= sg.x, "A x >= sg.x");
        assert!(a.x + a.width <= sg.x + sg.width, "A right <= sg right");
        assert!(b.x >= sg.x, "B x >= sg.x");
        assert!(b.x + b.width <= sg.x + sg.width, "B right <= sg right");

        // C must not overlap with the subgraph x-range
        let c_right = c.x + c.width;
        let sg_right = sg.x + sg.width;
        assert!(
            c_right <= sg.x || c.x >= sg_right,
            "bare node C overlaps subgraph: C({}-{}), sg({}-{})",
            c.x, c_right, sg.x, sg_right
        );
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
