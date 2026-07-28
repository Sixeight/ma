use crate::graph_ast::{Direction, Edge, EdgeType, GraphDiagram, NodeDecl, NodeShape};

pub fn parse_state(input: &str) -> Result<GraphDiagram, String> {
    let mut lines = input.lines().enumerate();
    let Some((_, header)) = lines.find(|(_, line)| !clean_line(line).is_empty()) else {
        return Err("expected stateDiagram-v2 header".to_string());
    };
    if !matches!(clean_line(header), "stateDiagram-v2" | "stateDiagram") {
        return Err("expected stateDiagram-v2 header".to_string());
    }

    let mut diagram = GraphDiagram {
        direction: Direction::TopDown,
        nodes: Vec::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
    };
    let mut marker_index = 0;

    for (line_index, raw_line) in lines {
        let line = clean_line(raw_line);
        if line.is_empty() {
            continue;
        }
        if let Some(direction) = line.strip_prefix("direction ") {
            diagram.direction = match direction.trim() {
                "LR" => Direction::LeftRight,
                "TD" | "TB" => Direction::TopDown,
                other => return line_error(line_index, format!("unsupported direction: {other}")),
            };
            continue;
        }
        if line.starts_with("state ") && line.ends_with('{') {
            return line_error(line_index, "composite states are not supported");
        }
        if let Some(rest) = line.strip_prefix("state ") {
            let (id, label) = parse_state_declaration(rest)
                .ok_or_else(|| format!("line {}: invalid state declaration", line_index + 1))?;
            upsert_node(&mut diagram.nodes, id, label, NodeShape::Box, true);
            continue;
        }
        if let Some((id, label)) = line.split_once(':')
            && line.find("-->").is_none_or(|arrow| id.len() < arrow)
        {
            let id = id.trim();
            let label = label.trim();
            validate_id(id)?;
            if label.is_empty() {
                return line_error(line_index, "state description is empty");
            }
            upsert_node(
                &mut diagram.nodes,
                id.to_string(),
                label.to_string(),
                NodeShape::Box,
                true,
            );
            continue;
        }
        if let Some((left, right)) = line.split_once("-->") {
            let (target, label) = match right.split_once(':') {
                Some((target, label)) => (target.trim(), Some(label.trim().to_string())),
                None => (right.trim(), None),
            };
            let source = left.trim();
            if source.is_empty() || target.is_empty() {
                return line_error(line_index, "transition endpoint is missing");
            }
            let from = add_endpoint(&mut diagram.nodes, source, true, &mut marker_index)?;
            let to = add_endpoint(&mut diagram.nodes, target, false, &mut marker_index)?;
            diagram.edges.push(Edge {
                from,
                to,
                edge_type: EdgeType::Arrow,
                label: label.filter(|label| !label.is_empty()),
            });
            continue;
        }
        validate_id(line).map_err(|error| format!("line {}: {error}", line_index + 1))?;
        upsert_node(
            &mut diagram.nodes,
            line.to_string(),
            line.to_string(),
            NodeShape::Box,
            false,
        );
    }

    Ok(diagram)
}

fn clean_line(line: &str) -> &str {
    line.split_once("%%")
        .map_or(line, |(before_comment, _)| before_comment)
        .trim()
}

fn parse_state_declaration(input: &str) -> Option<(String, String)> {
    if let Some(quoted) = input.strip_prefix('"') {
        let (label, id) = quoted.rsplit_once("\" as ")?;
        validate_id(id.trim()).ok()?;
        return Some((id.trim().to_string(), label.to_string()));
    }

    validate_id(input.trim()).ok()?;
    Some((input.trim().to_string(), input.trim().to_string()))
}

fn add_endpoint(
    nodes: &mut Vec<NodeDecl>,
    endpoint: &str,
    is_source: bool,
    marker_index: &mut usize,
) -> Result<String, String> {
    if endpoint == "[*]" {
        let id = format!("__state_marker_{}", *marker_index);
        *marker_index += 1;
        nodes.push(NodeDecl {
            id: id.clone(),
            label: if is_source { "●" } else { "◉" }.to_string(),
            shape: NodeShape::Circle,
        });
        return Ok(id);
    }

    validate_id(endpoint)?;
    upsert_node(
        nodes,
        endpoint.to_string(),
        endpoint.to_string(),
        NodeShape::Box,
        false,
    );
    Ok(endpoint.to_string())
}

fn upsert_node(
    nodes: &mut Vec<NodeDecl>,
    id: String,
    label: String,
    shape: NodeShape,
    explicit: bool,
) {
    if let Some(node) = nodes.iter_mut().find(|node| node.id == id) {
        if explicit {
            node.label = label;
            node.shape = shape;
        }
    } else {
        nodes.push(NodeDecl { id, label, shape });
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        Ok(())
    } else {
        Err(format!("invalid state identifier: {id}"))
    }
}

fn line_error<T>(line_index: usize, message: impl Into<String>) -> Result<T, String> {
    Err(format!("line {}: {}", line_index + 1, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_alias_updates_an_implicit_transition_node() {
        let diagram = parse_state(
            "stateDiagram-v2\n    Draft --> Review\n    state \"In review\" as Review\n",
        )
        .unwrap();

        let review = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Review")
            .unwrap();
        assert_eq!(review.label, "In review");
    }
}
