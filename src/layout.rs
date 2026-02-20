use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub participants: Vec<ParticipantLayout>,
    pub rows: Vec<Row>,
    pub total_width: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantLayout {
    pub name: String,
    pub center_col: usize,
    pub box_left: usize,
    pub box_right: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Message(MessageRow),
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

pub fn compute(diagram: &Diagram) -> Result<Layout, String> {
    let (participant_order, display_names) = collect_participants(diagram);

    if participant_order.is_empty() {
        return Err("no participants found".to_string());
    }

    let gaps = compute_gaps(diagram, &participant_order, &display_names);
    let participants = compute_positions(&participant_order, &display_names, &gaps);
    let rows = compute_rows(diagram, &participant_order, &participants);

    let total_width = participants
        .last()
        .map(|p| p.box_right + 1)
        .unwrap_or(0);

    Ok(Layout {
        participants,
        rows,
        total_width,
    })
}

fn collect_participants(
    diagram: &Diagram,
) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut order: Vec<String> = Vec::new();
    let mut display_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for stmt in &diagram.statements {
        match stmt {
            Statement::ParticipantDecl(p) => {
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
        }
    }

    (order, display_names)
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

    for stmt in &diagram.statements {
        if let Statement::Message(m) = stmt {
            let from_idx = order.iter().position(|id| *id == m.from);
            let to_idx = order.iter().position(|id| *id == m.to);

            if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                let (left, right) = if fi < ti { (fi, ti) } else { (ti, fi) };
                let span_count = right - left;
                if span_count > 0 {
                    let required = m.text.len() + ARROW_DECORATION_WIDTH + 2;
                    let per_gap = (required + span_count - 1) / span_count;
                    for gap in &mut gaps[left..right] {
                        *gap = (*gap).max(per_gap);
                    }
                }
            }
        }
    }

    for (i, gap_idx) in (0..order.len().saturating_sub(1)).enumerate() {
        let left_name = display_names.get(&order[i]).unwrap();
        let right_name = display_names.get(&order[i + 1]).unwrap();
        let left_half = left_name.len() / 2 + 2;
        let right_half = right_name.len() / 2 + 2;
        let min_for_boxes = left_half + right_half + 2;
        gaps[gap_idx] = gaps[gap_idx].max(min_for_boxes);
    }

    gaps
}

fn compute_positions(
    order: &[String],
    display_names: &std::collections::HashMap<String, String>,
    gaps: &[usize],
) -> Vec<ParticipantLayout> {
    let mut participants = Vec::new();

    let first_name = display_names.get(&order[0]).unwrap();
    let first_box_width = first_name.len() + 4;
    let first_center = first_box_width / 2;

    participants.push(ParticipantLayout {
        name: first_name.clone(),
        center_col: first_center,
        box_left: 0,
        box_right: first_box_width - 1,
    });

    for (i, gap) in gaps.iter().enumerate() {
        let prev_center = participants[i].center_col;
        let center = prev_center + gap;
        let name = display_names.get(&order[i + 1]).unwrap();
        let box_width = name.len() + 4;

        participants.push(ParticipantLayout {
            name: name.clone(),
            center_col: center,
            box_left: center - box_width / 2,
            box_right: center + (box_width - 1) / 2,
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

    for stmt in &diagram.statements {
        if let Statement::Message(m) = stmt {
            let from_idx = order.iter().position(|id| *id == m.from).unwrap();
            let to_idx = order.iter().position(|id| *id == m.to).unwrap();
            let from_col = participants[from_idx].center_col;
            let to_col = participants[to_idx].center_col;

            let direction = if from_idx <= to_idx {
                Direction::LeftToRight
            } else {
                Direction::RightToLeft
            };

            rows.push(Row::Message(MessageRow {
                from_col,
                to_col,
                text: m.text.clone(),
                arrow: m.arrow,
                direction,
            }));
        }
    }

    rows
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
        }
    }
}
