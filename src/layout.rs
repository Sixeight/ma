use crate::ast::*;
use crate::display_width::{display_width, line_count, multiline_width};

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub participants: Vec<ParticipantLayout>,
    pub rows: Vec<Row>,
    pub total_width: usize,
    pub activations: Vec<Vec<bool>>,
    pub destroyed: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantLayout {
    pub name: String,
    pub center_col: usize,
    pub box_left: usize,
    pub box_right: usize,
    pub box_height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Message(MessageRow),
    Note(NoteRow),
    BlockStart(BlockRow),
    BlockEnd(BlockRow),
    BlockDivider(BlockRow),
    Destroy(DestroyRow),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockRow {
    pub label: String,
    pub frame_left: usize,
    pub frame_right: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestroyRow {
    pub col: usize,
    pub participant_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteRow {
    pub box_left: usize,
    pub box_right: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageRow {
    pub from_col: usize,
    pub to_col: usize,
    pub text: String,
    pub arrow: Arrow,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

const MIN_GAP: usize = 10;
const ARROW_DECORATION_WIDTH: usize = 2;
const SELF_LOOP_ARM: usize = 4;

pub fn compute(diagram: &Diagram) -> Result<Layout, String> {
    let (participant_order, display_names) = collect_participants(diagram);

    if participant_order.is_empty() {
        return Err("no participants found".to_string());
    }

    let gaps = compute_gaps(diagram, &participant_order, &display_names);
    let participants = compute_positions(&participant_order, &display_names, &gaps);
    let rows = compute_rows(diagram, &participant_order, &participants);
    let activations = compute_activations(diagram, &participant_order, rows.len());
    let destroyed = compute_destroyed(&rows, participants.len());

    let mut total_width = participants
        .last()
        .map(|p| p.box_right + 1)
        .unwrap_or(0);

    for row in &rows {
        match row {
            Row::Message(m) if m.from_col == m.to_col => {
                let right = m.from_col + 2 + multiline_width(&m.text) + 1;
                total_width = total_width.max(right);
                let arm_right = m.from_col + SELF_LOOP_ARM + 1;
                total_width = total_width.max(arm_right);
            }
            Row::Note(n) => {
                total_width = total_width.max(n.box_right + 1);
            }
            Row::BlockStart(b) | Row::BlockEnd(b) | Row::BlockDivider(b) => {
                total_width = total_width.max(b.frame_right + 1);
            }
            _ => {}
        }
    }

    Ok(Layout {
        participants,
        rows,
        total_width,
        activations,
        destroyed,
    })
}

pub fn compute_with_max_width(diagram: &Diagram, max_width: usize) -> Result<Layout, String> {
    let (order, display_names) = collect_participants(diagram);

    if order.is_empty() {
        return Err("no participants found".to_string());
    }

    let mut names = display_names;

    loop {
        // Try layout with gap shrinking
        let gaps = compute_gaps(diagram, &order, &names);
        let min_gaps = compute_min_box_gaps(&order, &names);
        let full_width = {
            let p = compute_positions(&order, &names, &gaps);
            p.last().map(|pp| pp.box_right + 1).unwrap_or(0)
        };
        let shrunk = shrink_gaps_to_fit(&gaps, &min_gaps, full_width, max_width);
        let participants = compute_positions(&order, &names, &shrunk);
        let base_width = participants.last().map(|p| p.box_right + 1).unwrap_or(0);

        if base_width <= max_width {
            return finish_layout(diagram, &order, participants, max_width);
        }

        // Find the longest name and truncate it by 1 char
        let (longest_id, longest_width) = order
            .iter()
            .map(|id| (id.clone(), multiline_width(names.get(id).unwrap())))
            .max_by_key(|(_, w)| *w)
            .unwrap();

        if longest_width <= 2 {
            return Err(format!(
                "diagram requires at least {base_width} columns, but max_width is {max_width}"
            ));
        }

        let name = names.get(&longest_id).unwrap().clone();
        names.insert(longest_id, truncate_to_display_width(&name, longest_width - 1));
    }
}

fn finish_layout(
    diagram: &Diagram,
    participant_order: &[String],
    participants: Vec<ParticipantLayout>,
    max_width: usize,
) -> Result<Layout, String> {
    let rows = compute_rows(diagram, participant_order, &participants);
    let activations = compute_activations(diagram, participant_order, rows.len());
    let destroyed = compute_destroyed(&rows, participants.len());

    let mut total_width = participants
        .last()
        .map(|p| p.box_right + 1)
        .unwrap_or(0);

    for row in &rows {
        match row {
            Row::Message(m) if m.from_col == m.to_col => {
                let right = m.from_col + 2 + multiline_width(&m.text) + 1;
                total_width = total_width.max(right);
                let arm_right = m.from_col + SELF_LOOP_ARM + 1;
                total_width = total_width.max(arm_right);
            }
            Row::Note(n) => {
                total_width = total_width.max(n.box_right + 1);
            }
            Row::BlockStart(b) | Row::BlockEnd(b) | Row::BlockDivider(b) => {
                total_width = total_width.max(b.frame_right + 1);
            }
            _ => {}
        }
    }

    // Cap at max_width — notes/blocks beyond will be clipped by the renderer
    total_width = total_width.min(max_width);

    Ok(Layout {
        participants,
        rows,
        total_width,
        activations,
        destroyed,
    })
}

fn compute_min_box_gaps(
    order: &[String],
    display_names: &std::collections::HashMap<String, String>,
) -> Vec<usize> {
    (0..order.len().saturating_sub(1))
        .map(|i| {
            let left = display_names.get(&order[i]).unwrap();
            let right = display_names.get(&order[i + 1]).unwrap();
            let left_half = multiline_width(left) / 2 + 2;
            let right_half = multiline_width(right) / 2 + 2;
            left_half + right_half + 2
        })
        .collect()
}

fn shrink_gaps_to_fit(
    gaps: &[usize],
    min_gaps: &[usize],
    current_width: usize,
    max_width: usize,
) -> Vec<usize> {
    let mut result = gaps.to_vec();
    if current_width <= max_width {
        return result;
    }

    let excess = current_width - max_width;
    let reducible: usize = result
        .iter()
        .zip(min_gaps.iter())
        .map(|(g, m)| g.saturating_sub(*m))
        .sum();

    if reducible == 0 {
        return result;
    }

    let reduction = excess.min(reducible);
    let mut remaining = reduction;
    for (gap, min) in result.iter_mut().zip(min_gaps.iter()) {
        let can_reduce = gap.saturating_sub(*min);
        if can_reduce == 0 {
            continue;
        }
        let share = (can_reduce * reduction).div_ceil(reducible);
        let actual = share.min(remaining).min(can_reduce);
        *gap -= actual;
        remaining -= actual;
    }

    result
}

fn truncate_to_display_width(name: &str, target_width: usize) -> String {
    if target_width <= 1 {
        return "…".to_string();
    }
    let mut result = String::new();
    let mut w = 0;
    for ch in name.chars() {
        if ch == '…' {
            continue;
        }
        let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + ch_w >= target_width {
            break;
        }
        result.push(ch);
        w += ch_w;
    }
    result.push('…');
    result
}

fn collect_participants(
    diagram: &Diagram,
) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut order: Vec<String> = Vec::new();
    let mut display_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for stmt in &diagram.statements {
        match stmt {
            Statement::ParticipantDecl(p) | Statement::Create(p) => {
                if !order.contains(&p.id) {
                    order.push(p.id.clone());
                    let name = p.alias.clone().unwrap_or_else(|| p.id.clone());
                    display_names.insert(p.id.clone(), name);
                }
            }
            Statement::Message(m) => {
                for id in [&m.from, &m.to] {
                    if !order.contains(id) {
                        order.push(id.clone());
                        display_names.insert(id.clone(), id.clone());
                    }
                }
            }
            Statement::Note(_) | Statement::Activate(_) | Statement::Deactivate(_) | Statement::Destroy(_) | Statement::AutoNumber => {}
            Statement::Loop(lb) | Statement::Opt(lb) | Statement::Break(lb) | Statement::Rect(lb) => {
                collect_participants_inner(&lb.body, &mut order, &mut display_names);
            }
            Statement::Alt(ab) | Statement::Par(ab) | Statement::Critical(ab) => {
                collect_participants_inner(&ab.body, &mut order, &mut display_names);
                for branch in &ab.else_branches {
                    collect_participants_inner(&branch.body, &mut order, &mut display_names);
                }
            }
        }
    }

    (order, display_names)
}

fn collect_participants_inner(
    statements: &[Statement],
    order: &mut Vec<String>,
    display_names: &mut std::collections::HashMap<String, String>,
) {
    for stmt in statements {
        if let Statement::Message(m) = stmt {
            for id in [&m.from, &m.to] {
                if !order.contains(id) {
                    order.push(id.clone());
                    display_names.insert(id.clone(), id.clone());
                }
            }
        }
    }
}

fn compute_gaps(
    diagram: &Diagram,
    order: &[String],
    display_names: &std::collections::HashMap<String, String>,
) -> Vec<usize> {
    if order.len() <= 1 {
        return vec![];
    }

    let mut gaps = vec![MIN_GAP; order.len() - 1];

    compute_gaps_inner(&diagram.statements, order, &mut gaps);

    for (i, gap_idx) in (0..order.len().saturating_sub(1)).enumerate() {
        let left_name = display_names.get(&order[i]).unwrap();
        let right_name = display_names.get(&order[i + 1]).unwrap();
        let left_half = multiline_width(left_name) / 2 + 2;
        let right_half = multiline_width(right_name) / 2 + 2;
        let min_for_boxes = left_half + right_half + 2;
        gaps[gap_idx] = gaps[gap_idx].max(min_for_boxes);
    }

    gaps
}

fn compute_gaps_inner(statements: &[Statement], order: &[String], gaps: &mut [usize]) {
    for stmt in statements {
        match stmt {
            Statement::Message(m) => {
                let from_idx = order.iter().position(|id| *id == m.from);
                let to_idx = order.iter().position(|id| *id == m.to);

                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    if fi == ti {
                        // Self-message: need space to the right for text + loop arm
                        let required =
                            (multiline_width(&m.text) + 3).max(SELF_LOOP_ARM + 2);
                        if fi < gaps.len() {
                            gaps[fi] = gaps[fi].max(required);
                        }
                    } else {
                        let (left, right) = if fi < ti { (fi, ti) } else { (ti, fi) };
                        let span_count = right - left;
                        let required = multiline_width(&m.text) + ARROW_DECORATION_WIDTH + 2;
                        let per_gap = required.div_ceil(span_count);
                        for gap in &mut gaps[left..right] {
                            *gap = (*gap).max(per_gap);
                        }
                    }
                }
            }
            Statement::Note(n) => {
                let note_box_width = multiline_width(&n.text) + 4;
                match &n.placement {
                    NotePlacement::RightOf(id) => {
                        if let Some(idx) = order.iter().position(|p| p == id)
                            && idx + 1 < order.len()
                        {
                            let required = note_box_width + 4;
                            gaps[idx] = gaps[idx].max(required);
                        }
                    }
                    NotePlacement::LeftOf(id) => {
                        if let Some(idx) = order.iter().position(|p| p == id)
                            && idx > 0
                        {
                            let required = note_box_width + 4;
                            gaps[idx - 1] = gaps[idx - 1].max(required);
                        }
                    }
                    NotePlacement::Over(id) => {
                        if let Some(idx) = order.iter().position(|p| p == id) {
                            let half = note_box_width / 2 + 1;
                            if idx > 0 {
                                gaps[idx - 1] = gaps[idx - 1].max(half);
                            }
                            if idx + 1 < order.len() {
                                gaps[idx] = gaps[idx].max(half);
                            }
                        }
                    }
                    NotePlacement::OverTwo(a, b) => {
                        let a_idx = order.iter().position(|p| p == a);
                        let b_idx = order.iter().position(|p| p == b);
                        if let (Some(ai), Some(bi)) = (a_idx, b_idx) {
                            let (left, right) = if ai < bi { (ai, bi) } else { (bi, ai) };
                            let span_count = right - left;
                            if span_count > 0 {
                                let required = note_box_width + 2;
                                let per_gap = required.div_ceil(span_count);
                                for gap in &mut gaps[left..right] {
                                    *gap = (*gap).max(per_gap);
                                }
                            }
                        }
                    }
                }
            }
            Statement::Loop(lb) | Statement::Opt(lb) | Statement::Break(lb) | Statement::Rect(lb) => {
                compute_gaps_inner(&lb.body, order, gaps);
            }
            Statement::Alt(ab) | Statement::Par(ab) | Statement::Critical(ab) => {
                compute_gaps_inner(&ab.body, order, gaps);
                for branch in &ab.else_branches {
                    compute_gaps_inner(&branch.body, order, gaps);
                }
            }
            _ => {}
        }
    }
}

fn compute_positions(
    order: &[String],
    display_names: &std::collections::HashMap<String, String>,
    gaps: &[usize],
) -> Vec<ParticipantLayout> {
    let mut participants = Vec::new();

    let first_name = display_names.get(&order[0]).unwrap();
    let first_box_width = multiline_width(first_name) + 4;
    let first_center = first_box_width / 2;

    participants.push(ParticipantLayout {
        name: first_name.clone(),
        center_col: first_center,
        box_left: 0,
        box_right: first_box_width - 1,
        box_height: 2 + line_count(first_name),
    });

    for (i, gap) in gaps.iter().enumerate() {
        let prev_center = participants[i].center_col;
        let center = prev_center + gap;
        let name = display_names.get(&order[i + 1]).unwrap();
        let box_width = multiline_width(name) + 4;

        participants.push(ParticipantLayout {
            name: name.clone(),
            center_col: center,
            box_left: center - box_width / 2,
            box_right: center + (box_width - 1) / 2,
            box_height: 2 + line_count(name),
        });
    }

    participants
}

fn compute_rows(
    diagram: &Diagram,
    order: &[String],
    participants: &[ParticipantLayout],
) -> Vec<Row> {
    let mut rows = Vec::new();
    let autonumber = diagram.statements.iter().any(|s| matches!(s, Statement::AutoNumber));
    let mut msg_counter = if autonumber { Some(1usize) } else { None };
    flatten_statements(&diagram.statements, order, participants, &mut rows, &mut msg_counter);
    rows
}

fn flatten_statements(
    statements: &[Statement],
    order: &[String],
    participants: &[ParticipantLayout],
    rows: &mut Vec<Row>,
    msg_counter: &mut Option<usize>,
) {
    for stmt in statements {
        match stmt {
            Statement::Message(m) => {
                let from_idx = order.iter().position(|id| *id == m.from).unwrap();
                let to_idx = order.iter().position(|id| *id == m.to).unwrap();
                let from_col = participants[from_idx].center_col;
                let to_col = participants[to_idx].center_col;

                let direction = if from_idx <= to_idx {
                    Direction::LeftToRight
                } else {
                    Direction::RightToLeft
                };

                let text = if let Some(n) = msg_counter.as_mut() {
                    let numbered = format!("{n}. {}", m.text);
                    *n += 1;
                    numbered
                } else {
                    m.text.clone()
                };

                rows.push(Row::Message(MessageRow {
                    from_col,
                    to_col,
                    text,
                    arrow: m.arrow,
                    direction,
                }));
            }
            Statement::Note(n) => {
                let note_box_width = multiline_width(&n.text) + 4;
                let (box_left, box_right) = match &n.placement {
                    NotePlacement::RightOf(id) => {
                        let idx = order.iter().position(|p| p == id).unwrap();
                        let left = participants[idx].center_col + 2;
                        (left, left + note_box_width - 1)
                    }
                    NotePlacement::LeftOf(id) => {
                        let idx = order.iter().position(|p| p == id).unwrap();
                        let right = participants[idx].center_col.saturating_sub(2);
                        (right.saturating_sub(note_box_width - 1), right)
                    }
                    NotePlacement::Over(id) => {
                        let idx = order.iter().position(|p| p == id).unwrap();
                        let center = participants[idx].center_col;
                        let half = note_box_width / 2;
                        let left = center.saturating_sub(half);
                        (left, left + note_box_width - 1)
                    }
                    NotePlacement::OverTwo(a, b) => {
                        let a_idx = order.iter().position(|p| p == a).unwrap();
                        let b_idx = order.iter().position(|p| p == b).unwrap();
                        let (left_idx, right_idx) = if a_idx < b_idx {
                            (a_idx, b_idx)
                        } else {
                            (b_idx, a_idx)
                        };
                        let left = participants[left_idx].center_col.saturating_sub(1);
                        let right = participants[right_idx].center_col + 1;
                        let min_right = left + note_box_width - 1;
                        (left, right.max(min_right))
                    }
                };
                rows.push(Row::Note(NoteRow {
                    box_left,
                    box_right,
                    text: n.text.clone(),
                }));
            }
            Statement::Loop(lb) => {
                push_simple_block("loop", lb, participants, order, rows, msg_counter);
            }
            Statement::Opt(lb) => {
                push_simple_block("opt", lb, participants, order, rows, msg_counter);
            }
            Statement::Break(lb) => {
                push_simple_block("break", lb, participants, order, rows, msg_counter);
            }
            Statement::Alt(ab) => {
                push_divided_block("alt", "else", ab, participants, order, rows, msg_counter);
            }
            Statement::Par(ab) => {
                push_divided_block("par", "and", ab, participants, order, rows, msg_counter);
            }
            Statement::Critical(ab) => {
                push_divided_block("critical", "option", ab, participants, order, rows, msg_counter);
            }
            Statement::Rect(lb) => {
                push_simple_block("rect", lb, participants, order, rows, msg_counter);
            }
            Statement::Destroy(id) => {
                if let Some(idx) = order.iter().position(|p| p == id) {
                    let col = participants[idx].center_col;
                    rows.push(Row::Destroy(DestroyRow {
                        col,
                        participant_idx: idx,
                    }));
                }
            }
            _ => {}
        }
    }
}

fn push_simple_block(
    keyword: &str,
    block: &LoopBlock,
    participants: &[ParticipantLayout],
    order: &[String],
    rows: &mut Vec<Row>,
    msg_counter: &mut Option<usize>,
) {
    let (frame_left, frame_right) = compute_frame_bounds(participants);
    let label = format!("{keyword} {}", block.label);
    let frame_right = frame_right.max(frame_left + 2 + display_width(&label) + 1);
    rows.push(Row::BlockStart(BlockRow {
        label,
        frame_left,
        frame_right,
    }));
    flatten_statements(&block.body, order, participants, rows, msg_counter);
    rows.push(Row::BlockEnd(BlockRow {
        label: String::new(),
        frame_left,
        frame_right,
    }));
}

fn push_divided_block(
    keyword: &str,
    divider: &str,
    block: &AltBlock,
    participants: &[ParticipantLayout],
    order: &[String],
    rows: &mut Vec<Row>,
    msg_counter: &mut Option<usize>,
) {
    let (frame_left, frame_right) = compute_frame_bounds(participants);
    let start_label = format!("{keyword} {}", block.label);
    let mut max_label_width = display_width(&start_label);
    for branch in &block.else_branches {
        let div_label = format!("{divider} {}", branch.label);
        max_label_width = max_label_width.max(display_width(&div_label));
    }
    let frame_right = frame_right.max(frame_left + 2 + max_label_width + 1);
    rows.push(Row::BlockStart(BlockRow {
        label: start_label,
        frame_left,
        frame_right,
    }));
    flatten_statements(&block.body, order, participants, rows, msg_counter);
    for branch in &block.else_branches {
        rows.push(Row::BlockDivider(BlockRow {
            label: format!("{divider} {}", branch.label),
            frame_left,
            frame_right,
        }));
        flatten_statements(&branch.body, order, participants, rows, msg_counter);
    }
    rows.push(Row::BlockEnd(BlockRow {
        label: String::new(),
        frame_left,
        frame_right,
    }));
}

fn compute_frame_bounds(participants: &[ParticipantLayout]) -> (usize, usize) {
    let frame_left = participants.first().map(|p| p.center_col.saturating_sub(2)).unwrap_or(0);
    let frame_right = participants.last().map(|p| p.center_col + 2).unwrap_or(0);
    (frame_left, frame_right)
}

fn compute_activations(
    diagram: &Diagram,
    order: &[String],
    row_count: usize,
) -> Vec<Vec<bool>> {
    let participant_count = order.len();
    let mut depths: Vec<i32> = vec![0; participant_count];
    let mut activations = Vec::with_capacity(row_count);

    compute_activations_inner(&diagram.statements, order, &mut depths, &mut activations);

    debug_assert_eq!(activations.len(), row_count);
    activations
}

fn compute_activations_inner(
    statements: &[Statement],
    order: &[String],
    depths: &mut Vec<i32>,
    activations: &mut Vec<Vec<bool>>,
) {
    for stmt in statements {
        match stmt {
            Statement::Activate(id) => {
                if let Some(idx) = order.iter().position(|p| p == id) {
                    depths[idx] += 1;
                }
            }
            Statement::Deactivate(id) => {
                if let Some(idx) = order.iter().position(|p| p == id) {
                    depths[idx] = (depths[idx] - 1).max(0);
                }
            }
            Statement::Message(m) => {
                if m.activate_target
                    && let Some(idx) = order.iter().position(|p| p == &m.to)
                {
                    depths[idx] += 1;
                }

                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);

                if m.deactivate_source
                    && let Some(idx) = order.iter().position(|p| p == &m.from)
                {
                    depths[idx] = (depths[idx] - 1).max(0);
                }
            }
            Statement::Note(_) => {
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);
            }
            Statement::Loop(lb) | Statement::Opt(lb) | Statement::Break(lb) | Statement::Rect(lb) => {
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active.clone());
                compute_activations_inner(&lb.body, order, depths, activations);
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);
            }
            Statement::Alt(ab) | Statement::Par(ab) | Statement::Critical(ab) => {
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);
                compute_activations_inner(&ab.body, order, depths, activations);
                for branch in &ab.else_branches {
                    let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                    activations.push(row_active);
                    compute_activations_inner(&branch.body, order, depths, activations);
                }
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);
            }
            Statement::Destroy(_) => {
                let row_active: Vec<bool> = depths.iter().map(|&d| d > 0).collect();
                activations.push(row_active);
            }
            Statement::ParticipantDecl(_) | Statement::Create(_) | Statement::AutoNumber => {}
        }
    }
}

fn compute_destroyed(rows: &[Row], participant_count: usize) -> Vec<bool> {
    let mut destroyed = vec![false; participant_count];
    for row in rows {
        if let Row::Destroy(d) = row {
            destroyed[d.participant_idx] = true;
        }
    }
    destroyed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_diagram;
    use pretty_assertions::assert_eq;

    #[test]
    fn layout_two_implicit_participants() {
        let diagram = parse_diagram("sequenceDiagram\n    Alice->>Bob: Hello\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.participants.len(), 2);
        assert_eq!(layout.participants[0].name, "Alice");
        assert_eq!(layout.participants[1].name, "Bob");
        assert!(layout.participants[0].center_col < layout.participants[1].center_col);
    }

    #[test]
    fn layout_explicit_participants_with_alias() {
        let input = "\
sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hello
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.participants.len(), 2);
        assert_eq!(layout.participants[0].name, "Alice");
        assert_eq!(layout.participants[1].name, "Bob");
    }

    #[test]
    fn layout_gap_accommodates_message_text() {
        let diagram =
            parse_diagram("sequenceDiagram\n    Alice->>Bob: A very long message text\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let gap = layout.participants[1].center_col - layout.participants[0].center_col;
        assert!(
            gap >= "A very long message text".len() + ARROW_DECORATION_WIDTH,
            "gap {gap} should accommodate message text"
        );
    }

    #[test]
    fn layout_message_direction_left_to_right() {
        let diagram = parse_diagram("sequenceDiagram\n    Alice->>Bob: Hi\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 1);
        match &layout.rows[0] {
            Row::Message(m) => {
                assert_eq!(m.direction, Direction::LeftToRight);
                assert_eq!(m.text, "Hi");
            }
            other => panic!("expected Message, got {other:?}"),
        }
    }

    #[test]
    fn layout_message_direction_right_to_left() {
        let diagram = parse_diagram("sequenceDiagram\n    Alice->>Bob: Hi\n    Bob-->>Alice: Hello\n").unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 2);
        match &layout.rows[1] {
            Row::Message(m) => {
                assert_eq!(m.direction, Direction::RightToLeft);
                assert_eq!(m.text, "Hello");
            }
            other => panic!("expected Message, got {other:?}"),
        }
    }

    #[test]
    fn layout_box_dimensions() {
        let diagram = parse_diagram("sequenceDiagram\n    Alice->>Bob: Hi\n").unwrap();
        let layout = compute(&diagram).unwrap();

        let alice = &layout.participants[0];
        let box_width = alice.box_right - alice.box_left + 1;
        assert_eq!(box_width, "Alice".len() + 4);
        assert_eq!(alice.center_col, alice.box_left + box_width / 2);
    }

    #[test]
    fn layout_three_participants() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hi
    Bob->>Charlie: Hey
    Charlie-->>Alice: Hello
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.participants.len(), 3);
        assert!(layout.participants[0].center_col < layout.participants[1].center_col);
        assert!(layout.participants[1].center_col < layout.participants[2].center_col);
        assert_eq!(layout.rows.len(), 3);

        match &layout.rows[2] {
            Row::Message(m) => {
                assert_eq!(m.direction, Direction::RightToLeft);
            }
            other => panic!("expected Message, got {other:?}"),
        }
    }

    // --- activations ---

    #[test]
    fn layout_activation_with_shorthand() {
        let input = "\
sequenceDiagram
    Alice->>+Bob: Hello
    Bob-->>-Alice: Hi!
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        // Row 0: Alice->>+Bob → Bob active after this message
        // Row 1: Bob-->>-Alice → Bob deactivated after this message
        assert!(!layout.activations[0][0], "Alice not active at row 0");
        assert!(layout.activations[0][1], "Bob active at row 0");
        assert!(!layout.activations[1][0], "Alice not active at row 1");
        assert!(layout.activations[1][1], "Bob still active at row 1 (deactivated after)");
    }

    #[test]
    fn layout_activation_explicit() {
        let input = "\
sequenceDiagram
    activate Alice
    Alice->>Bob: Working
    deactivate Alice
    Bob-->>Alice: Done
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        // Only Message rows are in layout.rows, Activate/Deactivate are not rows
        assert_eq!(layout.rows.len(), 2);
        assert!(layout.activations[0][0], "Alice active at row 0");
        assert!(!layout.activations[1][0], "Alice not active at row 1");
    }

    #[test]
    fn layout_no_activations() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.activations.len(), 1);
        assert!(!layout.activations[0][0]);
        assert!(!layout.activations[0][1]);
    }

    // --- notes ---

    // --- blocks ---

    #[test]
    fn layout_loop_generates_block_rows() {
        let input = "\
sequenceDiagram
    loop Check
        A->>B: Ping
    end
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 3, "BlockStart + Message + BlockEnd");
        match &layout.rows[0] {
            Row::BlockStart(b) => {
                assert_eq!(b.label, "loop Check");
            }
            other => panic!("expected BlockStart, got {other:?}"),
        }
        match &layout.rows[1] {
            Row::Message(m) => {
                assert_eq!(m.text, "Ping");
            }
            other => panic!("expected Message, got {other:?}"),
        }
        match &layout.rows[2] {
            Row::BlockEnd(b) => {
                assert!(b.frame_left < layout.participants[0].center_col);
                assert!(b.frame_right > layout.participants[1].center_col);
            }
            other => panic!("expected BlockEnd, got {other:?}"),
        }
    }

    #[test]
    fn layout_loop_with_surrounding_messages() {
        let input = "\
sequenceDiagram
    A->>B: Hello
    loop Check
        A->>B: Ping
    end
    B-->>A: Bye
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 5, "Hello + BlockStart + Ping + BlockEnd + Bye");
    }

    #[test]
    fn layout_note_right_of_generates_row() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note right of Bob: Got it!
    Bob-->>Alice: Hi!
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 3);
        match &layout.rows[1] {
            Row::Note(n) => {
                assert_eq!(n.text, "Got it!");
                assert!(
                    n.box_left > layout.participants[1].center_col,
                    "note box should be right of Bob's center"
                );
            }
            other => panic!("expected Note row, got {other:?}"),
        }
    }

    #[test]
    fn layout_note_left_of_generates_row() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note left of Bob: Left note
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 2);
        match &layout.rows[1] {
            Row::Note(n) => {
                assert_eq!(n.text, "Left note");
                assert!(
                    n.box_right < layout.participants[1].center_col,
                    "note box should be left of Bob's center"
                );
            }
            other => panic!("expected Note row, got {other:?}"),
        }
    }

    #[test]
    fn layout_note_over_single() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note over Alice: Thinking
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 2);
        match &layout.rows[1] {
            Row::Note(n) => {
                assert_eq!(n.text, "Thinking");
                let alice_center = layout.participants[0].center_col;
                assert!(n.box_left < alice_center);
                assert!(n.box_right > alice_center);
            }
            other => panic!("expected Note row, got {other:?}"),
        }
    }

    #[test]
    fn layout_note_over_two() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note over Alice,Bob: Shared note
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        assert_eq!(layout.rows.len(), 2);
        match &layout.rows[1] {
            Row::Note(n) => {
                assert_eq!(n.text, "Shared note");
                let alice_center = layout.participants[0].center_col;
                let bob_center = layout.participants[1].center_col;
                assert!(n.box_left <= alice_center);
                assert!(n.box_right >= bob_center);
            }
            other => panic!("expected Note row, got {other:?}"),
        }
    }

    // --- max_width ---

    #[test]
    fn layout_max_width_no_shrink_when_fits() {
        let diagram = parse_diagram("sequenceDiagram\n    Alice->>Bob: Hello\n").unwrap();
        let normal = compute(&diagram).unwrap();
        let constrained = compute_with_max_width(&diagram, normal.total_width + 10).unwrap();
        assert_eq!(constrained.total_width, normal.total_width);
    }

    #[test]
    fn layout_max_width_shrinks_gaps() {
        let input = "sequenceDiagram\n    Alice->>Bob: A somewhat long message here\n";
        let diagram = parse_diagram(input).unwrap();
        let normal = compute(&diagram).unwrap();
        let target = normal.total_width - 5;
        let constrained = compute_with_max_width(&diagram, target).unwrap();
        assert!(
            constrained.total_width <= target,
            "width {} should be <= {target}",
            constrained.total_width,
        );
    }

    #[test]
    fn layout_max_width_truncates_names() {
        let input = "sequenceDiagram\n    VeryLongParticipantName->>AnotherLongName: Hi\n";
        let diagram = parse_diagram(input).unwrap();
        let constrained = compute_with_max_width(&diagram, 25).unwrap();
        assert!(
            constrained.total_width <= 25,
            "width {} should be <= 25",
            constrained.total_width,
        );
        assert!(
            constrained.participants.iter().any(|p| p.name.contains('…')),
            "at least one name should be truncated",
        );
    }

    #[test]
    fn layout_max_width_impossible_returns_error() {
        let diagram = parse_diagram("sequenceDiagram\n    A->>B: Hi\n").unwrap();
        let result = compute_with_max_width(&diagram, 3);
        assert!(result.is_err());
    }

    #[test]
    fn layout_self_message_extends_total_width() {
        let input = "sequenceDiagram\n    A->>A: self message text\n";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();
        let a = &layout.participants[0];
        let required = a.center_col + 2 + display_width("self message text") + 1;
        assert!(
            layout.total_width >= required,
            "total_width {} should be >= {} for self-message",
            layout.total_width, required,
        );
    }

    #[test]
    fn layout_self_message_gap_with_neighbor() {
        let input = "sequenceDiagram\n    A->>B: Hi\n    A->>A: self message here\n";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();
        let a = &layout.participants[0];
        let b = &layout.participants[1];
        let text_end = a.center_col + 2 + display_width("self message here");
        assert!(
            b.center_col > text_end,
            "B center {} should be beyond self-message text end {}",
            b.center_col, text_end,
        );
    }

    #[test]
    fn layout_gap_accommodates_message_inside_loop() {
        let input = "\
sequenceDiagram
    A->>B: short
    loop Check
        A->>B: This is a much longer message inside a loop
    end
";
        let diagram = parse_diagram(input).unwrap();
        let layout = compute(&diagram).unwrap();

        let gap = layout.participants[1].center_col - layout.participants[0].center_col;
        let long_msg = "This is a much longer message inside a loop";
        assert!(
            gap >= display_width(long_msg) + ARROW_DECORATION_WIDTH,
            "gap {gap} should accommodate the long message inside loop (need {})",
            display_width(long_msg) + ARROW_DECORATION_WIDTH,
        );
    }
}
