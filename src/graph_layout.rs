use std::collections::{HashMap, HashSet};

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
    pub id: String,
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
    pub route: EdgeRoute,
}

pub(crate) fn edge_endpoint_nodes(
    nodes: &[NodeLayout],
    subgraphs: &[SubgraphLayout],
) -> Vec<NodeLayout> {
    let mut endpoints = nodes.to_vec();
    endpoints.extend(subgraphs.iter().map(|subgraph| NodeLayout {
        id: subgraph.id.clone(),
        label: subgraph.label.clone(),
        shape: NodeShape::Box,
        x: subgraph.x,
        y: subgraph.y,
        width: subgraph.width,
        height: subgraph.height,
        center_x: subgraph.x + subgraph.width / 2,
        center_y: subgraph.y + subgraph.height / 2,
    }));
    endpoints
}

pub(crate) fn is_subgraph_entry(edge: &EdgeLayout, subgraphs: &[SubgraphLayout]) -> bool {
    edge.route == EdgeRoute::Forward
        && edge.label.is_none()
        && subgraphs.iter().any(|sg| sg.id == edge.to_id)
        && !subgraphs.iter().any(|sg| sg.id == edge.from_id)
}

fn reserve_subgraph_entry_space(layout: &mut GraphLayout) {
    if layout.direction == Direction::TopDown {
        layout.width += layout
            .edges
            .iter()
            .filter(|edge| is_subgraph_entry(edge, &layout.subgraphs))
            .count();
    }
}

/// How an edge reaches its target, decided once the nodes are placed. Space is
/// reserved and the route is drawn from this, so both agree by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeRoute {
    Forward,
    SelfLoop,
    /// Runs backwards around the nodes, through gutter lane `lane`.
    Back {
        lane: usize,
    },
}

const SUBGRAPH_GAP: usize = 3;

/// Extent of the drawing before any routing space is reserved.
fn base_extents(nodes: &[NodeLayout], subgraphs: &[SubgraphLayout]) -> (usize, usize) {
    let mut width = nodes.iter().map(|n| n.x + n.width).max().unwrap_or(0);
    let mut height = nodes.iter().map(|n| n.y + n.height).max().unwrap_or(0);
    for sg in subgraphs {
        width = width.max(sg.x + sg.width);
        height = height.max(sg.y + sg.height);
    }
    (width, height)
}

/// Row/column kept free between the nodes and the back-edge gutter lanes.
const BACK_EDGE_CLEARANCE: usize = 1;

/// True when the target does not sit ahead of the source in the flow direction,
/// so the edge has to be routed backwards around the nodes. Self-loops are the
/// caller's business.
pub fn is_back_edge(direction: &Direction, from: &NodeLayout, to: &NodeLayout) -> bool {
    match direction {
        Direction::TopDown => to.y < from.y + from.height,
        Direction::LeftRight => to.x < from.x + from.width,
    }
}

/// Decides each edge's route now that the nodes are placed, handing every back
/// edge its own gutter lane in declaration order.
fn assign_edge_routes(direction: &Direction, nodes: &[NodeLayout], edges: &mut [EdgeLayout]) {
    let mut lane = 0;
    for edge in edges.iter_mut() {
        edge.route = if edge.from_id == edge.to_id {
            EdgeRoute::SelfLoop
        } else {
            let from = nodes.iter().find(|n| n.id == edge.from_id);
            let to = nodes.iter().find(|n| n.id == edge.to_id);
            match (from, to) {
                (Some(from), Some(to)) if is_back_edge(direction, from, to) => {
                    lane += 1;
                    EdgeRoute::Back { lane: lane - 1 }
                }
                _ => EdgeRoute::Forward,
            }
        };
    }
}

/// Number of gutter lanes the back edges occupy.
pub fn back_edge_lane_count(edges: &[EdgeLayout]) -> usize {
    edges
        .iter()
        .filter(|edge| matches!(edge.route, EdgeRoute::Back { .. }))
        .count()
}

fn reserve_back_edge_space(
    direction: &Direction,
    edges: &[EdgeLayout],
    width: &mut usize,
    height: &mut usize,
) {
    let lane_count = back_edge_lane_count(edges);
    if lane_count == 0 {
        return;
    }

    let label_width = edges
        .iter()
        .filter(|edge| matches!(edge.route, EdgeRoute::Back { .. }))
        .filter_map(|edge| edge.label.as_ref())
        .map(|label| display_width(label) + 1)
        .max()
        .unwrap_or(0);

    // Invariant the renderer depends on: the gutter dimension grows by exactly
    // BACK_EDGE_CLEARANCE + lane_count, so lane i lives at
    // `size - lane_count + i` and never collides with a node.
    match direction {
        // TD routes through gutter columns right of everything, reached over
        // the free row below the source's rank. The label rides on that row.
        Direction::TopDown => {
            *width += BACK_EDGE_CLEARANCE + label_width + lane_count;
            *height += BACK_EDGE_CLEARANCE + lane_count;
        }
        // LR routes through gutter rows below everything, reached over the
        // free column right of the source's rank — the last rank needs one
        // column added for that, plus room for a label parked right of a route
        // too short to hold it.
        Direction::LeftRight => {
            *width += BACK_EDGE_CLEARANCE + label_width + 1;
            *height += BACK_EDGE_CLEARANCE + lane_count;
        }
    }
}

pub fn compute(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    if diagram.nodes.is_empty() {
        return Err("no nodes found".to_string());
    }

    if diagram.is_nested() {
        return layout_nested(diagram);
    }

    if !diagram.subgraphs.is_empty() {
        if outer_node_on_subgraph_ranks(diagram) {
            return match diagram.direction {
                Direction::TopDown => layout_td_shared_ranks(diagram),
                Direction::LeftRight => layout_lr_shared_ranks(diagram),
            };
        }
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
        Direction::TopDown => layout_td(&ranks_nodes, &diagram.edges),
        Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &diagram.edges),
    };

    let mut edges: Vec<EdgeLayout> = diagram
        .edges
        .iter()
        .filter(|e| e.edge_type != EdgeType::Invisible)
        .map(|e| EdgeLayout {
            from_id: e.from.clone(),
            to_id: e.to.clone(),
            edge_type: e.edge_type,
            label: e.label.clone(),
            route: EdgeRoute::Forward,
        })
        .collect();

    let subgraphs = compute_subgraph_layouts(&diagram.subgraphs, &mut node_layouts);
    assign_edge_routes(&diagram.direction, &node_layouts, &mut edges);

    let (mut width, mut height) = base_extents(&node_layouts, &subgraphs);

    if diagram.direction == Direction::TopDown {
        width = width.max(td_labels_right(&node_layouts, &diagram.edges));
    }

    // Self-loop nodes need extra space: arm (2 cols) + label width to the right,
    // and 1 row below the node for the return arrow
    for edge in &diagram.edges {
        if edge.from == edge.to
            && let Some(nl) = node_layouts.iter().find(|n| n.id == edge.from)
        {
            let label_w = edge.label.as_ref().map(|l| display_width(l)).unwrap_or(0);
            let needed_right = nl.x + nl.width + 2 + label_w;
            width = width.max(needed_right);
            let needed_bottom = nl.y + nl.height + 1;
            height = height.max(needed_bottom);
        }
    }

    // Cross-rank fan-in edges: reserve gutter column width.
    let max_right = node_layouts
        .iter()
        .map(|n| n.x + n.width)
        .max()
        .unwrap_or(0);
    let has_cross_rank_fan_in = diagram.edges.iter().any(|edge| {
        if edge.from == edge.to {
            return false;
        }
        let parents: Vec<&NodeLayout> = diagram
            .edges
            .iter()
            .filter(|e| e.to == edge.to && e.from != e.to)
            .filter_map(|e| node_layouts.iter().find(|n| n.id == e.from))
            .collect();
        parents.len() > 1 && !parents.windows(2).all(|w| w[0].y == w[1].y)
    });
    if has_cross_rank_fan_in {
        width = width.max(max_right + 2);
    }

    reserve_back_edge_space(&diagram.direction, &edges, &mut width, &mut height);

    Ok(GraphLayout {
        nodes: node_layouts,
        edges,
        subgraphs,
        width,
        height,
        direction: diagram.direction.clone(),
    })
}

fn outer_node_on_subgraph_ranks(diagram: &GraphDiagram) -> bool {
    let ranks = ranks_with_subgraph_endpoints(diagram);
    let node_to_subgraph = node_to_subgraph_index(diagram);
    let spans: Vec<(usize, usize)> = diagram
        .subgraphs
        .iter()
        .filter_map(|sg| {
            let member_ranks: Vec<usize> = sg
                .node_ids
                .iter()
                .filter_map(|id| ranks.get(id).copied())
                .collect();
            let min = *member_ranks.iter().min()?;
            let max = *member_ranks.iter().max()?;
            Some((min, max))
        })
        .collect();
    diagram.nodes.iter().any(|node| {
        if node_to_subgraph.contains_key(node.id.as_str()) {
            return false;
        }
        let Some(&rank) = ranks.get(&node.id) else {
            return false;
        };
        spans.iter().any(|&(min, max)| rank >= min && rank <= max)
    })
}

fn node_to_subgraph_index(diagram: &GraphDiagram) -> HashMap<&str, usize> {
    diagram
        .subgraphs
        .iter()
        .enumerate()
        .flat_map(|(index, sg)| sg.node_ids.iter().map(move |id| (id.as_str(), index)))
        .collect()
}

fn subgraph_member_ids(diagram: &GraphDiagram) -> HashMap<&str, &[String]> {
    let mut members = HashMap::new();
    for sg in &diagram.subgraphs {
        members.insert(sg.id.as_str(), sg.node_ids.as_slice());
    }
    members
}

fn endpoint_node_ids<'a>(
    id: &'a str,
    members: &HashMap<&str, &'a [String]>,
    node_ids: &HashSet<&str>,
) -> Vec<&'a str> {
    if let Some(ids) = members.get(id) {
        ids.iter()
            .map(String::as_str)
            .filter(|id| node_ids.contains(id))
            .collect()
    } else if node_ids.contains(id) {
        vec![id]
    } else {
        Vec::new()
    }
}

fn ranks_with_subgraph_endpoints(diagram: &GraphDiagram) -> HashMap<String, usize> {
    let members = subgraph_member_ids(diagram);
    let node_ids: HashSet<&str> = diagram.nodes.iter().map(|node| node.id.as_str()).collect();
    let mut edges = Vec::new();
    for edge in &diagram.edges {
        for from in endpoint_node_ids(&edge.from, &members, &node_ids) {
            for to in endpoint_node_ids(&edge.to, &members, &node_ids) {
                if from != to {
                    edges.push(Edge {
                        from: from.to_string(),
                        to: to.to_string(),
                        edge_type: edge.edge_type,
                        label: edge.label.clone(),
                    });
                }
            }
        }
    }
    assign_ranks(&GraphDiagram {
        direction: diagram.direction.clone(),
        nodes: diagram.nodes.clone(),
        edges,
        subgraphs: Vec::new(),
    })
}

fn shift_nodes_from(nodes: &mut [NodeLayout], y_threshold: usize, dy: usize) {
    if dy == 0 {
        return;
    }
    for node in nodes {
        if node.y >= y_threshold {
            node.y += dy;
            node.center_y += dy;
        }
    }
}

fn shift_nodes_from_x(nodes: &mut [NodeLayout], x_threshold: usize, dx: usize) {
    if dx == 0 {
        return;
    }
    for node in nodes {
        if node.x >= x_threshold {
            node.x += dx;
            node.center_x += dx;
        }
    }
}

fn shift_all_nodes(nodes: &mut [NodeLayout], dx: usize, dy: usize) {
    for node in nodes {
        node.x += dx;
        node.y += dy;
        node.center_x += dx;
        node.center_y += dy;
    }
}

fn insert_subgraph_chrome(diagram: &GraphDiagram, nodes: &mut [NodeLayout]) {
    let mut first_ys: Vec<(usize, bool)> = Vec::new();
    for sg in &diagram.subgraphs {
        let members: Vec<&NodeLayout> = nodes
            .iter()
            .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let first_y = members.iter().map(|node| node.y).min().unwrap();
        let incoming_from_above = diagram.edges.iter().any(|edge| {
            sg.node_ids.iter().any(|id| id == &edge.to)
                && !sg.node_ids.iter().any(|id| id == &edge.from)
                && nodes
                    .iter()
                    .any(|node| node.id == edge.from && node.y + node.height <= first_y)
        });
        first_ys.push((first_y, incoming_from_above));
    }
    first_ys.sort_by_key(|(y, _)| *y);
    let mut merged_first = Vec::new();
    for (y, incoming) in first_ys {
        match merged_first.last_mut() {
            Some((last_y, last_incoming)) if *last_y == y => *last_incoming |= incoming,
            _ => merged_first.push((y, incoming)),
        }
    }
    first_ys = merged_first;
    for (first_y, incoming_from_above) in first_ys.into_iter().rev() {
        let need = SUBGRAPH_PAD_TOP + usize::from(incoming_from_above);
        let above = nodes
            .iter()
            .filter(|node| node.y + node.height <= first_y)
            .map(|node| node.y + node.height)
            .max()
            .unwrap_or(0);
        let gap = first_y.saturating_sub(above);
        if gap < need {
            shift_nodes_from(nodes, first_y, need - gap);
        }
    }

    let mut last_bottoms: Vec<(usize, bool)> = Vec::new();
    for sg in &diagram.subgraphs {
        let members: Vec<&NodeLayout> = nodes
            .iter()
            .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let last_bottom = members
            .iter()
            .map(|node| node.y + node.height)
            .max()
            .unwrap();
        let outgoing_below = diagram.edges.iter().any(|edge| {
            sg.node_ids.iter().any(|id| id == &edge.from)
                && !sg.node_ids.iter().any(|id| id == &edge.to)
                && nodes
                    .iter()
                    .any(|node| node.id == edge.to && node.y >= last_bottom)
        });
        last_bottoms.push((last_bottom, outgoing_below));
    }
    last_bottoms.sort_by_key(|(y, _)| *y);
    let mut merged_last = Vec::new();
    for (y, outgoing) in last_bottoms {
        match merged_last.last_mut() {
            Some((last_y, last_outgoing)) if *last_y == y => *last_outgoing |= outgoing,
            _ => merged_last.push((y, outgoing)),
        }
    }
    last_bottoms = merged_last;
    for (last_bottom, outgoing_below) in last_bottoms.into_iter().rev() {
        let need = SUBGRAPH_PAD_BOTTOM + usize::from(outgoing_below);
        let below = nodes
            .iter()
            .filter(|node| node.y >= last_bottom)
            .map(|node| node.y)
            .min()
            .unwrap_or(last_bottom + need);
        let gap = below.saturating_sub(last_bottom);
        if gap < need {
            shift_nodes_from(nodes, last_bottom, need - gap);
        }
    }
}

fn ensure_forward_edge_gaps(nodes: &mut [NodeLayout], edges: &[Edge]) {
    loop {
        let mut inserted = false;
        for edge in edges {
            if edge.from == edge.to || edge.edge_type == EdgeType::Invisible {
                continue;
            }
            let Some(from) = nodes.iter().find(|node| node.id == edge.from).cloned() else {
                continue;
            };
            let Some(to) = nodes.iter().find(|node| node.id == edge.to).cloned() else {
                continue;
            };
            if to.y < from.y + from.height {
                continue;
            }
            let label_rows = edge.label.as_deref().map(line_count).unwrap_or(0);
            let jog_rows = if from.center_x == to.center_x { 0 } else { 2 };
            let need = from.y + from.height + 1 + label_rows + jog_rows;
            if to.y < need {
                shift_nodes_from(nodes, to.y, need - to.y);
                inserted = true;
                break;
            }
        }
        if !inserted {
            break;
        }
    }
}

fn finish_td_layout(
    diagram: &GraphDiagram,
    node_layouts: Vec<NodeLayout>,
    subgraphs: Vec<SubgraphLayout>,
) -> GraphLayout {
    let mut edges: Vec<EdgeLayout> = diagram
        .edges
        .iter()
        .filter(|edge| edge.edge_type != EdgeType::Invisible)
        .map(|edge| EdgeLayout {
            from_id: edge.from.clone(),
            to_id: edge.to.clone(),
            edge_type: edge.edge_type,
            label: edge.label.clone(),
            route: EdgeRoute::Forward,
        })
        .collect();
    let endpoints = edge_endpoint_nodes(&node_layouts, &subgraphs);
    assign_edge_routes(&diagram.direction, &endpoints, &mut edges);
    let (mut width, mut height) = base_extents(&node_layouts, &subgraphs);
    width = width.max(td_labels_right(&node_layouts, &diagram.edges));
    for edge in &diagram.edges {
        if edge.from == edge.to
            && let Some(nl) = node_layouts.iter().find(|node| node.id == edge.from)
        {
            let label_w = edge
                .label
                .as_ref()
                .map(|label| display_width(label))
                .unwrap_or(0);
            width = width.max(nl.x + nl.width + 2 + label_w);
            height = height.max(nl.y + nl.height + 1);
        }
    }
    let max_right = node_layouts
        .iter()
        .map(|node| node.x + node.width)
        .max()
        .unwrap_or(0);
    let has_cross_rank_fan_in = diagram.edges.iter().any(|edge| {
        if edge.from == edge.to {
            return false;
        }
        let parents: Vec<&NodeLayout> = diagram
            .edges
            .iter()
            .filter(|other| other.to == edge.to && other.from != other.to)
            .filter_map(|other| node_layouts.iter().find(|node| node.id == other.from))
            .collect();
        parents.len() > 1 && !parents.windows(2).all(|window| window[0].y == window[1].y)
    });
    if has_cross_rank_fan_in {
        width = width.max(max_right + 2);
    }
    reserve_back_edge_space(&diagram.direction, &edges, &mut width, &mut height);
    let mut layout = GraphLayout {
        nodes: node_layouts,
        edges,
        subgraphs,
        width,
        height,
        direction: diagram.direction.clone(),
    };
    reserve_subgraph_entry_space(&mut layout);
    clear_entry_title_collisions(&mut layout);
    layout
}

struct SubgraphBand {
    sg_index: usize,
    min_rank: usize,
    max_rank: usize,
    width: usize,
    x: usize,
}

fn band_width_for_subgraph(
    diagram: &GraphDiagram,
    sg: &Subgraph,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
    node_gap: usize,
) -> usize {
    let mut width = display_width(&sg.label) + SUBGRAPH_TITLE_DECOR;
    if let (Some(min_rank), Some(max_rank)) = (
        sg.node_ids
            .iter()
            .filter_map(|id| ranks.get(id).copied())
            .min(),
        sg.node_ids
            .iter()
            .filter_map(|id| ranks.get(id).copied())
            .max(),
    ) {
        for rank_nodes in ranks_nodes.iter().take(max_rank + 1).skip(min_rank) {
            let footprints: Vec<usize> = rank_nodes
                .iter()
                .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                .map(|node| {
                    box_width(&node.label, node.shape)
                        .max(td_label_width(&node.id, &diagram.edges) + 2)
                })
                .collect();
            if footprints.is_empty() {
                continue;
            }
            let content =
                footprints.iter().sum::<usize>() + footprints.len().saturating_sub(1) * node_gap;
            width = width.max(content + SUBGRAPH_PAD_LEFT + SUBGRAPH_PAD_RIGHT);
        }
    }
    width
}

fn pack_subgraph_bands(
    diagram: &GraphDiagram,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
    node_gap: usize,
) -> Vec<SubgraphBand> {
    let mut bands: Vec<SubgraphBand> = diagram
        .subgraphs
        .iter()
        .enumerate()
        .filter_map(|(sg_index, sg)| {
            let member_ranks: Vec<usize> = sg
                .node_ids
                .iter()
                .filter_map(|id| ranks.get(id).copied())
                .collect();
            let min_rank = *member_ranks.iter().min()?;
            let max_rank = *member_ranks.iter().max()?;
            Some(SubgraphBand {
                sg_index,
                min_rank,
                max_rank,
                width: band_width_for_subgraph(diagram, sg, ranks, ranks_nodes, node_gap),
                x: 0,
            })
        })
        .collect();

    let mut col_spans: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut col_of = vec![0; bands.len()];
    for (index, band) in bands.iter().enumerate() {
        let col = col_spans.iter().position(|spans| {
            spans
                .iter()
                .all(|&(min, max)| band.max_rank < min || band.min_rank > max)
        });
        let col = if let Some(col) = col {
            col_spans[col].push((band.min_rank, band.max_rank));
            col
        } else {
            col_spans.push(vec![(band.min_rank, band.max_rank)]);
            col_spans.len() - 1
        };
        col_of[index] = col;
    }

    let col_count = col_spans.len();
    let mut col_widths = vec![0; col_count];
    for (index, band) in bands.iter().enumerate() {
        col_widths[col_of[index]] = col_widths[col_of[index]].max(band.width);
    }
    let mut col_x = vec![0; col_count];
    let mut x = 0;
    for col in 0..col_count {
        col_x[col] = x;
        x += col_widths[col] + SUBGRAPH_GAP;
    }
    for (index, band) in bands.iter_mut().enumerate() {
        band.x = col_x[col_of[index]];
        band.width = col_widths[col_of[index]];
    }
    bands
}

fn place_rank_nodes(
    nodes: &mut [NodeLayout],
    decls: &[&NodeDecl],
    start_x: usize,
    node_gap: usize,
    edges: &[Edge],
) {
    let mut x = start_x;
    for decl in decls {
        let footprint = box_width(&decl.label, decl.shape).max(td_label_width(&decl.id, edges) + 2);
        if let Some(node) = nodes.iter_mut().find(|node| node.id == decl.id) {
            node.x = x + footprint / 2 - node.width / 2;
            node.center_x = node.x + node.width / 2;
        }
        x += footprint + node_gap;
    }
}

fn apply_td_bands(
    diagram: &GraphDiagram,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
    nodes: &mut [NodeLayout],
    node_to_subgraph: &HashMap<&str, usize>,
    node_gap: usize,
) -> Vec<SubgraphLayout> {
    let bands = pack_subgraph_bands(diagram, ranks, ranks_nodes, node_gap);
    let pack_width = bands
        .iter()
        .map(|band| band.x + band.width)
        .max()
        .unwrap_or(0);

    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        for band in &bands {
            if rank < band.min_rank || rank > band.max_rank {
                continue;
            }
            let sg = &diagram.subgraphs[band.sg_index];
            let siblings: Vec<&NodeDecl> = rank_nodes
                .iter()
                .copied()
                .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                .collect();
            if siblings.is_empty() {
                continue;
            }
            let total: usize = siblings
                .iter()
                .map(|node| {
                    box_width(&node.label, node.shape)
                        .max(td_label_width(&node.id, &diagram.edges) + 2)
                })
                .sum::<usize>()
                + siblings.len().saturating_sub(1) * node_gap;
            let inner = band
                .width
                .saturating_sub(SUBGRAPH_PAD_LEFT + SUBGRAPH_PAD_RIGHT);
            let start = band.x + SUBGRAPH_PAD_LEFT + inner.saturating_sub(total) / 2;
            place_rank_nodes(nodes, &siblings, start, node_gap, &diagram.edges);
        }

        let outers: Vec<&NodeDecl> = rank_nodes
            .iter()
            .copied()
            .filter(|node| !node_to_subgraph.contains_key(node.id.as_str()))
            .collect();
        if outers.is_empty() {
            continue;
        }
        let covering: Vec<&SubgraphBand> = bands
            .iter()
            .filter(|band| rank >= band.min_rank && rank <= band.max_rank)
            .collect();
        let start = if covering.is_empty() {
            let total: usize = outers
                .iter()
                .map(|node| {
                    box_width(&node.label, node.shape)
                        .max(td_label_width(&node.id, &diagram.edges) + 2)
                })
                .sum::<usize>()
                + outers.len().saturating_sub(1) * node_gap;
            pack_width.saturating_sub(total) / 2
        } else {
            covering
                .iter()
                .map(|band| band.x + band.width)
                .max()
                .unwrap_or(0)
                + SUBGRAPH_GAP
        };
        place_rank_nodes(nodes, &outers, start, node_gap, &diagram.edges);
    }

    bands
        .iter()
        .filter_map(|band| {
            let sg = &diagram.subgraphs[band.sg_index];
            let members: Vec<&NodeLayout> = nodes
                .iter()
                .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                .collect();
            if members.is_empty() {
                return None;
            }
            let min_y = members.iter().map(|node| node.y).min().unwrap();
            let max_bottom = members
                .iter()
                .map(|node| node.y + node.height)
                .max()
                .unwrap();
            let y = min_y.saturating_sub(SUBGRAPH_PAD_TOP);
            Some(SubgraphLayout {
                id: sg.id.clone(),
                label: sg.label.clone(),
                x: band.x,
                y,
                width: band.width,
                height: max_bottom + SUBGRAPH_PAD_BOTTOM - y,
            })
        })
        .collect()
}

fn layout_td_shared_ranks(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    debug_assert!(
        !diagram.is_nested(),
        "nested diagrams go through layout_nested"
    );
    layout_td_shared_ranks_with_gap(diagram, TD_NODE_GAP)
}

fn layout_td_shared_ranks_with_gap(
    diagram: &GraphDiagram,
    node_gap: usize,
) -> Result<GraphLayout, String> {
    let ranks = ranks_with_subgraph_endpoints(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);
    let node_to_subgraph = node_to_subgraph_index(diagram);
    let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
    for node in &diagram.nodes {
        ranks_nodes[ranks[&node.id]].push(node);
    }
    for rank_nodes in &mut ranks_nodes {
        rank_nodes.sort_by_key(|node| {
            node_to_subgraph
                .get(node.id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }

    let mut node_layouts = layout_td_with_gap(&ranks_nodes, &diagram.edges, node_gap);
    apply_td_bands(
        diagram,
        &ranks,
        &ranks_nodes,
        &mut node_layouts,
        &node_to_subgraph,
        node_gap,
    );
    insert_subgraph_chrome(diagram, &mut node_layouts);
    ensure_forward_edge_gaps(&mut node_layouts, &diagram.edges);
    if node_layouts
        .iter()
        .any(|node| node_to_subgraph.contains_key(node.id.as_str()) && node.y < SUBGRAPH_PAD_TOP)
    {
        shift_all_nodes(&mut node_layouts, 0, SUBGRAPH_PAD_TOP);
    }
    let subgraphs = apply_td_bands(
        diagram,
        &ranks,
        &ranks_nodes,
        &mut node_layouts,
        &node_to_subgraph,
        node_gap,
    );
    Ok(finish_td_layout(diagram, node_layouts, subgraphs))
}

struct SubgraphRowBand {
    sg_index: usize,
    min_rank: usize,
    max_rank: usize,
    height: usize,
    y: usize,
}

fn band_height_for_subgraph(
    sg: &Subgraph,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
) -> usize {
    let mut height = SUBGRAPH_PAD_TOP + SUBGRAPH_PAD_BOTTOM;
    if let (Some(min_rank), Some(max_rank)) = (
        sg.node_ids
            .iter()
            .filter_map(|id| ranks.get(id).copied())
            .min(),
        sg.node_ids
            .iter()
            .filter_map(|id| ranks.get(id).copied())
            .max(),
    ) {
        for rank_nodes in ranks_nodes.iter().take(max_rank + 1).skip(min_rank) {
            let footprints: Vec<usize> = rank_nodes
                .iter()
                .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                .map(|node| box_height(&node.label, node.shape))
                .collect();
            if footprints.is_empty() {
                continue;
            }
            let content = footprints.iter().sum::<usize>()
                + footprints.len().saturating_sub(1) * LR_NODE_VERTICAL_GAP;
            height = height.max(content + SUBGRAPH_PAD_TOP + SUBGRAPH_PAD_BOTTOM);
        }
    }
    height
}

fn pack_subgraph_row_bands(
    diagram: &GraphDiagram,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
) -> Vec<SubgraphRowBand> {
    let mut bands: Vec<SubgraphRowBand> = diagram
        .subgraphs
        .iter()
        .enumerate()
        .filter_map(|(sg_index, sg)| {
            let member_ranks: Vec<usize> = sg
                .node_ids
                .iter()
                .filter_map(|id| ranks.get(id).copied())
                .collect();
            let min_rank = *member_ranks.iter().min()?;
            let max_rank = *member_ranks.iter().max()?;
            Some(SubgraphRowBand {
                sg_index,
                min_rank,
                max_rank,
                height: band_height_for_subgraph(sg, ranks, ranks_nodes),
                y: 0,
            })
        })
        .collect();

    let mut row_spans: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut row_of = vec![0; bands.len()];
    for (index, band) in bands.iter().enumerate() {
        let row = row_spans.iter().position(|spans| {
            spans
                .iter()
                .all(|&(min, max)| band.max_rank < min || band.min_rank > max)
        });
        let row = if let Some(row) = row {
            row_spans[row].push((band.min_rank, band.max_rank));
            row
        } else {
            row_spans.push(vec![(band.min_rank, band.max_rank)]);
            row_spans.len() - 1
        };
        row_of[index] = row;
    }

    let row_count = row_spans.len();
    let mut row_heights = vec![0; row_count];
    for (index, band) in bands.iter().enumerate() {
        row_heights[row_of[index]] = row_heights[row_of[index]].max(band.height);
    }
    let mut row_y = vec![0; row_count];
    let mut y = 0;
    for row in 0..row_count {
        row_y[row] = y;
        y += row_heights[row] + SUBGRAPH_GAP;
    }
    for (index, band) in bands.iter_mut().enumerate() {
        band.y = row_y[row_of[index]];
        band.height = row_heights[row_of[index]];
    }
    bands
}

fn place_rank_nodes_y(nodes: &mut [NodeLayout], decls: &[&NodeDecl], start_y: usize) {
    let mut y = start_y;
    for decl in decls {
        if let Some(node) = nodes.iter_mut().find(|node| node.id == decl.id) {
            node.y = y;
            node.center_y = y + node.height / 2;
            y += node.height + LR_NODE_VERTICAL_GAP;
        }
    }
}

fn apply_lr_bands(
    diagram: &GraphDiagram,
    ranks: &HashMap<String, usize>,
    ranks_nodes: &[Vec<&NodeDecl>],
    nodes: &mut [NodeLayout],
    node_to_subgraph: &HashMap<&str, usize>,
    place_y: bool,
) -> Vec<SubgraphLayout> {
    let bands = pack_subgraph_row_bands(diagram, ranks, ranks_nodes);
    let pack_height = bands
        .iter()
        .map(|band| band.y + band.height)
        .max()
        .unwrap_or(0);

    if place_y {
        for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
            for band in &bands {
                if rank < band.min_rank || rank > band.max_rank {
                    continue;
                }
                let sg = &diagram.subgraphs[band.sg_index];
                let siblings: Vec<&NodeDecl> = rank_nodes
                    .iter()
                    .copied()
                    .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                    .collect();
                if siblings.is_empty() {
                    continue;
                }
                let total: usize = siblings
                    .iter()
                    .map(|node| box_height(&node.label, node.shape))
                    .sum::<usize>()
                    + siblings.len().saturating_sub(1) * LR_NODE_VERTICAL_GAP;
                let inner = band
                    .height
                    .saturating_sub(SUBGRAPH_PAD_TOP + SUBGRAPH_PAD_BOTTOM);
                let start = band.y + SUBGRAPH_PAD_TOP + inner.saturating_sub(total) / 2;
                place_rank_nodes_y(nodes, &siblings, start);
            }

            let outers: Vec<&NodeDecl> = rank_nodes
                .iter()
                .copied()
                .filter(|node| !node_to_subgraph.contains_key(node.id.as_str()))
                .collect();
            if outers.is_empty() {
                continue;
            }
            let covering: Vec<&SubgraphRowBand> = bands
                .iter()
                .filter(|band| rank >= band.min_rank && rank <= band.max_rank)
                .collect();
            let start = if covering.is_empty() {
                let total: usize = outers
                    .iter()
                    .map(|node| box_height(&node.label, node.shape))
                    .sum::<usize>()
                    + outers.len().saturating_sub(1) * LR_NODE_VERTICAL_GAP;
                pack_height.saturating_sub(total) / 2
            } else {
                covering
                    .iter()
                    .map(|band| band.y + band.height)
                    .max()
                    .unwrap_or(0)
            };
            place_rank_nodes_y(nodes, &outers, start);
        }
    }

    bands
        .iter()
        .filter_map(|band| {
            let sg = &diagram.subgraphs[band.sg_index];
            let members: Vec<&NodeLayout> = nodes
                .iter()
                .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
                .collect();
            if members.is_empty() {
                return None;
            }
            let min_x = members.iter().map(|node| node.x).min().unwrap();
            let max_right = members
                .iter()
                .map(|node| node.x + node.width)
                .max()
                .unwrap();
            let title_width = display_width(&sg.label) + SUBGRAPH_TITLE_DECOR;
            let x = min_x.saturating_sub(SUBGRAPH_PAD_LEFT);
            let width = (max_right + SUBGRAPH_PAD_RIGHT - x).max(title_width);
            Some(SubgraphLayout {
                id: sg.id.clone(),
                label: sg.label.clone(),
                x,
                y: band.y,
                width,
                height: band.height,
            })
        })
        .collect()
}

fn insert_lr_subgraph_chrome(diagram: &GraphDiagram, nodes: &mut [NodeLayout]) {
    let mut first_xs: Vec<(usize, bool)> = Vec::new();
    for sg in &diagram.subgraphs {
        let members: Vec<&NodeLayout> = nodes
            .iter()
            .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let first_x = members.iter().map(|node| node.x).min().unwrap();
        let incoming_from_left = diagram.edges.iter().any(|edge| {
            sg.node_ids.iter().any(|id| id == &edge.to)
                && !sg.node_ids.iter().any(|id| id == &edge.from)
                && nodes
                    .iter()
                    .any(|node| node.id == edge.from && node.x + node.width <= first_x)
        });
        first_xs.push((first_x, incoming_from_left));
    }
    first_xs.sort_by_key(|(x, _)| *x);
    let mut merged_first = Vec::new();
    for (x, incoming) in first_xs {
        match merged_first.last_mut() {
            Some((last_x, last_incoming)) if *last_x == x => *last_incoming |= incoming,
            _ => merged_first.push((x, incoming)),
        }
    }
    for (first_x, incoming_from_left) in merged_first.into_iter().rev() {
        let need = SUBGRAPH_PAD_LEFT + usize::from(incoming_from_left);
        let left = nodes
            .iter()
            .filter(|node| node.x + node.width <= first_x)
            .map(|node| node.x + node.width)
            .max()
            .unwrap_or(0);
        let gap = first_x.saturating_sub(left);
        if gap < need {
            shift_nodes_from_x(nodes, first_x, need - gap);
        }
    }

    let mut last_rights: Vec<(usize, bool)> = Vec::new();
    for sg in &diagram.subgraphs {
        let members: Vec<&NodeLayout> = nodes
            .iter()
            .filter(|node| sg.node_ids.iter().any(|id| id == &node.id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let last_right = members
            .iter()
            .map(|node| node.x + node.width)
            .max()
            .unwrap();
        let outgoing_right = diagram.edges.iter().any(|edge| {
            sg.node_ids.iter().any(|id| id == &edge.from)
                && !sg.node_ids.iter().any(|id| id == &edge.to)
                && nodes
                    .iter()
                    .any(|node| node.id == edge.to && node.x >= last_right)
        });
        last_rights.push((last_right, outgoing_right));
    }
    last_rights.sort_by_key(|(x, _)| *x);
    let mut merged_last = Vec::new();
    for (x, outgoing) in last_rights {
        match merged_last.last_mut() {
            Some((last_x, last_outgoing)) if *last_x == x => *last_outgoing |= outgoing,
            _ => merged_last.push((x, outgoing)),
        }
    }
    for (last_right, outgoing_right) in merged_last.into_iter().rev() {
        let need = SUBGRAPH_PAD_RIGHT + usize::from(outgoing_right);
        let right = nodes
            .iter()
            .filter(|node| node.x >= last_right)
            .map(|node| node.x)
            .min()
            .unwrap_or(last_right + need);
        let gap = right.saturating_sub(last_right);
        if gap < need {
            shift_nodes_from_x(nodes, last_right, need - gap);
        }
    }
}

fn ensure_lr_forward_edge_gaps(nodes: &mut [NodeLayout], edges: &[Edge]) {
    loop {
        let mut inserted = false;
        for edge in edges {
            if edge.from == edge.to || edge.edge_type == EdgeType::Invisible {
                continue;
            }
            let Some(from) = nodes.iter().find(|node| node.id == edge.from).cloned() else {
                continue;
            };
            let Some(to) = nodes.iter().find(|node| node.id == edge.to).cloned() else {
                continue;
            };
            if to.x < from.x + from.width {
                continue;
            }
            let label_cols = edge.label.as_deref().map(multiline_width).unwrap_or(0);
            let jog_cols = if from.center_y == to.center_y { 0 } else { 2 };
            let need = from.x + from.width + 1 + label_cols + jog_cols;
            if to.x < need {
                shift_nodes_from_x(nodes, to.x, need - to.x);
                inserted = true;
                break;
            }
        }
        if !inserted {
            break;
        }
    }
}

fn finish_lr_layout(
    diagram: &GraphDiagram,
    node_layouts: Vec<NodeLayout>,
    subgraphs: Vec<SubgraphLayout>,
) -> GraphLayout {
    let mut edges: Vec<EdgeLayout> = diagram
        .edges
        .iter()
        .filter(|edge| edge.edge_type != EdgeType::Invisible)
        .map(|edge| EdgeLayout {
            from_id: edge.from.clone(),
            to_id: edge.to.clone(),
            edge_type: edge.edge_type,
            label: edge.label.clone(),
            route: EdgeRoute::Forward,
        })
        .collect();
    let endpoints = edge_endpoint_nodes(&node_layouts, &subgraphs);
    assign_edge_routes(&diagram.direction, &endpoints, &mut edges);
    let (mut width, mut height) = base_extents(&node_layouts, &subgraphs);
    for edge in &diagram.edges {
        if edge.from == edge.to
            && let Some(nl) = node_layouts.iter().find(|node| node.id == edge.from)
        {
            let label_w = edge
                .label
                .as_ref()
                .map(|label| display_width(label))
                .unwrap_or(0);
            width = width.max(nl.x + nl.width + 2 + label_w);
            height = height.max(nl.y + nl.height + 1);
        }
    }
    reserve_back_edge_space(&diagram.direction, &edges, &mut width, &mut height);
    GraphLayout {
        nodes: node_layouts,
        edges,
        subgraphs,
        width,
        height,
        direction: diagram.direction.clone(),
    }
}

fn layout_lr_shared_ranks(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    debug_assert!(
        !diagram.is_nested(),
        "nested diagrams go through layout_nested"
    );
    layout_lr_shared_ranks_with_gap(diagram, LR_GAP)
}

fn layout_lr_shared_ranks_with_gap(
    diagram: &GraphDiagram,
    min_gap: usize,
) -> Result<GraphLayout, String> {
    let ranks = ranks_with_subgraph_endpoints(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);
    let node_to_subgraph = node_to_subgraph_index(diagram);
    let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
    for node in &diagram.nodes {
        ranks_nodes[ranks[&node.id]].push(node);
    }
    for rank_nodes in &mut ranks_nodes {
        rank_nodes.sort_by_key(|node| {
            node_to_subgraph
                .get(node.id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }

    let mut node_layouts = layout_lr_with_gap(&ranks_nodes, &ranks, &diagram.edges, min_gap);
    apply_lr_bands(
        diagram,
        &ranks,
        &ranks_nodes,
        &mut node_layouts,
        &node_to_subgraph,
        true,
    );
    insert_lr_subgraph_chrome(diagram, &mut node_layouts);
    ensure_lr_forward_edge_gaps(&mut node_layouts, &diagram.edges);
    if node_layouts
        .iter()
        .any(|node| node_to_subgraph.contains_key(node.id.as_str()) && node.y < SUBGRAPH_PAD_TOP)
    {
        shift_all_nodes(&mut node_layouts, 0, SUBGRAPH_PAD_TOP);
    }
    if node_layouts
        .iter()
        .any(|node| node_to_subgraph.contains_key(node.id.as_str()) && node.x < SUBGRAPH_PAD_LEFT)
    {
        shift_all_nodes(&mut node_layouts, SUBGRAPH_PAD_LEFT, 0);
    }
    let subgraphs = apply_lr_bands(
        diagram,
        &ranks,
        &ranks_nodes,
        &mut node_layouts,
        &node_to_subgraph,
        false,
    );
    Ok(finish_lr_layout(diagram, node_layouts, subgraphs))
}

fn layout_with_subgraphs(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    debug_assert!(
        !diagram.is_nested(),
        "nested diagrams go through layout_nested"
    );
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
    let diagram_ranks = assign_ranks(diagram);

    for (i, sg_diagram) in sg_groups.iter().enumerate() {
        if sg_diagram.nodes.is_empty() {
            continue;
        }

        let mut used_ranks: Vec<usize> = sg_diagram
            .nodes
            .iter()
            .map(|node| diagram_ranks[&node.id])
            .collect();
        used_ranks.sort_unstable();
        used_ranks.dedup();
        let ranks: HashMap<String, usize> = sg_diagram
            .nodes
            .iter()
            .map(|node| {
                let dense_rank = used_ranks.binary_search(&diagram_ranks[&node.id]).unwrap();
                (node.id.clone(), dense_rank)
            })
            .collect();
        let max_rank = *ranks.values().max().unwrap_or(&0);
        let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
        for node in &sg_diagram.nodes {
            let rank = ranks[&node.id];
            ranks_nodes[rank].push(node);
        }

        let mut node_layouts = match diagram.direction {
            Direction::TopDown => layout_td(&ranks_nodes, &sg_diagram.edges),
            Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &sg_diagram.edges),
        };

        if diagram.direction == Direction::LeftRight {
            let external_targets: HashSet<&str> = diagram
                .edges
                .iter()
                .filter(|edge| node_to_subgraph.get(&edge.to) == Some(&i))
                .filter(|edge| node_to_subgraph.get(&edge.from) != Some(&i))
                .map(|edge| edge.to.as_str())
                .collect();
            let mut next_y = node_layouts
                .iter()
                .filter(|node| !external_targets.contains(node.id.as_str()))
                .map(|node| node.y + node.height)
                .max()
                .unwrap_or(0);
            if next_y > 0 && !external_targets.is_empty() {
                next_y += LR_NODE_VERTICAL_GAP;
            }
            for node in node_layouts
                .iter_mut()
                .filter(|node| external_targets.contains(node.id.as_str()))
            {
                node.y = next_y;
                node.center_y = next_y + node.height / 2;
                next_y += node.height + LR_NODE_VERTICAL_GAP;
            }
        }

        // Apply subgraph padding
        let sg = &diagram.subgraphs[i];
        let top_padding = SUBGRAPH_PAD_TOP
            + if diagram.edges.iter().any(|edge| {
                node_to_subgraph.get(&edge.to) == Some(&i)
                    && node_to_subgraph.get(&edge.from) != Some(&i)
            }) {
                TD_RANK_SPACING
            } else {
                0
            };
        for nl in &mut node_layouts {
            nl.x += x_offset + SUBGRAPH_PAD_LEFT;
            nl.y += top_padding;
            nl.center_x += x_offset + SUBGRAPH_PAD_LEFT;
            nl.center_y += top_padding;
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
        let bottom_padding = SUBGRAPH_PAD_BOTTOM
            + if diagram.edges.iter().any(|edge| {
                node_to_subgraph.get(&edge.from) == Some(&i)
                    && node_to_subgraph.get(&edge.to) != Some(&i)
            }) {
                1
            } else {
                0
            };
        let sg_height = content_bottom + bottom_padding;
        let content_offset = sg_width / 2 - content_width / 2;
        for node in &mut node_layouts {
            node.x += content_offset;
            node.center_x += content_offset;
        }

        sg_layouts.push(SubgraphLayout {
            id: sg.id.clone(),
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
            Direction::TopDown => layout_td(&ranks_nodes, &bare_diagram.edges),
            Direction::LeftRight => layout_lr(&ranks_nodes, &ranks, &bare_diagram.edges),
        };

        for nl in &mut node_layouts {
            nl.x += x_offset;
            nl.center_x += x_offset;
        }

        all_nodes.extend(node_layouts);
    }

    let mut edges: Vec<EdgeLayout> = diagram
        .edges
        .iter()
        .filter(|e| e.edge_type != EdgeType::Invisible)
        .map(|e| EdgeLayout {
            from_id: e.from.clone(),
            to_id: e.to.clone(),
            edge_type: e.edge_type,
            label: e.label.clone(),
            route: EdgeRoute::Forward,
        })
        .collect();

    let endpoints = edge_endpoint_nodes(&all_nodes, &sg_layouts);
    assign_edge_routes(&diagram.direction, &endpoints, &mut edges);
    let (mut width, mut height) = base_extents(&all_nodes, &sg_layouts);
    reserve_back_edge_space(&diagram.direction, &edges, &mut width, &mut height);

    let mut layout = GraphLayout {
        nodes: all_nodes,
        edges,
        subgraphs: sg_layouts,
        width,
        height,
        direction: diagram.direction.clone(),
    };
    reserve_subgraph_entry_space(&mut layout);

    let subgraph_ids: HashSet<&str> = diagram
        .subgraphs
        .iter()
        .map(|subgraph| subgraph.id.as_str())
        .collect();
    let node_to_subgraph: HashMap<&str, usize> = diagram
        .subgraphs
        .iter()
        .enumerate()
        .flat_map(|(i, sg)| sg.node_ids.iter().map(move |id| (id.as_str(), i)))
        .collect();
    let has_invisible_subgraph_constraint = diagram.edges.iter().any(|edge| {
        edge.edge_type == EdgeType::Invisible
            && subgraph_ids.contains(edge.from.as_str())
            && subgraph_ids.contains(edge.to.as_str())
    });
    let has_cross_subgraph_edge = diagram.edges.iter().any(|edge| {
        let from_sg = node_to_subgraph.get(edge.from.as_str());
        let to_sg = node_to_subgraph.get(edge.to.as_str());
        from_sg != to_sg && (from_sg.is_some() || to_sg.is_some())
    });
    let has_subgraph_endpoint = diagram.edges.iter().any(|edge| {
        subgraph_ids.contains(edge.from.as_str()) || subgraph_ids.contains(edge.to.as_str())
    });
    if diagram.direction == Direction::TopDown
        && (has_invisible_subgraph_constraint || has_subgraph_endpoint || has_cross_subgraph_edge)
    {
        stack_subgraphs(diagram, layout, usize::MAX)
    } else {
        clear_entry_title_collisions(&mut layout);
        Ok(layout)
    }
}

/// Ranks every node so that each forward edge points from a lower rank to a
/// higher one. Edges that close a cycle are dropped from the ranking (they are
/// drawn as back edges) — keeping them would produce ranks that contradict the
/// edge direction the renderer draws from.
fn assign_ranks(diagram: &GraphDiagram) -> HashMap<String, usize> {
    let back_edges = find_back_edges(diagram);

    let mut out_edges: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for node in &diagram.nodes {
        out_edges.entry(node.id.as_str()).or_default();
        in_degree.entry(node.id.as_str()).or_insert(0);
    }
    for (index, edge) in diagram.edges.iter().enumerate() {
        if edge.from == edge.to || back_edges.contains(&index) {
            continue;
        }
        out_edges
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
        *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
    }

    // Longest path from the sources, in declaration order (Kahn).
    let mut ranks: HashMap<String, usize> = HashMap::new();
    let mut queue: Vec<&str> = diagram
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .filter(|id| in_degree[id] == 0)
        .collect();
    for id in &queue {
        ranks.insert((*id).to_string(), 0);
    }

    let mut head = 0;
    while head < queue.len() {
        let id = queue[head];
        head += 1;
        let rank = ranks[id];
        let targets: &[&str] = out_edges.get(id).map(Vec::as_slice).unwrap_or(&[]);
        for target in targets.iter().copied() {
            let entry = ranks.entry(target.to_string()).or_insert(0);
            *entry = (*entry).max(rank + 1);
            if let Some(degree) = in_degree.get_mut(target) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push(target);
                }
            }
        }
    }

    // Every node is reachable once back edges are removed, but stay total.
    for node in &diagram.nodes {
        ranks.entry(node.id.clone()).or_insert(0);
    }

    ranks
}

/// Indices of the edges that point back to a node already on the DFS stack.
fn find_back_edges(diagram: &GraphDiagram) -> HashSet<usize> {
    let mut out_edges: HashMap<&str, Vec<(usize, &str)>> = HashMap::new();
    for node in &diagram.nodes {
        out_edges.entry(node.id.as_str()).or_default();
    }
    for (index, edge) in diagram.edges.iter().enumerate() {
        if edge.from == edge.to {
            continue;
        }
        out_edges
            .entry(edge.from.as_str())
            .or_default()
            .push((index, edge.to.as_str()));
    }

    let mut back_edges = HashSet::new();
    let mut done: HashSet<&str> = HashSet::new();
    let mut on_stack: HashSet<&str> = HashSet::new();

    for node in &diagram.nodes {
        let root = node.id.as_str();
        if done.contains(root) {
            continue;
        }
        // (node, index of the next outgoing edge to visit)
        let mut stack: Vec<(&str, usize)> = vec![(root, 0)];
        on_stack.insert(root);

        while let Some((id, cursor)) = stack.pop() {
            let targets = out_edges.get(id).map(Vec::as_slice).unwrap_or(&[]);
            if cursor < targets.len() {
                let (edge_index, target) = targets[cursor];
                stack.push((id, cursor + 1));
                if on_stack.contains(target) {
                    back_edges.insert(edge_index);
                } else if !done.contains(target) {
                    on_stack.insert(target);
                    stack.push((target, 0));
                }
            } else {
                on_stack.remove(id);
                done.insert(id);
            }
        }
    }

    back_edges
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

    if diagram.is_nested() {
        return layout_nested_within_width(diagram, max_width);
    }

    if !diagram.subgraphs.is_empty() {
        if outer_node_on_subgraph_ranks(diagram) {
            match diagram.direction {
                Direction::TopDown => {
                    for node_gap in (0..=TD_NODE_GAP).rev() {
                        let layout = layout_td_shared_ranks_with_gap(diagram, node_gap)?;
                        if layout.width <= max_width {
                            return Ok(layout);
                        }
                    }
                    return Err(format!("graph diagram too wide for {max_width} columns"));
                }
                Direction::LeftRight => {
                    for lr_gap in (1..=LR_GAP).rev() {
                        let layout = layout_lr_shared_ranks_with_gap(diagram, lr_gap)?;
                        if layout.width <= max_width {
                            return Ok(layout);
                        }
                    }
                    return Err(format!("graph diagram too wide for {max_width} columns"));
                }
            }
        }
        let mut vertical = diagram.clone();
        vertical.direction = Direction::TopDown;
        let layout = layout_with_subgraphs(&vertical)?;
        return stack_subgraphs(&vertical, layout, max_width);
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
                Direction::TopDown => layout_td_with_gap(&ranks_nodes, &diagram.edges, node_gap),
                Direction::LeftRight => {
                    layout_lr_with_gap(&ranks_nodes, &ranks, &diagram.edges, lr_gap)
                }
            };

            let mut edges: Vec<EdgeLayout> = diagram
                .edges
                .iter()
                .filter(|e| e.edge_type != EdgeType::Invisible)
                .map(|e| EdgeLayout {
                    from_id: e.from.clone(),
                    to_id: e.to.clone(),
                    edge_type: e.edge_type,
                    label: e.label.clone(),
                    route: EdgeRoute::Forward,
                })
                .collect();

            let subgraphs = compute_subgraph_layouts(&diagram.subgraphs, &mut node_layouts);

            assign_edge_routes(&diagram.direction, &node_layouts, &mut edges);
            let (mut width, mut height) = base_extents(&node_layouts, &subgraphs);
            if diagram.direction == Direction::TopDown {
                width = width.max(td_labels_right(&node_layouts, &diagram.edges));
            }
            reserve_back_edge_space(&diagram.direction, &edges, &mut width, &mut height);

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

    // Preserve max_width without hiding a valid graph: narrow LR graphs can be
    // reflowed vertically while keeping the same topology.
    if diagram.direction == Direction::LeftRight {
        let mut vertical = diagram.clone();
        vertical.direction = Direction::TopDown;
        if let Ok(layout) = compute_with_max_width(&vertical, max_width) {
            return Ok(layout);
        }
    }

    Err(format!("graph diagram too wide for {max_width} columns"))
}

fn stack_subgraphs(
    diagram: &GraphDiagram,
    mut layout: GraphLayout,
    max_width: usize,
) -> Result<GraphLayout, String> {
    debug_assert!(
        !diagram.is_nested(),
        "nested diagrams go through layout_nested"
    );
    let node_to_subgraph: HashMap<&str, usize> = diagram
        .subgraphs
        .iter()
        .enumerate()
        .flat_map(|(index, sg)| sg.node_ids.iter().map(move |id| (id.as_str(), index)))
        .collect();
    let mut groups = GraphDiagram {
        direction: Direction::TopDown,
        nodes: diagram
            .subgraphs
            .iter()
            .filter(|sg| !sg.node_ids.is_empty())
            .map(|sg| NodeDecl {
                id: sg.id.clone(),
                label: sg.label.clone(),
                shape: NodeShape::Box,
            })
            .chain(
                diagram
                    .nodes
                    .iter()
                    .filter(|node| !node_to_subgraph.contains_key(node.id.as_str()))
                    .cloned(),
            )
            .collect(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
    };
    let group_id = |id: &str| {
        node_to_subgraph.get(id).map_or_else(
            || id.to_string(),
            |index| diagram.subgraphs[*index].id.clone(),
        )
    };
    groups.edges = diagram
        .edges
        .iter()
        .filter_map(|edge| {
            let from = group_id(&edge.from);
            let to = group_id(&edge.to);
            (from != to).then_some(Edge {
                from,
                to,
                edge_type: edge.edge_type,
                label: None,
            })
        })
        .collect();
    let ranks = assign_ranks(&groups);
    groups.nodes.sort_by_key(|node| ranks[&node.id]);
    let center_x = layout
        .subgraphs
        .iter()
        .map(|sg| sg.width / 2)
        .chain(
            layout
                .nodes
                .iter()
                .filter(|node| !node_to_subgraph.contains_key(node.id.as_str()))
                .map(|node| node.width / 2),
        )
        .max()
        .unwrap_or(0);
    let mut y_offset = 0;

    for group in groups.nodes {
        if let Some(sg_layout) = layout.subgraphs.iter_mut().find(|sg| sg.id == group.id) {
            if sg_layout.width > max_width {
                return Err(format!("graph diagram too wide for {max_width} columns"));
            }
            let old_x = sg_layout.x;
            let old_y = sg_layout.y;
            let x_offset = center_x - sg_layout.width / 2;
            for node in layout.nodes.iter_mut().filter(|node| {
                node_to_subgraph
                    .get(node.id.as_str())
                    .is_some_and(|index| diagram.subgraphs[*index].id == group.id)
            }) {
                node.x = node.x - old_x + x_offset;
                node.y = node.y - old_y + y_offset;
                node.center_x = node.center_x - old_x + x_offset;
                node.center_y = node.center_y - old_y + y_offset;
            }
            sg_layout.x = x_offset;
            sg_layout.y = y_offset;
            y_offset += sg_layout.height + TD_RANK_SPACING;
        } else if let Some(node) = layout.nodes.iter_mut().find(|node| node.id == group.id) {
            if node.width > max_width {
                return Err(format!("graph diagram too wide for {max_width} columns"));
            }
            node.x = center_x - node.width / 2;
            node.y = y_offset;
            node.center_x = center_x;
            node.center_y = y_offset + node.height / 2;
            y_offset += node.height + TD_RANK_SPACING;
        }
    }

    layout.direction = Direction::TopDown;
    let endpoints = edge_endpoint_nodes(&layout.nodes, &layout.subgraphs);
    assign_edge_routes(&layout.direction, &endpoints, &mut layout.edges);
    let (mut width, mut height) = base_extents(&layout.nodes, &layout.subgraphs);
    reserve_back_edge_space(&layout.direction, &layout.edges, &mut width, &mut height);
    layout.width = width;
    layout.height = height;
    reserve_subgraph_entry_space(&mut layout);
    clear_entry_title_collisions(&mut layout);
    let (width, height) = base_extents(&layout.nodes, &layout.subgraphs);
    layout.width = layout.width.max(width);
    layout.height = layout.height.max(height);
    if layout.width > max_width {
        return Err(format!("graph diagram too wide for {max_width} columns"));
    }
    Ok(layout)
}

fn td_label_width(node_id: &str, edges: &[Edge]) -> usize {
    edges
        .iter()
        .filter(|edge| {
            edge.edge_type != EdgeType::Invisible && (edge.from == node_id || edge.to == node_id)
        })
        .filter_map(|edge| edge.label.as_deref())
        .map(multiline_width)
        .max()
        .unwrap_or(0)
}

fn td_labels_right(nodes: &[NodeLayout], edges: &[Edge]) -> usize {
    nodes
        .iter()
        .map(|node| {
            let width = td_label_width(&node.id, edges);
            node.center_x.saturating_sub(width / 2) + width
        })
        .max()
        .unwrap_or(0)
}

fn layout_td(ranks_nodes: &[Vec<&NodeDecl>], edges: &[Edge]) -> Vec<NodeLayout> {
    layout_td_with_gap(ranks_nodes, edges, TD_NODE_GAP)
}

fn layout_td_with_gap(
    ranks_nodes: &[Vec<&NodeDecl>],
    edges: &[Edge],
    node_gap: usize,
) -> Vec<NodeLayout> {
    let mut layouts = Vec::new();

    let mut rank_widths: Vec<usize> = Vec::new();
    for rank_nodes in ranks_nodes {
        let total: usize = rank_nodes
            .iter()
            .map(|n| box_width(&n.label, n.shape).max(td_label_width(&n.id, edges) + 2))
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
            .map(|n| box_height(&n.label, n.shape))
            .max()
            .unwrap_or(BOX_HEIGHT);
        rank_heights.push(max_h);
    }

    let mut y = 0;
    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        let rank_total = rank_widths[rank];
        let base_x = if max_width > rank_total {
            max_width / 2 - rank_total / 2
        } else {
            0
        };

        let mut x = base_x;

        for node in rank_nodes {
            let w = box_width(&node.label, node.shape);
            let h = box_height(&node.label, node.shape);
            let footprint = w.max(td_label_width(&node.id, edges) + 2);
            let node_x = x + footprint / 2 - w / 2;
            layouts.push(NodeLayout {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape,
                x: node_x,
                y,
                width: w,
                height: h,
                center_x: node_x + w / 2,
                center_y: y + h / 2,
            });
            x += footprint + node_gap;
        }

        let label_spacing = edges
            .iter()
            .filter(|edge| {
                edge.edge_type != EdgeType::Invisible
                    && edge.from != edge.to
                    && rank_nodes.iter().any(|node| node.id == edge.from)
            })
            .filter_map(|edge| {
                edge.label.as_deref().map(|label| {
                    let branches = edges
                        .iter()
                        .filter(|other| {
                            other.edge_type != EdgeType::Invisible
                                && other.from != other.to
                                && (other.from == edge.from || other.to == edge.to)
                        })
                        .count();
                    line_count(label) + if branches > 1 { 2 } else { 1 }
                })
            })
            .max()
            .unwrap_or(0);
        y += rank_heights[rank] + TD_RANK_SPACING.max(label_spacing);
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
    let mut labeled_child_counts: HashMap<&str, usize> = HashMap::new();
    for edge in edges.iter().filter(|edge| {
        edge.edge_type != EdgeType::Invisible
            && edge.label.is_some()
            && ranks.get(&edge.from) < ranks.get(&edge.to)
    }) {
        *labeled_child_counts.entry(&edge.from).or_default() += 1;
    }
    let mut label_heights: HashMap<&str, usize> = HashMap::new();
    for edge in edges.iter().filter(|edge| {
        edge.edge_type != EdgeType::Invisible && ranks.get(&edge.from) < ranks.get(&edge.to)
    }) {
        if let Some(label) = &edge.label {
            let anchor = if labeled_child_counts
                .get(edge.from.as_str())
                .copied()
                .unwrap_or(0)
                > 1
            {
                &edge.to
            } else {
                &edge.from
            };
            let height = label_heights.entry(anchor).or_default();
            *height = (*height).max(line_count(label));
        }
    }

    for (rank, rank_nodes) in ranks_nodes.iter().enumerate() {
        let rank_max_width = rank_nodes
            .iter()
            .map(|n| box_width(&n.label, n.shape))
            .max()
            .unwrap_or(0);
        let mut y = 0;

        for node in rank_nodes {
            let w = box_width(&node.label, node.shape);
            let h = box_height(&node.label, node.shape);
            y += label_heights
                .get(node.id.as_str())
                .copied()
                .unwrap_or(0)
                .saturating_sub(h / 2);
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
            let gap = min_gap.max(lr_rank_gap(edges, ranks, rank));
            rank_x += rank_max_width + gap;
        }
    }

    // A diamond is taller than a box, so top-aligning ranks puts their text on
    // different rows and the edge between them has to bend. Shift each rank so
    // the top node's text row matches.
    align_lr_baselines(&mut layouts);
    layouts
}

/// Columns between this rank and the next.
///
/// A labeled fan-out spends one column on the dash out of the source and the
/// next on the vertical bar. Both are outside the label, so the longest label
/// keeps a space after the bar.
fn lr_rank_gap(edges: &[Edge], ranks: &HashMap<String, usize>, rank: usize) -> usize {
    let crossing: Vec<&Edge> = edges
        .iter()
        .filter(|edge| {
            edge.edge_type != EdgeType::Invisible
                && ranks
                    .get(&edge.from)
                    .is_some_and(|from_rank| *from_rank <= rank)
                && ranks.get(&edge.to).is_some_and(|to_rank| *to_rank > rank)
        })
        .collect();

    let label_gap = crossing
        .iter()
        .filter_map(|edge| edge.label.as_ref().map(|label| multiline_width(label) + 2))
        .max()
        .unwrap_or(0);
    let fan_out = crossing.iter().any(|edge| {
        edge.label.is_some()
            && crossing
                .iter()
                .filter(|other| other.from == edge.from && other.label.is_some())
                .count()
                > 1
    });
    if fan_out && label_gap > 0 {
        label_gap + 2
    } else {
        label_gap
    }
}

fn align_lr_baselines(nodes: &mut [NodeLayout]) {
    let mut top_y: HashMap<usize, usize> = HashMap::new();
    for node in nodes.iter() {
        let top = top_y.entry(node.x).or_insert(usize::MAX);
        *top = (*top).min(node.y);
    }

    let mut top_center: HashMap<usize, usize> = HashMap::new();
    let mut baseline = 0;
    for node in nodes.iter() {
        if node.y == top_y[&node.x] {
            top_center.insert(node.x, node.center_y);
            baseline = baseline.max(node.center_y);
        }
    }

    for node in nodes.iter_mut() {
        let shift = baseline - top_center[&node.x];
        node.y += shift;
        node.center_y += shift;
    }
}

fn box_width(label: &str, shape: NodeShape) -> usize {
    let base = multiline_width(label) + 4;
    match shape {
        NodeShape::Circle => base + 4,
        NodeShape::Subroutine | NodeShape::Hexagon => base + 2,
        _ => base,
    }
}

fn box_height(label: &str, shape: NodeShape) -> usize {
    match shape {
        NodeShape::Diamond => 4 + line_count(label),
        NodeShape::Cylinder => 3 + line_count(label),
        _ => 2 + line_count(label),
    }
}

const SUBGRAPH_PAD_LEFT: usize = 2;
const SUBGRAPH_PAD_RIGHT: usize = 2;
const SUBGRAPH_PAD_TOP: usize = 1;
const SUBGRAPH_PAD_BOTTOM: usize = 1;
const MIN_FRAME_SEPARATION: usize = SUBGRAPH_PAD_TOP;
const SUBGRAPH_TITLE_DECOR: usize = 6;
const SUBGRAPH_TITLE_TEXT_OFFSET: usize = 3;

pub(crate) fn subgraph_title_col(sg: &SubgraphLayout) -> usize {
    sg.x + SUBGRAPH_TITLE_TEXT_OFFSET
}

pub(crate) fn subgraph_title_reserved_end(sg: &SubgraphLayout) -> usize {
    subgraph_title_col(sg) + display_width(&sg.label) + 2
}

fn node_in_subgraph(node: &NodeLayout, sg: &SubgraphLayout) -> bool {
    node.x >= sg.x
        && node.x + node.width <= sg.x + sg.width
        && node.y >= sg.y
        && node.y + node.height <= sg.y + sg.height
}

fn top_entry_centers(layout: &GraphLayout, sg: &SubgraphLayout) -> Vec<usize> {
    layout
        .nodes
        .iter()
        .filter(|node| node_in_subgraph(node, sg))
        .filter(|node| {
            layout.edges.iter().any(|edge| {
                edge.to_id == node.id
                    && layout
                        .nodes
                        .iter()
                        .any(|from| from.id == edge.from_id && from.y + from.height <= sg.y)
            })
        })
        .map(|node| node.center_x)
        .collect()
}

fn clear_entry_title_collisions(layout: &mut GraphLayout) {
    let mut order: Vec<usize> = (0..layout.subgraphs.len()).collect();
    order.sort_by_key(|&i| (layout.subgraphs[i].x, layout.subgraphs[i].y));
    for i in order {
        let reserved_end = subgraph_title_reserved_end(&layout.subgraphs[i]);
        let entry_centers = top_entry_centers(layout, &layout.subgraphs[i]);
        let Some(&leftmost) = entry_centers.iter().min() else {
            continue;
        };
        let shift = reserved_end.saturating_sub(leftmost);
        if shift == 0 {
            continue;
        }
        let sg_x = layout.subgraphs[i].x;
        for node in &mut layout.nodes {
            if node.x >= sg_x {
                node.x += shift;
                node.center_x += shift;
            }
        }
        for (j, other) in layout.subgraphs.iter_mut().enumerate() {
            if j == i {
                other.width += shift;
            } else if other.x >= sg_x {
                other.x += shift;
            }
        }
        layout.width += shift;
    }
}

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
            id: sg.id.clone(),
            label: sg.label.clone(),
            x: min_x,
            y: min_y,
            width,
            height,
        });
    }

    sg_layouts
}

fn layout_nested(diagram: &GraphDiagram) -> Result<GraphLayout, String> {
    layout_nested_with_gaps(diagram, TD_NODE_GAP, LR_GAP)
}

fn layout_nested_within_width(
    diagram: &GraphDiagram,
    max_width: usize,
) -> Result<GraphLayout, String> {
    match diagram.direction {
        Direction::TopDown => {
            for node_gap in (0..TD_NODE_GAP).rev() {
                let layout = layout_nested_with_gaps(diagram, node_gap, LR_GAP)?;
                if layout.width <= max_width {
                    return Ok(layout);
                }
            }
        }
        Direction::LeftRight => {
            for lr_gap in (1..LR_GAP).rev() {
                let layout = layout_nested_with_gaps(diagram, TD_NODE_GAP, lr_gap)?;
                if layout.width <= max_width {
                    return Ok(layout);
                }
            }
        }
    }
    Err(format!("graph diagram too wide for {max_width} columns"))
}

fn layout_nested_with_gaps(
    diagram: &GraphDiagram,
    node_gap: usize,
    lr_gap: usize,
) -> Result<GraphLayout, String> {
    let roots = roots_diagram(diagram);
    let mut placed = if outer_node_on_subgraph_ranks(&roots) {
        match diagram.direction {
            Direction::TopDown => layout_td_shared_ranks_with_gap(&roots, node_gap)?,
            Direction::LeftRight => layout_lr_shared_ranks_with_gap(&roots, lr_gap)?,
        }
    } else {
        place_remaining_groups(diagram, node_gap, lr_gap)?
    };
    placed.subgraphs.clear();
    wrap_frames_inside_out(diagram, &mut placed);
    finish_routes(&mut placed, diagram);
    Ok(placed)
}

fn roots_diagram(diagram: &GraphDiagram) -> GraphDiagram {
    let subgraphs = diagram
        .roots()
        .into_iter()
        .map(|index| {
            let sg = &diagram.subgraphs[index.get()];
            Subgraph {
                id: sg.id.clone(),
                label: sg.label.clone(),
                node_ids: sg.node_ids.clone(),
                parent: None,
            }
        })
        .collect();
    GraphDiagram {
        direction: diagram.direction.clone(),
        nodes: diagram.nodes.clone(),
        edges: diagram.edges.clone(),
        subgraphs,
    }
}

struct PlacedGroup {
    id: String,
    nodes: Vec<NodeLayout>,
}

fn place_remaining_groups(
    diagram: &GraphDiagram,
    node_gap: usize,
    lr_gap: usize,
) -> Result<GraphLayout, String> {
    let mut groups = Vec::new();
    for root in diagram.roots() {
        let sg = &diagram.subgraphs[root.get()];
        let mut closure = Vec::new();
        collect_exclusive(diagram, root, &mut closure);
        let mut sorted_closure = closure.clone();
        let mut closed: Vec<&str> = sg.node_ids.iter().map(String::as_str).collect();
        sorted_closure.sort_unstable();
        closed.sort_unstable();
        debug_assert_eq!(sorted_closure, closed);
        if closure.is_empty() {
            continue;
        }
        let id_set: HashSet<&str> = closure.into_iter().collect();
        let nodes: Vec<NodeDecl> = diagram
            .nodes
            .iter()
            .filter(|node| id_set.contains(node.id.as_str()))
            .cloned()
            .collect();
        let edges: Vec<Edge> = diagram
            .edges
            .iter()
            .filter(|edge| id_set.contains(edge.from.as_str()) && id_set.contains(edge.to.as_str()))
            .cloned()
            .collect();
        let mut laid = layout_members(diagram, &nodes, &edges, node_gap, lr_gap);
        align_single_node_ranks(&mut laid, &diagram.direction);
        groups.push(PlacedGroup {
            id: sg.id.clone(),
            nodes: laid,
        });
    }
    for node in &diagram.nodes {
        if diagram.innermost(&node.id).is_some() {
            continue;
        }
        let laid = layout_members(diagram, std::slice::from_ref(node), &[], node_gap, lr_gap);
        groups.push(PlacedGroup {
            id: node.id.clone(),
            nodes: laid,
        });
    }

    let ranks = contracted_ranks(diagram, &groups);
    pack_groups(diagram, &mut groups, &ranks, node_gap, lr_gap);
    Ok(GraphLayout {
        nodes: groups.into_iter().flat_map(|group| group.nodes).collect(),
        edges: forward_edges(diagram),
        subgraphs: Vec::new(),
        width: 0,
        height: 0,
        direction: diagram.direction.clone(),
    })
}

fn collect_exclusive<'a>(diagram: &'a GraphDiagram, index: SubgraphIndex, out: &mut Vec<&'a str>) {
    out.extend(diagram.exclusive_members(index));
    for child in diagram.children(index) {
        collect_exclusive(diagram, child, out);
    }
}

fn forward_edges(diagram: &GraphDiagram) -> Vec<EdgeLayout> {
    diagram
        .edges
        .iter()
        .filter(|edge| edge.edge_type != EdgeType::Invisible)
        .map(|edge| EdgeLayout {
            from_id: edge.from.clone(),
            to_id: edge.to.clone(),
            edge_type: edge.edge_type,
            label: edge.label.clone(),
            route: EdgeRoute::Forward,
        })
        .collect()
}

fn layout_members(
    diagram: &GraphDiagram,
    nodes: &[NodeDecl],
    edges: &[Edge],
    node_gap: usize,
    lr_gap: usize,
) -> Vec<NodeLayout> {
    let mini = GraphDiagram {
        direction: diagram.direction.clone(),
        nodes: nodes.to_vec(),
        edges: edges.to_vec(),
        subgraphs: Vec::new(),
    };
    let ranks = assign_ranks(&mini);
    let max_rank = *ranks.values().max().unwrap_or(&0);
    let mut ranks_nodes: Vec<Vec<&NodeDecl>> = vec![Vec::new(); max_rank + 1];
    for node in &mini.nodes {
        ranks_nodes[ranks[&node.id]].push(node);
    }
    match diagram.direction {
        Direction::TopDown => layout_td_with_gap(&ranks_nodes, &mini.edges, node_gap),
        Direction::LeftRight => layout_lr_with_gap(&ranks_nodes, &ranks, &mini.edges, lr_gap),
    }
}

fn align_single_node_ranks(nodes: &mut [NodeLayout], direction: &Direction) {
    let rank_of = |node: &NodeLayout| match direction {
        Direction::TopDown => node.y,
        Direction::LeftRight => node.x,
    };
    let center_of = |node: &NodeLayout| match direction {
        Direction::TopDown => node.center_x,
        Direction::LeftRight => node.center_y,
    };
    let mut keys: Vec<usize> = nodes.iter().map(rank_of).collect();
    keys.sort_unstable();
    keys.dedup();
    let singles: Vec<usize> = keys
        .into_iter()
        .filter(|key| nodes.iter().filter(|node| rank_of(node) == *key).count() == 1)
        .collect();
    if singles.len() < 2 {
        return;
    }
    let mut bump_x = 0isize;
    let mut bump_y = 0isize;
    let target = center_of(
        nodes
            .iter()
            .find(|node| rank_of(node) == singles[0])
            .unwrap(),
    );
    for node in nodes.iter() {
        if !singles.contains(&rank_of(node)) {
            continue;
        }
        let delta = target as isize - center_of(node) as isize;
        match direction {
            Direction::TopDown => bump_x = bump_x.max(-(node.x as isize + delta)),
            Direction::LeftRight => bump_y = bump_y.max(-(node.y as isize + delta)),
        }
    }
    if bump_x > 0 || bump_y > 0 {
        translate_nodes(nodes, bump_x, bump_y);
    }
    let target = center_of(
        nodes
            .iter()
            .find(|node| rank_of(node) == singles[0])
            .unwrap(),
    );
    for node in nodes.iter_mut() {
        if !singles.contains(&rank_of(node)) {
            continue;
        }
        let delta = target as isize - center_of(node) as isize;
        match direction {
            Direction::TopDown => translate_nodes(std::slice::from_mut(node), delta, 0),
            Direction::LeftRight => translate_nodes(std::slice::from_mut(node), 0, delta),
        }
    }
}

fn contracted_ranks(diagram: &GraphDiagram, groups: &[PlacedGroup]) -> HashMap<String, usize> {
    let mut owner: HashMap<&str, &str> = HashMap::new();
    for group in groups {
        for node in &group.nodes {
            owner.insert(node.id.as_str(), group.id.as_str());
        }
    }
    let mut contracted = GraphDiagram {
        direction: diagram.direction.clone(),
        nodes: groups
            .iter()
            .map(|group| NodeDecl {
                id: group.id.clone(),
                label: group.id.clone(),
                shape: NodeShape::Box,
            })
            .collect(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
    };
    for edge in &diagram.edges {
        let (Some(from), Some(to)) = (owner.get(edge.from.as_str()), owner.get(edge.to.as_str()))
        else {
            continue;
        };
        if from != to {
            contracted.edges.push(Edge {
                from: (*from).to_string(),
                to: (*to).to_string(),
                edge_type: edge.edge_type,
                label: None,
            });
        }
    }
    assign_ranks(&contracted)
}

fn pack_groups(
    diagram: &GraphDiagram,
    groups: &mut [PlacedGroup],
    ranks: &HashMap<String, usize>,
    node_gap: usize,
    lr_gap: usize,
) {
    if groups.is_empty() {
        return;
    }
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    let mut flow_at = 0usize;
    for rank in 0..=max_rank {
        let indexes: Vec<usize> = groups
            .iter()
            .enumerate()
            .filter(|(_, group)| ranks.get(&group.id).copied() == Some(rank))
            .map(|(index, _)| index)
            .collect();
        if indexes.is_empty() {
            continue;
        }
        let mut cross_at = 0usize;
        let mut rank_far = flow_at;
        for index in indexes {
            place_group_at(groups, index, &diagram.direction, flow_at, cross_at);
            align_group_entry(diagram, groups, index);
            rank_far = rank_far.max(
                groups[index]
                    .nodes
                    .iter()
                    .map(|node| flow_end(node, &diagram.direction))
                    .max()
                    .unwrap_or(flow_at),
            );
            let far_cross = groups[index]
                .nodes
                .iter()
                .map(|node| cross_end(node, &diagram.direction))
                .max()
                .unwrap_or(cross_at);
            cross_at = far_cross
                + match diagram.direction {
                    Direction::TopDown => node_gap.max(1),
                    Direction::LeftRight => LR_NODE_VERTICAL_GAP,
                };
        }
        let from_ids = node_ids_at(groups, ranks, |group_rank| group_rank == rank);
        let to_ids = node_ids_at(groups, ranks, |group_rank| group_rank > rank);
        flow_at = rank_far + flow_gap(diagram, &from_ids, &to_ids, lr_gap);
    }
}

fn node_ids_at(
    groups: &[PlacedGroup],
    ranks: &HashMap<String, usize>,
    keep: impl Fn(usize) -> bool,
) -> HashSet<String> {
    groups
        .iter()
        .filter(|group| ranks.get(&group.id).copied().is_some_and(&keep))
        .flat_map(|group| group.nodes.iter().map(|node| node.id.clone()))
        .collect()
}

fn flow_gap(
    diagram: &GraphDiagram,
    from_ids: &HashSet<String>,
    to_ids: &HashSet<String>,
    lr_gap: usize,
) -> usize {
    let crossing: Vec<&Edge> = diagram
        .edges
        .iter()
        .filter(|edge| {
            edge.edge_type != EdgeType::Invisible
                && from_ids.contains(&edge.from)
                && to_ids.contains(&edge.to)
        })
        .collect();
    match diagram.direction {
        Direction::TopDown => {
            let spacing = crossing
                .iter()
                .filter_map(|edge| {
                    edge.label.as_deref().map(|label| {
                        let branches = crossing
                            .iter()
                            .filter(|other| other.from == edge.from || other.to == edge.to)
                            .count();
                        line_count(label) + if branches > 1 { 2 } else { 1 }
                    })
                })
                .max()
                .unwrap_or(0);
            TD_RANK_SPACING.max(spacing)
        }
        Direction::LeftRight => {
            let spacing = crossing
                .iter()
                .filter_map(|edge| {
                    edge.label
                        .as_deref()
                        .map(|label| multiline_width(label) + 2)
                })
                .max()
                .unwrap_or(0);
            lr_gap.max(spacing).max(1)
        }
    }
}

fn place_group_at(
    groups: &mut [PlacedGroup],
    index: usize,
    direction: &Direction,
    flow_at: usize,
    cross_at: usize,
) {
    let min_flow = groups[index]
        .nodes
        .iter()
        .map(|node| flow_origin(node, direction))
        .min()
        .unwrap_or(0);
    let min_cross = groups[index]
        .nodes
        .iter()
        .map(|node| cross_origin(node, direction))
        .min()
        .unwrap_or(0);
    let d_flow = flow_at as isize - min_flow as isize;
    let d_cross = cross_at as isize - min_cross as isize;
    let (dx, dy) = match direction {
        Direction::TopDown => (d_cross, d_flow),
        Direction::LeftRight => (d_flow, d_cross),
    };
    translate_nodes(&mut groups[index].nodes, dx, dy);
}

fn align_group_entry(diagram: &GraphDiagram, groups: &mut [PlacedGroup], index: usize) {
    let delta = {
        let group_ids: HashSet<&str> = groups[index]
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        let mut delta = None;
        for node in &groups[index].nodes {
            let preds: Vec<&str> = diagram
                .edges
                .iter()
                .filter(|edge| {
                    edge.to == node.id
                        && edge.from != edge.to
                        && edge.edge_type != EdgeType::Invisible
                })
                .map(|edge| edge.from.as_str())
                .collect();
            if preds.len() != 1 || group_ids.contains(preds[0]) {
                continue;
            }
            let Some(pred) = groups
                .iter()
                .flat_map(|group| group.nodes.iter())
                .find(|other| other.id == preds[0])
            else {
                continue;
            };
            let (pred_center, node_center) = match diagram.direction {
                Direction::TopDown => (pred.center_x, node.center_x),
                Direction::LeftRight => (pred.center_y, node.center_y),
            };
            delta = Some(pred_center as isize - node_center as isize);
            break;
        }
        delta
    };
    let Some(delta) = delta else {
        return;
    };
    if delta == 0 {
        return;
    }
    let min_after = groups[index]
        .nodes
        .iter()
        .map(|node| match diagram.direction {
            Direction::TopDown => node.x as isize + delta,
            Direction::LeftRight => node.y as isize + delta,
        })
        .min()
        .unwrap_or(0);
    if min_after < 0 {
        let bump = -min_after;
        for group in groups.iter_mut() {
            match diagram.direction {
                Direction::TopDown => translate_nodes(&mut group.nodes, bump, 0),
                Direction::LeftRight => translate_nodes(&mut group.nodes, 0, bump),
            }
        }
    }
    match diagram.direction {
        Direction::TopDown => translate_nodes(&mut groups[index].nodes, delta, 0),
        Direction::LeftRight => translate_nodes(&mut groups[index].nodes, 0, delta),
    }
}

fn flow_origin(node: &NodeLayout, direction: &Direction) -> usize {
    match direction {
        Direction::TopDown => node.y,
        Direction::LeftRight => node.x,
    }
}

fn flow_end(node: &NodeLayout, direction: &Direction) -> usize {
    match direction {
        Direction::TopDown => node.y + node.height,
        Direction::LeftRight => node.x + node.width,
    }
}

fn cross_origin(node: &NodeLayout, direction: &Direction) -> usize {
    match direction {
        Direction::TopDown => node.x,
        Direction::LeftRight => node.y,
    }
}

fn cross_end(node: &NodeLayout, direction: &Direction) -> usize {
    match direction {
        Direction::TopDown => node.x + node.width,
        Direction::LeftRight => node.y + node.height,
    }
}

fn translate_nodes(nodes: &mut [NodeLayout], dx: isize, dy: isize) {
    for node in nodes {
        node.x = (node.x as isize + dx) as usize;
        node.y = (node.y as isize + dy) as usize;
        node.center_x = (node.center_x as isize + dx) as usize;
        node.center_y = (node.center_y as isize + dy) as usize;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy)]
struct FrameDraft {
    x: isize,
    y: isize,
    width: usize,
    height: usize,
}

impl FrameDraft {
    fn right(self) -> isize {
        self.x + self.width as isize
    }

    fn bottom(self) -> isize {
        self.y + self.height as isize
    }
}

struct Bounds {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

fn wrap_frames_inside_out(diagram: &GraphDiagram, layout: &mut GraphLayout) {
    for index in 0..diagram.subgraphs.len() {
        wrap_one(diagram, SubgraphIndex::new(index), layout);
    }
    clear_label_columns(diagram, layout);
}

fn wrap_one(diagram: &GraphDiagram, index: SubgraphIndex, layout: &mut GraphLayout) {
    let mut tries = 0;
    let draft = loop {
        tries += 1;
        let Some(draft) = fit_frame(diagram, index, layout) else {
            return;
        };
        if tries > 16 {
            break draft;
        }
        if repair_negative(layout, &draft) || repair_clearance(diagram, index, layout, &draft) {
            continue;
        }
        break draft;
    };
    let sg = &diagram.subgraphs[index.get()];
    let mut frame = SubgraphLayout {
        id: sg.id.clone(),
        label: sg.label.clone(),
        x: draft.x.max(0) as usize,
        y: draft.y.max(0) as usize,
        width: draft.width,
        height: draft.height,
    };
    repair_title(diagram, index, layout, &mut frame);
    for child in diagram.children(index) {
        let child_id = diagram.subgraphs[child.get()].id.as_str();
        if let Some(inner) = layout.subgraphs.iter().find(|sg| sg.id == child_id) {
            debug_assert!(
                strictly_contains(&frame, inner),
                "child frame {} must sit inside {}",
                inner.id,
                frame.id
            );
        }
    }
    layout.subgraphs.push(frame);
}

fn strictly_contains(outer: &SubgraphLayout, inner: &SubgraphLayout) -> bool {
    let inner_bottom = inner.y + inner.height - 1;
    let outer_bottom = outer.y + outer.height - 1;
    let inner_right = inner.x + inner.width - 1;
    let outer_right = outer.x + outer.width - 1;
    inner.y >= outer.y + MIN_FRAME_SEPARATION
        && inner_bottom + MIN_FRAME_SEPARATION <= outer_bottom
        && inner.x >= outer.x + MIN_FRAME_SEPARATION
        && inner_right + MIN_FRAME_SEPARATION <= outer_right
}

fn fit_frame(
    diagram: &GraphDiagram,
    index: SubgraphIndex,
    layout: &GraphLayout,
) -> Option<FrameDraft> {
    let exclusive = diagram.exclusive_members(index);
    let members: Vec<&NodeLayout> = layout
        .nodes
        .iter()
        .filter(|node| exclusive.iter().any(|id| *id == node.id))
        .collect();
    let child_ids: Vec<&str> = diagram
        .children(index)
        .iter()
        .map(|child| diagram.subgraphs[child.get()].id.as_str())
        .collect();
    let children: Vec<&SubgraphLayout> = layout
        .subgraphs
        .iter()
        .filter(|sg| child_ids.contains(&sg.id.as_str()))
        .collect();
    if members.is_empty() && children.is_empty() {
        return None;
    }
    let mut bounds = None;
    for node in &members {
        include_bounds(
            &mut bounds,
            node.x,
            node.y,
            node.x + node.width,
            node.y + node.height,
        );
    }
    for child in &children {
        include_bounds(
            &mut bounds,
            child.x,
            child.y,
            child.x + child.width,
            child.y + child.height,
        );
    }
    let bounds = bounds?;
    let crosses = crossing_sides(diagram, index, layout, &bounds);
    let left = side_inset(
        Side::Left,
        children.iter().any(|child| child.x == bounds.min_x),
        crosses.contains(&Side::Left),
        exit_rows(diagram, index, layout, &bounds, Side::Left),
    );
    let right = side_inset(
        Side::Right,
        children
            .iter()
            .any(|child| child.x + child.width == bounds.max_x),
        crosses.contains(&Side::Right),
        exit_rows(diagram, index, layout, &bounds, Side::Right),
    );
    let top = side_inset(
        Side::Top,
        children.iter().any(|child| child.y == bounds.min_y),
        crosses.contains(&Side::Top),
        exit_rows(diagram, index, layout, &bounds, Side::Top),
    );
    let bottom = side_inset(
        Side::Bottom,
        children
            .iter()
            .any(|child| child.y + child.height == bounds.max_y),
        crosses.contains(&Side::Bottom),
        exit_rows(diagram, index, layout, &bounds, Side::Bottom),
    );
    let mut x = bounds.min_x as isize - left as isize;
    let y = bounds.min_y as isize - top as isize;
    let mut width = bounds.max_x - bounds.min_x + left + right;
    let height = bounds.max_y - bounds.min_y + top + bottom;
    let title = display_width(&diagram.subgraphs[index.get()].label) + SUBGRAPH_TITLE_DECOR;
    if title > width {
        let extra = title - width;
        x -= (extra / 2) as isize;
        width = title;
    }
    Some(FrameDraft {
        x,
        y,
        width,
        height,
    })
}

fn include_bounds(bounds: &mut Option<Bounds>, x0: usize, y0: usize, x1: usize, y1: usize) {
    match bounds {
        None => {
            *bounds = Some(Bounds {
                min_x: x0,
                min_y: y0,
                max_x: x1,
                max_y: y1,
            });
        }
        Some(bounds) => {
            bounds.min_x = bounds.min_x.min(x0);
            bounds.min_y = bounds.min_y.min(y0);
            bounds.max_x = bounds.max_x.max(x1);
            bounds.max_y = bounds.max_y.max(y1);
        }
    }
}

fn side_inset(side: Side, against_child: bool, crosses: bool, exit_rows: usize) -> usize {
    let pad = match side {
        Side::Left => SUBGRAPH_PAD_LEFT,
        Side::Right => SUBGRAPH_PAD_RIGHT,
        Side::Top => SUBGRAPH_PAD_TOP,
        Side::Bottom => SUBGRAPH_PAD_BOTTOM,
    };
    let inset = pad + exit_rows;
    let inset = if against_child || crosses {
        inset.max(MIN_FRAME_SEPARATION)
    } else {
        inset
    };
    if side == Side::Top && crosses && !against_child {
        inset.max(SUBGRAPH_PAD_TOP + 1)
    } else {
        inset
    }
}

fn crossing_sides(
    diagram: &GraphDiagram,
    index: SubgraphIndex,
    layout: &GraphLayout,
    bounds: &Bounds,
) -> HashSet<Side> {
    let mut sides = HashSet::new();
    let closed = &diagram.subgraphs[index.get()].node_ids;
    for edge in &diagram.edges {
        if edge.edge_type == EdgeType::Invisible || edge.from == edge.to {
            continue;
        }
        let from_in = closed.iter().any(|id| id == &edge.from);
        let to_in = closed.iter().any(|id| id == &edge.to);
        if from_in == to_in {
            continue;
        }
        let outside_id = if from_in { &edge.to } else { &edge.from };
        let Some(outside) = layout.nodes.iter().find(|node| node.id == *outside_id) else {
            continue;
        };
        if let Some(side) = outside_side(outside, bounds) {
            sides.insert(side);
        }
    }
    sides
}

fn exit_rows(
    diagram: &GraphDiagram,
    index: SubgraphIndex,
    layout: &GraphLayout,
    bounds: &Bounds,
    side: Side,
) -> usize {
    let closed = &diagram.subgraphs[index.get()].node_ids;
    diagram
        .edges
        .iter()
        .filter(|edge| {
            closed.iter().any(|id| id == &edge.from)
                && !closed.iter().any(|id| id == &edge.to)
                && layout
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.to)
                    .and_then(|node| outside_side(node, bounds))
                    == Some(side)
        })
        .filter_map(|edge| edge.label.as_deref().map(line_count))
        .max()
        .unwrap_or(0)
}

fn outside_side(node: &NodeLayout, bounds: &Bounds) -> Option<Side> {
    if node.y + node.height <= bounds.min_y {
        Some(Side::Top)
    } else if node.y >= bounds.max_y {
        Some(Side::Bottom)
    } else if node.x + node.width <= bounds.min_x {
        Some(Side::Left)
    } else if node.x >= bounds.max_x {
        Some(Side::Right)
    } else {
        None
    }
}

fn repair_negative(layout: &mut GraphLayout, frame: &FrameDraft) -> bool {
    let dx = if frame.x < 0 { -frame.x } else { 0 };
    let dy = if frame.y < 0 { -frame.y } else { 0 };
    if dx == 0 && dy == 0 {
        return false;
    }
    translate_nodes(&mut layout.nodes, dx, dy);
    for sg in &mut layout.subgraphs {
        sg.x = (sg.x as isize + dx) as usize;
        sg.y = (sg.y as isize + dy) as usize;
    }
    true
}

fn repair_clearance(
    diagram: &GraphDiagram,
    index: SubgraphIndex,
    layout: &mut GraphLayout,
    frame: &FrameDraft,
) -> bool {
    let nodes = layout.nodes.clone();
    let closed = &diagram.subgraphs[index.get()].node_ids;
    for node in &nodes {
        if closed.iter().any(|id| id == &node.id) {
            continue;
        }
        if overlaps_frame(frame, node) && separate_overlap(&diagram.direction, layout, frame, node)
        {
            return true;
        }
    }
    match diagram.direction {
        Direction::TopDown => {
            for edge in &diagram.edges {
                if edge.edge_type == EdgeType::Invisible {
                    continue;
                }
                let from_in = closed.iter().any(|id| id == &edge.from);
                let to_in = closed.iter().any(|id| id == &edge.to);
                if from_in || !to_in {
                    continue;
                }
                let Some(from) = nodes.iter().find(|node| node.id == edge.from) else {
                    continue;
                };
                let Some(to) = nodes.iter().find(|node| node.id == edge.to) else {
                    continue;
                };
                if from.y + from.height > to.y {
                    continue;
                }
                let rows = edge.label.as_deref().map(line_count).unwrap_or(0);
                let need = (from.y + from.height + rows) as isize;
                if frame.y < need {
                    let amount = (need - frame.y) as usize;
                    let at = if frame.y > from.y as isize {
                        frame.y as usize
                    } else {
                        from.y + from.height
                    };
                    open_gap(layout, Axis::Y, at, amount);
                    return true;
                }
            }
            for edge in &diagram.edges {
                if edge.edge_type == EdgeType::Invisible {
                    continue;
                }
                let from_in = closed.iter().any(|id| id == &edge.from);
                let to_in = closed.iter().any(|id| id == &edge.to);
                if !from_in || to_in {
                    continue;
                }
                let Some(to) = nodes.iter().find(|node| node.id == edge.to) else {
                    continue;
                };
                if (to.y as isize) < frame.y {
                    continue;
                }
                let need = frame.bottom() + 1;
                if (to.y as isize) < need {
                    open_gap(layout, Axis::Y, to.y, (need - to.y as isize) as usize);
                    return true;
                }
            }
        }
        Direction::LeftRight => {
            for edge in &diagram.edges {
                if edge.edge_type == EdgeType::Invisible {
                    continue;
                }
                let from_in = closed.iter().any(|id| id == &edge.from);
                let to_in = closed.iter().any(|id| id == &edge.to);
                if !from_in || to_in {
                    continue;
                }
                let Some(to) = nodes.iter().find(|node| node.id == edge.to) else {
                    continue;
                };
                if (to.x as isize) < frame.x {
                    continue;
                }
                let need = frame.right() + 1;
                if (to.x as isize) < need {
                    open_gap(layout, Axis::X, to.x, (need - to.x as isize) as usize);
                    return true;
                }
            }
        }
    }
    false
}

fn overlaps_frame(frame: &FrameDraft, node: &NodeLayout) -> bool {
    let x = node.x as isize;
    let y = node.y as isize;
    frame.x < x + node.width as isize
        && x < frame.right()
        && frame.y < y + node.height as isize
        && y < frame.bottom()
}

fn separate_overlap(
    direction: &Direction,
    layout: &mut GraphLayout,
    frame: &FrameDraft,
    node: &NodeLayout,
) -> bool {
    match direction {
        Direction::TopDown => {
            if (node.y as isize) < frame.y {
                let amount = (node.y + node.height) as isize - frame.y;
                if amount > 0 && frame.y >= 0 {
                    open_gap(layout, Axis::Y, frame.y as usize, amount as usize);
                    return true;
                }
            } else {
                let amount = frame.bottom() - node.y as isize;
                if amount > 0 {
                    open_gap(layout, Axis::Y, node.y, amount as usize);
                    return true;
                }
            }
        }
        Direction::LeftRight => {
            if (node.x as isize) < frame.x {
                let amount = (node.x + node.width) as isize - frame.x;
                if amount > 0 && frame.x >= 0 {
                    open_gap(layout, Axis::X, frame.x as usize, amount as usize);
                    return true;
                }
            } else {
                let amount = frame.right() - node.x as isize;
                if amount > 0 {
                    open_gap(layout, Axis::X, node.x, amount as usize);
                    return true;
                }
            }
        }
    }
    false
}

fn repair_title(
    diagram: &GraphDiagram,
    index: SubgraphIndex,
    layout: &mut GraphLayout,
    frame: &mut SubgraphLayout,
) {
    if diagram.direction != Direction::TopDown {
        return;
    }
    let reserved = frame.x + SUBGRAPH_TITLE_TEXT_OFFSET + display_width(&frame.label) + 2;
    let closed = &diagram.subgraphs[index.get()].node_ids;
    let mut leftmost = None;
    for edge in &diagram.edges {
        if edge.edge_type == EdgeType::Invisible {
            continue;
        }
        let from_in = closed.iter().any(|id| id == &edge.from);
        let to_in = closed.iter().any(|id| id == &edge.to);
        if from_in || !to_in {
            continue;
        }
        let Some(from) = layout.nodes.iter().find(|node| node.id == edge.from) else {
            continue;
        };
        let Some(to) = layout.nodes.iter().find(|node| node.id == edge.to) else {
            continue;
        };
        if from.y + from.height > frame.y {
            continue;
        }
        leftmost = Some(leftmost.map_or(to.center_x, |value: usize| value.min(to.center_x)));
    }
    let Some(leftmost) = leftmost else {
        return;
    };
    let shift = reserved.saturating_sub(leftmost);
    if shift == 0 {
        return;
    }
    let descendants = descendant_ids(diagram, index);
    for node in &mut layout.nodes {
        let member = closed.iter().any(|id| id == &node.id);
        if member || !flow_overlaps(&diagram.direction, node, frame) {
            node.x += shift;
            node.center_x += shift;
        }
    }
    for sg in &mut layout.subgraphs {
        if descendants.contains(&sg.id) {
            sg.x += shift;
        }
    }
    frame.width += shift;
}

fn flow_overlaps(direction: &Direction, node: &NodeLayout, frame: &SubgraphLayout) -> bool {
    match direction {
        Direction::TopDown => node.y < frame.y + frame.height && node.y + node.height > frame.y,
        Direction::LeftRight => node.x < frame.x + frame.width && node.x + node.width > frame.x,
    }
}

fn descendant_ids(diagram: &GraphDiagram, index: SubgraphIndex) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut stack = diagram.children(index);
    while let Some(child) = stack.pop() {
        ids.insert(diagram.subgraphs[child.get()].id.clone());
        stack.extend(diagram.children(child));
    }
    ids
}

fn open_gap(layout: &mut GraphLayout, axis: Axis, at: usize, amount: usize) {
    if amount == 0 {
        return;
    }
    for node in &mut layout.nodes {
        match axis {
            Axis::X if node.x >= at => {
                node.x += amount;
                node.center_x += amount;
            }
            Axis::Y if node.y >= at => {
                node.y += amount;
                node.center_y += amount;
            }
            _ => {}
        }
    }
    for sg in &mut layout.subgraphs {
        match axis {
            Axis::X if sg.x >= at => sg.x += amount,
            Axis::X if sg.x + sg.width > at => sg.width += amount,
            Axis::Y if sg.y >= at => sg.y += amount,
            Axis::Y if sg.y + sg.height > at => sg.height += amount,
            _ => {}
        }
    }
}

fn clear_label_columns(diagram: &GraphDiagram, layout: &mut GraphLayout) {
    if diagram.direction != Direction::LeftRight {
        return;
    }
    for _ in 0..48 {
        let Some((at, amount)) = label_column_deficit(layout) else {
            return;
        };
        open_gap(layout, Axis::X, at, amount);
    }
}

fn label_column_deficit(layout: &GraphLayout) -> Option<(usize, usize)> {
    for edge in &layout.edges {
        if edge.route != EdgeRoute::Forward {
            continue;
        }
        let Some(label) = edge.label.as_deref() else {
            continue;
        };
        let Some(from) = layout.nodes.iter().find(|node| node.id == edge.from_id) else {
            continue;
        };
        let Some(to) = layout.nodes.iter().find(|node| node.id == edge.to_id) else {
            continue;
        };
        if from.center_y != to.center_y {
            continue;
        }
        let uses_source = layout
            .edges
            .iter()
            .filter(|other| {
                other.route == EdgeRoute::Forward
                    && other.label.is_some()
                    && other.from_id == edge.from_id
            })
            .count()
            <= 1;
        let (start, end) = if uses_source {
            (rank_gutter(layout, from), next_rank_x(layout, from, to))
        } else {
            (label_anchor(layout, to), to.x)
        };
        let width = multiline_width(label);
        let available = end.saturating_sub(start);
        let col = if width <= available {
            start + (available - width) / 2
        } else {
            start
        };
        let label_end = col + width;
        let row = from.center_y.saturating_sub(line_count(label));
        let mut borders = Vec::new();
        for sg in &layout.subgraphs {
            if row < sg.y || row >= sg.y + sg.height {
                continue;
            }
            for border in [sg.x, sg.x + sg.width - 1] {
                if border > from.x + from.width && border < to.x {
                    borders.push(border);
                }
            }
        }
        let Some(outermost) = borders.into_iter().min() else {
            continue;
        };
        if label_end > outermost {
            return Some((outermost, label_end - outermost));
        }
    }
    None
}

fn rank_gutter(layout: &GraphLayout, node: &NodeLayout) -> usize {
    layout
        .nodes
        .iter()
        .filter(|other| other.x == node.x)
        .map(|other| other.x + other.width)
        .max()
        .unwrap_or(node.x + node.width)
}

fn next_rank_x(layout: &GraphLayout, from: &NodeLayout, to: &NodeLayout) -> usize {
    layout
        .nodes
        .iter()
        .filter(|node| node.x > from.x)
        .map(|node| node.x)
        .min()
        .unwrap_or(to.x)
}

fn label_anchor(layout: &GraphLayout, to: &NodeLayout) -> usize {
    layout
        .nodes
        .iter()
        .filter(|node| node.x < to.x)
        .map(|node| node.x + node.width + 1)
        .max()
        .unwrap_or(0)
}

fn finish_routes(layout: &mut GraphLayout, diagram: &GraphDiagram) {
    let endpoints = edge_endpoint_nodes(&layout.nodes, &layout.subgraphs);
    assign_edge_routes(&diagram.direction, &endpoints, &mut layout.edges);
    let (mut width, mut height) = base_extents(&layout.nodes, &layout.subgraphs);
    if diagram.direction == Direction::TopDown {
        width = width.max(td_labels_right(&layout.nodes, &diagram.edges));
    }
    for edge in &diagram.edges {
        if edge.from == edge.to
            && let Some(node) = layout.nodes.iter().find(|node| node.id == edge.from)
        {
            let label_w = edge
                .label
                .as_ref()
                .map(|label| display_width(label))
                .unwrap_or(0);
            width = width.max(node.x + node.width + 2 + label_w);
            height = height.max(node.y + node.height + 1);
        }
    }
    reserve_back_edge_space(&diagram.direction, &layout.edges, &mut width, &mut height);
    layout.width = width;
    layout.height = height;
    reserve_subgraph_entry_space(layout);
    let (width, height) = base_extents(&layout.nodes, &layout.subgraphs);
    layout.width = layout.width.max(width);
    layout.height = layout.height.max(height);
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
    fn rank_cycle_keeps_forward_edges_forward() {
        let diagram = parse_graph(
            "graph LR\n    A --> B\n    A --> C\n    B --> D\n    C --> D\n    D --> A\n",
        )
        .unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 1);
        assert_eq!(ranks["D"], 2);
    }

    #[test]
    fn rank_cycle_entry_node_first() {
        let diagram = parse_graph("graph TD\n    A --> B\n    B --> C\n    C --> A\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
    }

    #[test]
    fn rank_every_kept_edge_points_forward() {
        // Two interlocking cycles, a parallel edge and a node entered only
        // through a cycle: whatever the DFS drops, every edge it keeps has to
        // point at a strictly higher rank, or the renderer draws backwards.
        let diagram = parse_graph(
            "graph TD\n    A --> B\n    A --> B\n    B --> C\n    C --> A\n    C --> D\n    D --> B\n    D --> E\n    E --> D\n",
        )
        .unwrap();
        let back_edges = find_back_edges(&diagram);
        let ranks = assign_ranks(&diagram);

        for (index, edge) in diagram.edges.iter().enumerate() {
            if back_edges.contains(&index) || edge.from == edge.to {
                continue;
            }
            assert!(
                ranks[&edge.from] < ranks[&edge.to],
                "{} --> {} kept but ranked {} --> {}",
                edge.from,
                edge.to,
                ranks[&edge.from],
                ranks[&edge.to]
            );
        }
        assert!(ranks.values().any(|r| *r == 0), "rank 0 is never empty");
    }

    #[test]
    fn rank_node_reached_only_through_a_cycle_is_ranked() {
        let diagram =
            parse_graph("graph TD\n    A --> B\n    B --> A\n    B --> C\n    C --> B\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
    }

    #[test]
    fn back_edge_lane_per_edge() {
        let diagram =
            parse_graph("graph TD\n    A --> B\n    B --> C\n    C --> A\n    C --> B\n").unwrap();
        let layout = compute(&diagram).unwrap();
        let routes: Vec<EdgeRoute> = layout.edges.iter().map(|e| e.route).collect();
        assert_eq!(
            routes,
            vec![
                EdgeRoute::Forward,
                EdgeRoute::Forward,
                EdgeRoute::Back { lane: 0 },
                EdgeRoute::Back { lane: 1 },
            ],
            "both back edges get their own lane"
        );
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
    fn layout_lr_box_and_diamond_share_a_text_row() {
        let diagram = parse_graph("graph LR\n    A[Start] --> B{Choice}\n").unwrap();
        let layout = compute(&diagram).unwrap();
        let a = layout.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.id == "B").unwrap();
        assert_eq!(
            a.center_y, b.center_y,
            "the edge between a box and a diamond stays straight"
        );
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
        let sg_a = layout
            .subgraphs
            .iter()
            .find(|s| s.label == "GroupA")
            .unwrap();
        let sg_b = layout
            .subgraphs
            .iter()
            .find(|s| s.label == "GroupB")
            .unwrap();

        // Subgraph x-ranges must not overlap
        let a_right = sg_a.x + sg_a.width;
        let b_right = sg_b.x + sg_b.width;
        assert!(
            a_right <= sg_b.x || b_right <= sg_a.x,
            "subgraphs overlap: GroupA({}-{}), GroupB({}-{})",
            sg_a.x,
            a_right,
            sg_b.x,
            b_right
        );

        // Each node must be within its subgraph bounds
        for node_id in &["A", "B", "C"] {
            let n = layout.nodes.iter().find(|n| n.id == *node_id).unwrap();
            assert!(n.x >= sg_a.x, "{node_id} x < sg_a.x");
            assert!(
                n.x + n.width <= sg_a.x + sg_a.width,
                "{node_id} right > sg_a right"
            );
            assert!(n.y >= sg_a.y, "{node_id} y < sg_a.y");
            assert!(
                n.y + n.height <= sg_a.y + sg_a.height,
                "{node_id} bottom > sg_a bottom"
            );
        }
        for node_id in &["D", "E"] {
            let n = layout.nodes.iter().find(|n| n.id == *node_id).unwrap();
            assert!(n.x >= sg_b.x, "{node_id} x < sg_b.x");
            assert!(
                n.x + n.width <= sg_b.x + sg_b.width,
                "{node_id} right > sg_b right"
            );
            assert!(n.y >= sg_b.y, "{node_id} y < sg_b.y");
            assert!(
                n.y + n.height <= sg_b.y + sg_b.height,
                "{node_id} bottom > sg_b bottom"
            );
        }
    }

    #[test]
    fn cross_subgraph_dependency_places_target_after_unrelated_nodes() {
        let diagram = parse_graph(
            "graph LR\n\
             subgraph First\n\
               source --> shared\n\
             end\n\
             subgraph Second\n\
               unrelated\n\
               shared --> target\n\
             end\n",
        )
        .unwrap();
        let layout = compute(&diagram).unwrap();
        let unrelated = layout
            .nodes
            .iter()
            .find(|node| node.id == "unrelated")
            .unwrap();
        let target = layout
            .nodes
            .iter()
            .find(|node| node.id == "target")
            .unwrap();

        assert!(
            target.x > unrelated.x + unrelated.width,
            "cross-subgraph target must retain its later dependency rank"
        );
    }

    #[test]
    fn cross_subgraph_dependency_does_not_preserve_absent_rank_gaps() {
        let diagram = parse_graph(
            "graph LR\n\
             subgraph First\n\
               a --> b --> c --> d --> shared\n\
             end\n\
             subgraph Second\n\
               unrelated\n\
               shared --> target\n\
             end\n",
        )
        .unwrap();
        let layout = compute(&diagram).unwrap();
        let unrelated = layout
            .nodes
            .iter()
            .find(|node| node.id == "unrelated")
            .unwrap();
        let target = layout
            .nodes
            .iter()
            .find(|node| node.id == "target")
            .unwrap();

        assert!(target.x > unrelated.x + unrelated.width);
        assert!(
            target.x - (unrelated.x + unrelated.width) <= LR_GAP,
            "absent global ranks must not create empty columns"
        );
    }

    #[test]
    fn max_width_stacks_subgraphs_vertically() {
        let diagram = parse_graph(
            "graph LR\n    subgraph One\n        A --> B\n    end\n    subgraph Two\n        C --> D\n    end\n    B --> C\n",
        )
        .unwrap();
        let natural = compute(&diagram).unwrap();
        let first_width = natural.subgraphs[0].width;
        let second_width = natural.subgraphs[1].width;
        let max_width = first_width.max(second_width);

        assert!(natural.width > max_width, "fixture must require reflow");
        assert_eq!(
            compute_with_max_width(&diagram, natural.width).unwrap(),
            natural,
            "wide layouts keep the horizontal arrangement"
        );

        let layout = compute_with_max_width(&diagram, max_width).unwrap();
        let one = &layout.subgraphs[0];
        let two = &layout.subgraphs[1];

        assert!(layout.width <= max_width);
        assert_eq!(layout.direction, Direction::TopDown);
        assert!(two.y >= one.y + one.height + TD_RANK_SPACING);
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

        // C is above the subgraph (vertical layout)
        assert_eq!(layout.direction, Direction::TopDown, "layout should be TD");
        assert!(
            c.y + c.height <= sg.y,
            "bare node C should be above Backend subgraph: C bottom {}, sg top {}",
            c.y + c.height,
            sg.y
        );
    }

    #[test]
    fn rank_self_loop() {
        let diagram =
            parse_graph("graph TD\n    A --> B\n    B -->|fallback| B\n    B --> C\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
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
        assert!(
            sg.x + sg.width >= b.x + b.width,
            "subgraph right >= node B right"
        );
        assert!(
            sg.y + sg.height >= b.y + b.height,
            "subgraph bottom >= node B bottom"
        );
    }

    #[test]
    fn rank_cycle_two_nodes() {
        let diagram = parse_graph("flowchart LR\n    A --> B\n    B --> A\n").unwrap();
        let ranks = assign_ranks(&diagram);
        // Both nodes should get a rank (no stack overflow)
        assert!(ranks.contains_key("A"));
        assert!(ranks.contains_key("B"));
    }

    #[test]
    fn rank_cycle_three_nodes() {
        let diagram = parse_graph("flowchart TD\n    A --> B\n    B --> C\n    C --> A\n").unwrap();
        let ranks = assign_ranks(&diagram);
        assert!(ranks.contains_key("A"));
        assert!(ranks.contains_key("B"));
        assert!(ranks.contains_key("C"));
    }

    #[test]
    fn layout_cycle_does_not_panic() {
        let diagram = parse_graph("flowchart LR\n    A --> B\n    B --> A\n").unwrap();
        let layout = compute(&diagram);
        assert!(layout.is_ok());
    }
}
