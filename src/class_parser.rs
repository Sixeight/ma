use crate::graph_ast::{Direction, Edge, EdgeType, GraphDiagram, NodeDecl, NodeShape};

#[derive(Debug)]
struct ClassDecl {
    id: String,
    label: String,
    attributes: Vec<String>,
    methods: Vec<String>,
    annotations: Vec<String>,
}

pub fn parse_class(input: &str) -> Result<GraphDiagram, String> {
    let mut lines = input.lines().enumerate();
    let Some((_, header)) = lines.find(|(_, line)| !clean_line(line).is_empty()) else {
        return Err("expected classDiagram header".to_string());
    };
    if clean_line(header) != "classDiagram" {
        return Err("expected classDiagram header".to_string());
    }

    let mut direction = Direction::TopDown;
    let mut classes = Vec::new();
    let mut edges = Vec::new();
    let mut current_class: Option<String> = None;

    for (line_index, raw_line) in lines {
        let line = clean_line(raw_line);
        if line.is_empty() {
            continue;
        }
        if let Some(class_id) = current_class.as_ref() {
            if line == "}" {
                current_class = None;
            } else if line.starts_with("<<") && line.ends_with(">>") {
                if line
                    .trim_start_matches("<<")
                    .trim_end_matches(">>")
                    .trim()
                    .is_empty()
                {
                    return line_error(line_index, "empty class annotation");
                }
                class_mut(&mut classes, class_id)
                    .annotations
                    .push(line.to_string());
            } else if line.contains('{') || line.contains('}') {
                return line_error(line_index, "nested class blocks are not supported");
            } else {
                add_member(class_mut(&mut classes, class_id), line);
            }
            continue;
        }

        if line.starts_with("namespace ") {
            return line_error(line_index, "namespace blocks are not supported");
        }
        if let Some(value) = line.strip_prefix("direction ") {
            direction = match value.trim() {
                "LR" => Direction::LeftRight,
                "TD" | "TB" => Direction::TopDown,
                other => return line_error(line_index, format!("unsupported direction: {other}")),
            };
            continue;
        }
        if line.starts_with("<<") {
            let Some((annotation, id)) = line.split_once(">>") else {
                return line_error(line_index, "invalid class annotation");
            };
            if annotation.trim_start_matches("<<").trim().is_empty() {
                return line_error(line_index, "empty class annotation");
            }
            let id = id.trim();
            validate_id(id).map_err(|error| format!("line {}: {error}", line_index + 1))?;
            upsert_class(&mut classes, id, None);
            class_mut(&mut classes, id)
                .annotations
                .push(format!("{annotation}>>"));
            continue;
        }
        if let Some(rest) = line.strip_prefix("class ") {
            let opens_block = rest.ends_with('{');
            let rest = rest.strip_suffix('{').unwrap_or(rest).trim();
            let (declaration, annotation) = match rest.split_once(" <<") {
                Some((declaration, annotation))
                    if annotation.ends_with(">>")
                        && !annotation.trim_end_matches(">>").trim().is_empty() =>
                {
                    (declaration.trim(), Some(format!("<<{annotation}")))
                }
                _ => (rest, None),
            };
            let (id, label) = parse_class_declaration(declaration)
                .ok_or_else(|| format!("line {}: invalid class declaration", line_index + 1))?;
            upsert_class(&mut classes, &id, Some(label));
            if let Some(annotation) = annotation {
                class_mut(&mut classes, &id).annotations.push(annotation);
            }
            if opens_block {
                current_class = Some(id);
            }
            continue;
        }
        if let Some(edge) = parse_relationship(line, &mut classes) {
            edges.push(edge?);
            continue;
        }
        if let Some((id, member)) = line.split_once(':') {
            let id = id.trim();
            validate_id(id).map_err(|error| format!("line {}: {error}", line_index + 1))?;
            if member.trim().is_empty() {
                return line_error(line_index, "class member is empty");
            }
            upsert_class(&mut classes, id, None);
            add_member(class_mut(&mut classes, id), member.trim());
            continue;
        }
        return line_error(
            line_index,
            format!("unsupported class diagram syntax: {line}"),
        );
    }

    if let Some(id) = current_class {
        return Err(format!("unclosed class block: {id}"));
    }

    let nodes = classes
        .into_iter()
        .map(|class| {
            let label = render_class_label(&class);
            NodeDecl {
                id: class.id,
                label,
                shape: NodeShape::Box,
            }
        })
        .collect();
    Ok(GraphDiagram {
        direction,
        nodes,
        edges,
        subgraphs: Vec::new(),
    })
}

fn clean_line(line: &str) -> &str {
    line.split_once("%%")
        .map_or(line, |(before_comment, _)| before_comment)
        .trim()
}

fn parse_class_declaration(input: &str) -> Option<(String, String)> {
    if let Some(open) = input.find("[\"") {
        let id = input[..open].trim();
        let label = input[open + 2..].strip_suffix("\"]")?;
        validate_id(id).ok()?;
        Some((id.to_string(), label.to_string()))
    } else {
        validate_id(input).ok()?;
        Some((input.to_string(), input.to_string()))
    }
}

fn parse_relationship(line: &str, classes: &mut Vec<ClassDecl>) -> Option<Result<Edge, String>> {
    const BIDIRECTIONAL_RELATIONS: [&str; 6] = ["<|--|>", "<|..|>", "<-->", "<..>", "*--*", "o--o"];
    let (relation, user_label) = match line.split_once(':') {
        Some((relation, label)) => (relation.trim(), Some(label.trim())),
        None => (line, None),
    };
    if BIDIRECTIONAL_RELATIONS
        .iter()
        .any(|token| relation.contains(token))
    {
        return Some(Err(
            "bidirectional class relationships are not supported".to_string()
        ));
    }

    const RELATIONS: [(&str, EdgeType, bool, Option<&str>); 14] = [
        ("<|--", EdgeType::Arrow, true, None),
        ("--|>", EdgeType::Arrow, false, None),
        ("<|..", EdgeType::DottedArrow, true, Some("realization")),
        ("..|>", EdgeType::DottedArrow, false, Some("realization")),
        ("<--", EdgeType::Arrow, true, None),
        ("-->", EdgeType::Arrow, false, None),
        ("<..", EdgeType::DottedArrow, true, Some("dependency")),
        ("..>", EdgeType::DottedArrow, false, Some("dependency")),
        ("*--", EdgeType::OpenLink, false, Some("composition")),
        ("o--", EdgeType::OpenLink, false, Some("aggregation")),
        ("--*", EdgeType::OpenLink, true, Some("composition")),
        ("--o", EdgeType::OpenLink, true, Some("aggregation")),
        ("--", EdgeType::OpenLink, false, None),
        ("..", EdgeType::DottedLink, false, None),
    ];

    for (token, edge_type, reverse, semantic) in RELATIONS {
        let Some(token_index) = relation.find(token) else {
            continue;
        };
        let left = relation[..token_index].trim();
        let right = relation[token_index + token.len()..].trim();
        let left_id = relationship_endpoint(left)?;
        let right_id = relationship_endpoint(right)?;
        if let Err(error) = validate_id(left_id).and_then(|_| validate_id(right_id)) {
            return Some(Err(error));
        }
        upsert_class(classes, left_id, None);
        upsert_class(classes, right_id, None);
        let label = match (semantic, user_label.filter(|label| !label.is_empty())) {
            (Some(semantic), Some(label)) => Some(format!("{semantic}: {label}")),
            (Some(semantic), None) => Some(semantic.to_string()),
            (None, Some(label)) => Some(label.to_string()),
            (None, None) => None,
        };
        let (from, to) = if reverse {
            (right_id, left_id)
        } else {
            (left_id, right_id)
        };
        return Some(Ok(Edge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type,
            label,
        }));
    }
    None
}

fn relationship_endpoint(input: &str) -> Option<&str> {
    let mut tokens = input
        .split_whitespace()
        .filter(|token| !token.starts_with('"'));
    let endpoint = tokens.next()?;
    tokens.next().is_none().then_some(endpoint)
}

fn upsert_class(classes: &mut Vec<ClassDecl>, id: &str, explicit_label: Option<String>) {
    if let Some(class) = classes.iter_mut().find(|class| class.id == id) {
        if let Some(label) = explicit_label {
            class.label = label;
        }
        return;
    }
    classes.push(ClassDecl {
        id: id.to_string(),
        label: explicit_label.unwrap_or_else(|| id.to_string()),
        attributes: Vec::new(),
        methods: Vec::new(),
        annotations: Vec::new(),
    });
}

fn class_mut<'a>(classes: &'a mut [ClassDecl], id: &str) -> &'a mut ClassDecl {
    classes.iter_mut().find(|class| class.id == id).unwrap()
}

fn add_member(class: &mut ClassDecl, member: &str) {
    if member.contains('(') {
        class.methods.push(member.to_string());
    } else {
        class.attributes.push(member.to_string());
    }
}

fn render_class_label(class: &ClassDecl) -> String {
    let mut lines = class.annotations.clone();
    lines.push(class.label.clone());
    if !class.attributes.is_empty() || !class.methods.is_empty() {
        lines.push("────────".to_string());
        lines.extend(class.attributes.iter().cloned());
        lines.push("────────".to_string());
        lines.extend(class.methods.iter().cloned());
    }
    lines.join("<br/>")
}

fn validate_id(id: &str) -> Result<(), String> {
    if !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '~')
        })
    {
        Ok(())
    } else {
        Err(format!("invalid class identifier: {id}"))
    }
}

fn line_error<T>(line_index: usize, message: impl Into<String>) -> Result<T, String> {
    Err(format!("line {}: {}", line_index + 1, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_declared_after_relationship_keeps_members() {
        let diagram = parse_class(
            "classDiagram\n    A --> B\n    class A {\n        +String name\n        +run()\n    }\n",
        )
        .unwrap();
        let class = diagram.nodes.iter().find(|node| node.id == "A").unwrap();
        assert!(class.label.contains("+String name"));
        assert!(class.label.contains("+run()"));
    }
}
