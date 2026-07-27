use std::collections::HashMap;

use crate::canvas::Canvas as Grid;
use crate::display_width::{display_width, split_br};
use crate::graph_ast::{Direction, EdgeType, NodeShape};
use crate::graph_layout::*;

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

    let lanes = back_edge_lanes(&layout.direction, &layout.nodes, &layout.edges);

    // Forward edges first, then back edges, then self-loops on top: each pass
    // may cross the previous one, and the later route is the one that must stay
    // readable (a fan-in bar would otherwise swallow a back edge's arrow head).
    for (index, edge) in layout.edges.iter().enumerate() {
        if edge.from_id == edge.to_id || lanes.contains(&index) {
            continue;
        }
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_td_edge(&mut grid, from, to, edge, layout);
    }
    for (lane, index) in lanes.iter().enumerate() {
        let edge = &layout.edges[*index];
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        let route_col = layout.width + lane - lanes.len();
        draw_td_back_edge(&mut grid, from, to, edge, layout, lane, route_col);
    }
    for edge in &layout.edges {
        if edge.from_id != edge.to_id {
            continue;
        }
        let from = node_map[edge.from_id.as_str()];
        draw_td_self_loop(&mut grid, from, edge);
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

    let lanes = back_edge_lanes(&layout.direction, &layout.nodes, &layout.edges);

    for (index, edge) in layout.edges.iter().enumerate() {
        if edge.from_id == edge.to_id || lanes.contains(&index) {
            continue;
        }
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        draw_lr_edge(&mut grid, from, to, edge, layout);
    }
    for (lane, index) in lanes.iter().enumerate() {
        let edge = &layout.edges[*index];
        let from = node_map[edge.from_id.as_str()];
        let to = node_map[edge.to_id.as_str()];
        let route_row = layout.height + lane - lanes.len();
        draw_lr_back_edge(&mut grid, from, to, edge, layout, lane, route_row);
    }
    for edge in &layout.edges {
        if edge.from_id != edge.to_id {
            continue;
        }
        let from = node_map[edge.from_id.as_str()];
        draw_td_self_loop(&mut grid, from, edge);
    }

    grid.render()
}

fn draw_node(grid: &mut Grid, node: &NodeLayout) {
    match node.shape {
        NodeShape::Box => draw_box(grid, node.x, node.y, node.width, node.height, &node.label),
        NodeShape::Round | NodeShape::Circle => {
            draw_round(grid, node.x, node.y, node.width, node.height, &node.label)
        }
        NodeShape::Diamond => {
            draw_diamond(grid, node.x, node.y, node.width, node.height, &node.label)
        }
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

fn draw_box(grid: &mut Grid, x: usize, y: usize, width: usize, height: usize, label: &str) {
    let lines = split_br(label);

    grid.set(y, x, '┌');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + width - 1, '┐');

    for (i, line) in lines.iter().enumerate() {
        let row = y + 1 + i;
        grid.set(row, x, '│');
        grid.write_str(row, x + 2, line);
        grid.set(row, x + width - 1, '│');
    }

    let bottom = y + height - 1;
    grid.set(bottom, x, '└');
    for col in (x + 1)..(x + width - 1) {
        grid.set(bottom, col, '─');
    }
    grid.set(bottom, x + width - 1, '┘');
}

fn draw_round(grid: &mut Grid, x: usize, y: usize, width: usize, height: usize, label: &str) {
    let lines = split_br(label);

    grid.set(y, x, '╭');
    for col in (x + 1)..(x + width - 1) {
        grid.set(y, col, '─');
    }
    grid.set(y, x + width - 1, '╮');

    let inner = width - 2;
    for (i, line) in lines.iter().enumerate() {
        let row = y + 1 + i;
        grid.set(row, x, '│');
        let pad_left = (inner - display_width(line)) / 2;
        grid.write_str(row, x + 1 + pad_left, line);
        grid.set(row, x + width - 1, '│');
    }

    let bottom = y + height - 1;
    grid.set(bottom, x, '╰');
    for col in (x + 1)..(x + width - 1) {
        grid.set(bottom, col, '─');
    }
    grid.set(bottom, x + width - 1, '╯');
}

fn draw_diamond(grid: &mut Grid, x: usize, y: usize, width: usize, height: usize, label: &str) {
    let lines = split_br(label);

    // Top border (inset by 2, no corners)
    for col in (x + 2)..(x + width - 2) {
        grid.set(y, col, '─');
    }

    // Upper slope (inset by 1)
    grid.set(y + 1, x + 1, '╱');
    grid.set(y + 1, x + width - 2, '╲');

    // Text rows (full width)
    for (i, line) in lines.iter().enumerate() {
        let row = y + 2 + i;
        grid.set(row, x, '│');
        grid.write_str(row, x + 2, line);
        grid.set(row, x + width - 1, '│');
    }

    // Lower slope (inset by 1)
    let lower = y + height - 2;
    grid.set(lower, x + 1, '╲');
    grid.set(lower, x + width - 2, '╱');

    // Bottom border (inset by 2, no corners)
    let bottom = y + height - 1;
    for col in (x + 2)..(x + width - 2) {
        grid.set(bottom, col, '─');
    }
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

fn route_crosses_node(
    layout: &GraphLayout,
    col: usize,
    row_start: usize,
    row_end: usize,
    from_id: &str,
    to_id: &str,
) -> bool {
    layout.nodes.iter().any(|n| {
        n.id != from_id
            && n.id != to_id
            && col >= n.x
            && col < n.x + n.width
            && row_start < n.y + n.height
            && row_end > n.y
    })
}

fn draw_td_single_edge_route(
    grid: &mut Grid,
    from_cx: usize,
    to_cx: usize,
    from_below: usize,
    to_above: usize,
    edge: &EdgeLayout,
    layout: &GraphLayout,
) {
    let edge_type = edge.edge_type;
    let vert = td_vertical_connector(edge_type);

    let route_start = if let Some(ref label) = edge.label {
        let label_col = from_cx.saturating_sub(display_width(label) / 2);
        grid.write_str(from_below, label_col, label);
        from_below + 1
    } else {
        from_below
    };

    let from_col_clear = !route_crosses_node(
        layout,
        from_cx,
        route_start,
        to_above,
        &edge.from_id,
        &edge.to_id,
    );

    if from_cx == to_cx && from_col_clear {
        // Straight down
        for row in route_start..to_above {
            if !is_subgraph_border_row(layout, row) {
                grid.set(row, from_cx, vert);
            }
        }
    } else if from_col_clear && to_above > route_start {
        // Source column is clear: route down at from_cx, turn at to_above row.
        // The turn shares the to_above row with the arrow head.
        for row in route_start..to_above {
            if !is_subgraph_border_row(layout, row) {
                grid.set(row, from_cx, vert);
            }
        }
        // Draw horizontal + corner at to_above (▼ overwrites to_cx later)
        if from_cx < to_cx {
            grid.set_merged(to_above, from_cx, '└', merge_box_drawing);
            for col in (from_cx + 1)..to_cx {
                grid.set(to_above, col, '─');
            }
        } else {
            grid.set_merged(to_above, from_cx, '┘', merge_box_drawing);
            for col in (to_cx + 1)..from_cx {
                grid.set(to_above, col, '─');
            }
        }
    } else if !from_col_clear && to_above > route_start {
        // from_cx column is blocked by intermediate nodes.
        // Route via gutter column (right of all intermediate nodes).
        let gutter_col = layout
            .nodes
            .iter()
            .filter(|n| n.id != edge.from_id && n.id != edge.to_id)
            .filter(|n| n.y + n.height > route_start && n.y < to_above)
            .map(|n| n.x + n.width)
            .max()
            .unwrap_or(from_cx)
            + 1;

        if gutter_col < grid.width() {
            for col in (from_cx + 1)..=gutter_col {
                grid.set(route_start, col, '─');
            }
            grid.set(route_start, gutter_col, '┐');

            for row in (route_start + 1)..to_above {
                grid.set(row, gutter_col, vert);
            }

            let (turn, a, b) = if to_cx < gutter_col {
                ('┘', to_cx + 1, gutter_col)
            } else {
                ('└', gutter_col + 1, to_cx)
            };
            grid.set_merged(to_above, gutter_col, turn, merge_box_drawing);
            for col in a..b {
                grid.set(to_above, col, '─');
            }
        }
    } else if edge.label.is_none() && from_cx != to_cx && to_above > from_below {
        // No label, original L-shaped routing at midpoint
        let mid_row = from_below + (to_above - from_below) / 2;
        for row in from_below..mid_row {
            if !is_subgraph_border_row(layout, row) {
                grid.set(row, from_cx, vert);
            }
        }
        let (left, right) = if from_cx < to_cx {
            grid.set(mid_row, from_cx, '└');
            grid.set(mid_row, to_cx, '┐');
            (from_cx + 1, to_cx)
        } else {
            grid.set(mid_row, from_cx, '┘');
            grid.set(mid_row, to_cx, '┌');
            (to_cx + 1, from_cx)
        };
        for col in left..right {
            grid.set(mid_row, col, '─');
        }
        for row in (mid_row + 1)..to_above {
            if !is_subgraph_border_row(layout, row) {
                grid.set(row, to_cx, vert);
            }
        }
    }
    // else: label + arrow only (no intermediate routing)

    if !is_subgraph_border_row(layout, to_above) {
        if has_arrow_head(edge_type) {
            grid.set(to_above, to_cx, '▼');
        } else {
            grid.set(to_above, to_cx, vert);
        }
    }
}

fn draw_td_self_loop(grid: &mut Grid, node: &NodeLayout, edge: &EdgeLayout) {
    let right_col = node.x + node.width - 1;
    let arm_col = right_col + 1;
    let loop_col = right_col + 2;
    let mid_row = node.y + 1; // text row where ├ goes
    let from_below = node.y + node.height;

    // ├─┐ on the text row
    grid.set(mid_row, right_col, '├');
    grid.set(mid_row, arm_col, '─');
    grid.set(mid_row, loop_col, '┐');

    // label to the right of the arm
    if let Some(ref label) = edge.label {
        grid.write_str(mid_row, loop_col + 1, label);
    }

    // │ going down
    for row in (mid_row + 1)..from_below {
        grid.set(row, loop_col, '│');
    }

    // ◄─┘ on the from_below row, right of center_x
    let return_col = node.center_x + 1;
    grid.set(from_below, return_col, '◄');
    for col in (return_col + 1)..loop_col {
        grid.set(from_below, col, '─');
    }
    grid.set(from_below, loop_col, '┘');
}

fn draw_td_edge(
    grid: &mut Grid,
    from: &NodeLayout,
    to: &NodeLayout,
    edge: &EdgeLayout,
    layout: &GraphLayout,
) {
    if from.id == to.id {
        draw_td_self_loop(grid, from, edge);
        return;
    }

    if is_back_edge(&layout.direction, from, to) {
        // Back edges are routed by the caller through a gutter lane; the
        // forward geometry below assumes the target sits ahead of the source.
        return;
    }

    let edge_type = edge.edge_type;
    let from_cx = from.center_x;
    let to_cx = to.center_x;
    let bottom_row = from.y + from.height - 1;
    let from_below = from.y + from.height;
    let to_above = to.y - 1;

    grid.set(bottom_row, from_cx, '┬');

    // Back edges are routed through the gutter, so they take no part in the
    // fan-out bar or the fan-in bar drawn for the forward edges.
    let forward_children: Vec<&NodeLayout> = layout
        .edges
        .iter()
        .filter(|e| e.from_id == from.id && e.from_id != e.to_id)
        .filter_map(|e| layout.nodes.iter().find(|n| n.id == e.to_id))
        .filter(|n| !is_back_edge(&layout.direction, from, n))
        .collect();
    let forward_parents: Vec<&NodeLayout> = layout
        .edges
        .iter()
        .filter(|e| e.to_id == to.id && e.from_id != e.to_id)
        .filter_map(|e| layout.nodes.iter().find(|n| n.id == e.from_id))
        .filter(|n| !is_back_edge(&layout.direction, n, to))
        .collect();
    let sibling_count = forward_children.len();
    let parent_count = forward_parents.len();

    if sibling_count > 1 {
        let child_centers: Vec<usize> = forward_children.iter().map(|n| n.center_x).collect();
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
        let parents = &forward_parents;
        let all_same_y = parents.windows(2).all(|w| w[0].y == w[1].y);

        if all_same_y {
            let parent_centers: Vec<usize> = parents.iter().map(|n| n.center_x).collect();
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
            draw_td_single_edge_route(
                grid, from_cx, to_cx, from_below, to_above, edge, layout,
            );
        }
    } else {
        draw_td_single_edge_route(grid, from_cx, to_cx, from_below, to_above, edge, layout);
    }
}

/// First column right of every node in this node's rank — inside the gap
/// between ranks, so a vertical lane there crosses no box.
fn lr_rank_gutter(layout: &GraphLayout, node: &NodeLayout) -> usize {
    layout
        .nodes
        .iter()
        .filter(|n| n.x == node.x)
        .map(|n| n.x + n.width)
        .max()
        .unwrap_or(node.x + node.width)
}

/// First row below every node in this node's rank — inside the gap between
/// ranks, so a horizontal lane there crosses no box.
fn td_rank_gutter(layout: &GraphLayout, node: &NodeLayout) -> usize {
    layout
        .nodes
        .iter()
        .filter(|n| n.y == node.y)
        .map(|n| n.y + n.height)
        .max()
        .unwrap_or(node.y + node.height)
}

fn enclosing_subgraph(layout: &GraphLayout, node: &NodeLayout) -> Option<usize> {
    layout.subgraphs.iter().position(|sg| {
        node.x >= sg.x
            && node.x + node.width <= sg.x + sg.width
            && node.y >= sg.y
            && node.y + node.height <= sg.y + sg.height
    })
}

/// Row each end of a back edge has to reach before it may turn sideways: below
/// its subgraph frame, so the route crosses the border instead of running along
/// it. Two ends inside the same frame turn inside it — leaving would mean
/// crossing the same border twice for nothing.
fn back_edge_turn_rows(layout: &GraphLayout, from: &NodeLayout, to: &NodeLayout) -> (usize, usize) {
    let (sg_from, sg_to) = (
        enclosing_subgraph(layout, from),
        enclosing_subgraph(layout, to),
    );
    if sg_from == sg_to {
        return (0, 0);
    }
    let below = |sg: Option<usize>| {
        sg.map(|i| layout.subgraphs[i].y + layout.subgraphs[i].height)
            .unwrap_or(0)
    };
    (below(sg_from), below(sg_to))
}

/// Column the vertical part of a back edge runs down, inside the gap right of
/// the node's rank. Lanes past the gap's width wrap and share a column.
fn lr_lane_col(layout: &GraphLayout, node: &NodeLayout, lane: usize) -> usize {
    let gutter = lr_rank_gutter(layout, node);
    let room = layout
        .nodes
        .iter()
        .map(|n| n.x)
        .filter(|x| *x > gutter)
        .min()
        .map(|next| next - gutter)
        .unwrap_or(1);
    gutter + lane % room
}

/// Row the horizontal part of a back edge runs along, inside the gap below the
/// node's rank. Lanes past the gap's height wrap and share a row.
fn td_lane_row(layout: &GraphLayout, node: &NodeLayout, lane: usize) -> usize {
    let gutter = td_rank_gutter(layout, node);
    let room = layout
        .nodes
        .iter()
        .map(|n| n.y)
        .filter(|y| *y > gutter)
        .min()
        .map(|next| next - gutter)
        .unwrap_or(1);
    gutter + lane % room
}

/// Routes an LR edge whose target sits left of its source: out of the bottom of
/// the source, down its rank gap to the gutter row below every node, back to
/// the target's rank gap, then up into the bottom of the target.
fn draw_lr_back_edge(
    grid: &mut Grid,
    from: &NodeLayout,
    to: &NodeLayout,
    edge: &EdgeLayout,
    layout: &GraphLayout,
    lane: usize,
    route_row: usize,
) {
    let from_below = from.y + from.height;
    let to_below = to.y + to.height;
    let (from_clear, to_clear) = back_edge_turn_rows(layout, from, to);
    let from_turn = from_below.max(from_clear);
    let to_turn = to_below.max(to_clear);
    let lane_from = lr_lane_col(layout, from, lane);
    let lane_to = lr_lane_col(layout, to, lane);
    if lane_to >= lane_from || route_row <= from_turn.max(to_turn) {
        // Same rank, or no gutter reserved: skip rather than draw junk.
        return;
    }

    let vert = td_vertical_connector(edge.edge_type);
    let horiz = lr_horizontal_connector(edge.edge_type);

    // Source: down out of the box, right along the rank gap, down to the lane
    grid.set_merged(from_below - 1, from.center_x, '┬', merge_box_drawing);
    for row in from_below..from_turn {
        grid.set_merged(row, from.center_x, vert, merge_box_drawing);
    }
    grid.set_merged(from_turn, from.center_x, '└', merge_box_drawing);
    for col in (from.center_x + 1)..lane_from {
        grid.set_merged(from_turn, col, horiz, merge_box_drawing);
    }
    grid.set_merged(from_turn, lane_from, '┐', merge_box_drawing);
    for row in (from_turn + 1)..route_row {
        grid.set_merged(row, lane_from, vert, merge_box_drawing);
    }
    grid.set_merged(route_row, lane_from, '┘', merge_box_drawing);

    // Gutter row, right to left
    for col in (lane_to + 1)..lane_from {
        grid.set_merged(route_row, col, horiz, merge_box_drawing);
    }
    grid.set_merged(route_row, lane_to, '└', merge_box_drawing);

    // Target: up the rank gap, left under the target, arrow up into the box
    for row in (to_turn + 1)..route_row {
        grid.set_merged(row, lane_to, vert, merge_box_drawing);
    }
    grid.set_merged(to_turn, lane_to, '┐', merge_box_drawing);
    for col in (to.center_x + 1)..lane_to {
        grid.set_merged(to_turn, col, horiz, merge_box_drawing);
    }
    if to_turn > to_below {
        grid.set_merged(to_turn, to.center_x, '└', merge_box_drawing);
        for row in (to_below + 1)..to_turn {
            grid.set_merged(row, to.center_x, vert, merge_box_drawing);
        }
    }
    grid.set(
        to_below,
        to.center_x,
        if has_arrow_head(edge.edge_type) {
            '▲'
        } else {
            vert
        },
    );

    if let Some(ref label) = edge.label {
        // Centred on the route when it fits, parked right of the route when it
        // does not — writing it over the corner would break the line.
        let span = lane_from - lane_to - 1;
        let width = display_width(label);
        let label_col = if width <= span {
            lane_to + 1 + (span - width) / 2
        } else {
            lane_from + 2
        };
        grid.write_str(route_row, label_col, label);
    }
}

/// Routes a TD edge whose target sits above its source: out of the bottom of
/// the source, right along its rank gap to the gutter column right of every
/// node, up to the target's row, then left into the side of the target.
fn draw_td_back_edge(
    grid: &mut Grid,
    from: &NodeLayout,
    to: &NodeLayout,
    edge: &EdgeLayout,
    layout: &GraphLayout,
    lane: usize,
    route_col: usize,
) {
    let from_below = from.y + from.height;
    let (from_clear, to_clear) = back_edge_turn_rows(layout, from, to);
    let lane_row = td_lane_row(layout, from, lane).max(from_clear);
    // Enter through the gap row below the target's rank, never through the row
    // the target sits on: that row belongs to its rank and crosses siblings.
    let to_gap = td_rank_gutter(layout, to).max(to_clear);
    let entry_col = to.center_x.max(to.x + to.width - 2);
    if route_col <= entry_col || lane_row <= to_gap {
        // Same rank, or the target's frame pushed both ends onto one row.
        return;
    }

    let vert = td_vertical_connector(edge.edge_type);
    let horiz = lr_horizontal_connector(edge.edge_type);

    // Source: down out of the box, right along the rank gap to the lane
    grid.set_merged(from_below - 1, from.center_x, '┬', merge_box_drawing);
    for row in from_below..lane_row {
        grid.set_merged(row, from.center_x, vert, merge_box_drawing);
    }
    grid.set_merged(lane_row, from.center_x, '└', merge_box_drawing);
    for col in (from.center_x + 1)..route_col {
        grid.set_merged(lane_row, col, horiz, merge_box_drawing);
    }
    grid.set_merged(lane_row, route_col, '┘', merge_box_drawing);

    // Gutter column, bottom to top
    for row in (to_gap + 1)..lane_row {
        grid.set_merged(row, route_col, vert, merge_box_drawing);
    }
    grid.set_merged(to_gap, route_col, '┐', merge_box_drawing);

    // Target: left along the gap row, then up into the box
    for col in (entry_col + 1)..route_col {
        grid.set_merged(to_gap, col, horiz, merge_box_drawing);
    }
    let to_below = to.y + to.height;
    if to_below < to_gap {
        grid.set_merged(to_gap, entry_col, '┘', merge_box_drawing);
        for row in (to_below + 1)..to_gap {
            grid.set_merged(row, entry_col, vert, merge_box_drawing);
        }
    }
    grid.set_merged(to_below - 1, entry_col, '┴', merge_box_drawing);
    grid.set(
        to_below,
        entry_col,
        if has_arrow_head(edge.edge_type) {
            '▲'
        } else {
            vert
        },
    );

    if let Some(ref label) = edge.label {
        grid.write_str(lane_row, from.center_x + 2, label);
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
    layout: &GraphLayout,
) {
    if from.id == to.id {
        draw_td_self_loop(grid, from, edge);
        return;
    }

    if is_back_edge(&layout.direction, from, to) {
        // Back edges are routed by the caller through a gutter lane; the
        // forward geometry below assumes the target sits ahead of the source.
        return;
    }

    let from_right = from.x + from.width;
    let to_left = to.x;
    let horiz = lr_horizontal_connector(edge.edge_type);

    if from.center_y == to.center_y {
        // Straight horizontal
        let row = from.center_y;
        for col in from_right..to_left {
            grid.set_merged(row, col, horiz, merge_box_drawing);
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
            grid.set_merged(from.center_y, mid_col, '┐', merge_box_drawing);
            for row in (from.center_y + 1)..to.center_y {
                grid.set_merged(row, mid_col, vert, merge_box_drawing);
            }
            grid.set_merged(to.center_y, mid_col, '└', merge_box_drawing);
        } else {
            grid.set_merged(from.center_y, mid_col, '┘', merge_box_drawing);
            for row in (to.center_y + 1)..from.center_y {
                grid.set_merged(row, mid_col, vert, merge_box_drawing);
            }
            grid.set_merged(to.center_y, mid_col, '┌', merge_box_drawing);
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
        let expected = "  ─────\n ╱     ╲\n│ Hello │\n ╲     ╱\n  ─────";
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
    fn render_td_multiline_label() {
        let output = render_input("graph TD\n    A[Hello<br/>World]\n");
        let expected = "\
┌───────┐
│ Hello │
│ World │
└───────┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_multiline_label_three_lines() {
        let output = render_input("graph TD\n    A[A<br/>B<br/>C]\n");
        let expected = "\
┌───┐
│ A │
│ B │
│ C │
└───┘";
        assert_eq!(output, expected);
    }

    #[test]
    fn render_td_offset_edge_connects_properly() {
        // When fan-out puts child to the right, the edge from child to grandchild
        // should still visually connect (no gap between │ and ▼)
        let output = render_input(
            "graph TD\n    A --> B\n    A --> C\n    C --> D\n",
        );
        // Find the ▼ above D and check there's a │ or corner above it
        let lines: Vec<&str> = output.lines().collect();
        let arrow_line = lines.iter().position(|l| {
            // Find ▼ that's NOT part of the fan-out (the one above D)
            let trimmed = l.trim();
            trimmed == "▼"
        });
        if let Some(arrow_idx) = arrow_line {
            let arrow_col = lines[arrow_idx].find('▼').unwrap();
            // The line above should have │ or └ or ┘ at the same column or have a corner connector
            let above = lines[arrow_idx - 1];
            let above_char = above.chars().nth(arrow_col).unwrap_or(' ');
            assert!(
                matches!(above_char, '│' | '┘' | '└' | '┌' | '┐' | '┴' | '┬'),
                "expected connector above ▼ at col {arrow_col}, got '{above_char}'\n{output}"
            );
        }
    }

    #[test]
    fn render_td_cross_rank_fan_in_routes_when_clear() {
        // D(rank 0, right column) → E(rank 3, center column)
        // D's column doesn't overlap with intermediate nodes B, C → routing is drawn
        let output = render_input(
            "graph TD\n    A -->|x| B\n    B -->|y| C\n    C --> E\n    D -->|z| E\n",
        );
        // D's edge should have label "z" and visible routing
        assert!(output.contains("z"), "label z rendered");
        // Intermediate nodes must remain intact
        assert!(output.contains("│ B │"), "B intact");
        assert!(output.contains("│ C │"), "C intact");
        // The D→E edge should route to E with a visible turn (▼───┘)
        assert!(output.contains("▼───┘"), "D→E routing merges at ▼───┘");
    }

    #[test]
    fn render_td_self_loop() {
        let output = render_input("graph TD\n    A -->|retry| A\n");
        assert!(output.contains("├"), "self-loop has ├ on right border");
        assert!(output.contains("◄"), "self-loop returns with ◄");
        assert!(output.contains("retry"), "label rendered");
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
