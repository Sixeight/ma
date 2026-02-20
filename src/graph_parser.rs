use winnow::prelude::*;
use winnow::ascii::{line_ending, space0, space1};
use winnow::combinator::{alt, opt, repeat};
use winnow::token::{take_until, take_while};

use crate::graph_ast::*;

pub fn parse_graph(input: &str) -> Result<GraphDiagram, String> {
    let mut input = input;
    graph_diagram(&mut input).map_err(|e| format!("{e}"))
}

fn graph_diagram(input: &mut &str) -> winnow::Result<GraphDiagram> {
    space0.parse_next(input)?;
    alt(("graph", "flowchart")).parse_next(input)?;
    space1.parse_next(input)?;
    let direction = direction.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let mut nodes: Vec<NodeDecl> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    let lines: Vec<Option<GraphLine>> = repeat(0.., graph_line).parse_next(input)?;
    for line in lines.into_iter().flatten() {
        match line {
            GraphLine::Edge(edge, from_decl, to_decl) => {
                add_node(&mut nodes, from_decl);
                add_node(&mut nodes, to_decl);
                edges.push(edge);
            }
            GraphLine::Node(decl) => {
                add_node(&mut nodes, decl);
            }
        }
    }

    Ok(GraphDiagram {
        direction,
        nodes,
        edges,
    })
}

fn add_node(nodes: &mut Vec<NodeDecl>, decl: NodeDecl) {
    if !nodes.iter().any(|n| n.id == decl.id) {
        nodes.push(decl);
    }
}

#[derive(Debug)]
enum GraphLine {
    Edge(Edge, NodeDecl, NodeDecl),
    Node(NodeDecl),
}

fn graph_line(input: &mut &str) -> winnow::Result<Option<GraphLine>> {
    space0.parse_next(input)?;

    if input.is_empty() {
        return Err(winnow::error::ParserError::from_input(input));
    }

    let result = alt((
        blank_line.map(|_| None),
        edge_line.map(Some),
        alt_edge_line.map(Some),
        node_line.map(Some),
    ))
    .parse_next(input)?;

    Ok(result)
}

fn blank_line(input: &mut &str) -> winnow::Result<()> {
    line_ending.void().parse_next(input)
}

fn direction(input: &mut &str) -> winnow::Result<Direction> {
    alt((
        "TD".value(Direction::TopDown),
        "TB".value(Direction::TopDown),
        "LR".value(Direction::LeftRight),
    ))
    .parse_next(input)
}

fn identifier<'s>(input: &mut &'s str) -> winnow::Result<&'s str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_').parse_next(input)
}

fn node_ref(input: &mut &str) -> winnow::Result<NodeDecl> {
    let id = identifier.parse_next(input)?;
    let label = opt(bracketed_label).parse_next(input)?;
    let label = label.unwrap_or_else(|| id.to_string());
    Ok(NodeDecl {
        id: id.to_string(),
        label,
    })
}

fn bracketed_label(input: &mut &str) -> winnow::Result<String> {
    "[".parse_next(input)?;
    let text = take_while(1.., |c: char| c != ']').parse_next(input)?;
    "]".parse_next(input)?;
    Ok(text.to_string())
}

fn edge_type(input: &mut &str) -> winnow::Result<EdgeType> {
    alt((
        "-->".value(EdgeType::Arrow),
        "---".value(EdgeType::OpenLink),
    ))
    .parse_next(input)
}

fn edge_label(input: &mut &str) -> winnow::Result<String> {
    "|".parse_next(input)?;
    let text = take_while(1.., |c: char| c != '|').parse_next(input)?;
    "|".parse_next(input)?;
    Ok(text.to_string())
}

fn edge_line(input: &mut &str) -> winnow::Result<GraphLine> {
    let from = node_ref.parse_next(input)?;
    space0.parse_next(input)?;
    let et = edge_type.parse_next(input)?;
    let label = opt(edge_label).parse_next(input)?;
    space0.parse_next(input)?;
    let to = node_ref.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let edge = Edge {
        from: from.id.clone(),
        to: to.id.clone(),
        edge_type: et,
        label,
    };
    Ok(GraphLine::Edge(edge, from, to))
}

fn alt_edge_line(input: &mut &str) -> winnow::Result<GraphLine> {
    let from = node_ref.parse_next(input)?;
    space0.parse_next(input)?;
    "-- ".parse_next(input)?;
    let (label_text, et) = alt((
        (take_until(1.., " -->"), " -->".value(EdgeType::Arrow)),
        (take_until(1.., " ---"), " ---".value(EdgeType::OpenLink)),
    ))
    .parse_next(input)?;
    space0.parse_next(input)?;
    let to = node_ref.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let label = label_text.trim().to_string();
    let edge = Edge {
        from: from.id.clone(),
        to: to.id.clone(),
        edge_type: et,
        label: Some(label),
    };
    Ok(GraphLine::Edge(edge, from, to))
}

fn node_line(input: &mut &str) -> winnow::Result<GraphLine> {
    let decl = node_ref.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(GraphLine::Node(decl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_direction_td() {
        let mut input = "TD";
        assert_eq!(direction(&mut input).unwrap(), Direction::TopDown);
    }

    #[test]
    fn parse_direction_tb() {
        let mut input = "TB";
        assert_eq!(direction(&mut input).unwrap(), Direction::TopDown);
    }

    #[test]
    fn parse_direction_lr() {
        let mut input = "LR";
        assert_eq!(direction(&mut input).unwrap(), Direction::LeftRight);
    }

    #[test]
    fn parse_node_ref_with_label() {
        let mut input = "A[Start]";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "Start");
    }

    #[test]
    fn parse_node_ref_without_label() {
        let mut input = "A rest";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "A");
    }

    #[test]
    fn parse_edge_arrow() {
        let mut input = "-->";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::Arrow);
    }

    #[test]
    fn parse_edge_open_link() {
        let mut input = "---";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::OpenLink);
    }

    #[test]
    fn parse_simple_td_graph() {
        let input = "graph TD\n    A[Start] --> B[End]\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.direction, Direction::TopDown);
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.nodes[0].id, "A");
        assert_eq!(diagram.nodes[0].label, "Start");
        assert_eq!(diagram.nodes[1].id, "B");
        assert_eq!(diagram.nodes[1].label, "End");
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].from, "A");
        assert_eq!(diagram.edges[0].to, "B");
        assert_eq!(diagram.edges[0].edge_type, EdgeType::Arrow);
    }

    #[test]
    fn parse_lr_graph() {
        let input = "graph LR\n    A[Start] --> B[End]\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.direction, Direction::LeftRight);
    }

    #[test]
    fn parse_flowchart_keyword() {
        let input = "flowchart TD\n    A --> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.direction, Direction::TopDown);
        assert_eq!(diagram.nodes[0].label, "A");
    }

    #[test]
    fn parse_fan_out() {
        let input = "graph TD\n    A --> B\n    A --> C\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].to, "B");
        assert_eq!(diagram.edges[1].to, "C");
    }

    #[test]
    fn parse_fan_in() {
        let input = "graph TD\n    A --> C\n    B --> C\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
    }

    #[test]
    fn parse_open_link() {
        let input = "graph TD\n    A --- B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::OpenLink);
    }

    #[test]
    fn parse_edge_label_arrow() {
        let input = "graph TD\n    A -->|yes| B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].label, Some("yes".to_string()));
    }

    #[test]
    fn parse_edge_label_open_link() {
        let input = "graph TD\n    A ---|text| B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].label, Some("text".to_string()));
    }

    #[test]
    fn parse_edge_no_label() {
        let input = "graph TD\n    A --> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].label, None);
    }

    #[test]
    fn parse_alt_label_arrow() {
        let input = "graph TD\n    A -- text --> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].edge_type, EdgeType::Arrow);
        assert_eq!(diagram.edges[0].label, Some("text".to_string()));
        assert_eq!(diagram.edges[0].from, "A");
        assert_eq!(diagram.edges[0].to, "B");
    }

    #[test]
    fn parse_alt_label_open_link() {
        let input = "graph TD\n    A -- text --- B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].edge_type, EdgeType::OpenLink);
        assert_eq!(diagram.edges[0].label, Some("text".to_string()));
    }

    #[test]
    fn parse_alt_label_with_spaces() {
        let input = "graph TD\n    A -- hello world --> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(
            diagram.edges[0].label,
            Some("hello world".to_string())
        );
    }

    #[test]
    fn parse_deduplicates_nodes() {
        let input = "graph TD\n    A[Start] --> B\n    A --> C\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        let a_nodes: Vec<_> = diagram.nodes.iter().filter(|n| n.id == "A").collect();
        assert_eq!(a_nodes.len(), 1);
        assert_eq!(a_nodes[0].label, "Start");
    }
}
