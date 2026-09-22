use winnow::ascii::{line_ending, space0, space1};
use winnow::combinator::{alt, opt, repeat};
use winnow::prelude::*;
use winnow::token::{take_until, take_while};

use crate::graph_ast::*;

pub fn parse_graph(input: &str) -> Result<GraphDiagram, String> {
    let mut input = input;
    graph_diagram(&mut input).map_err(|_| {
        let context = input.lines().next().unwrap_or("").trim();
        let context_display = if context.len() > 40 {
            format!("{}...", &context[..40])
        } else {
            context.to_string()
        };
        format!("syntax error in graph diagram: unexpected `{context_display}`")
    })
}

fn graph_diagram(input: &mut &str) -> winnow::Result<GraphDiagram> {
    space0.parse_next(input)?;
    alt(("graph", "flowchart")).parse_next(input)?;
    space1.parse_next(input)?;
    let direction = direction.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let mut nodes: Vec<NodeDecl> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut subgraphs: Vec<Subgraph> = Vec::new();

    let lines: Vec<Option<GraphLine>> = repeat(0.., graph_line).parse_next(input)?;
    for line in lines.into_iter().flatten() {
        collect_line(line, &mut nodes, &mut edges, &mut subgraphs);
    }

    // Mermaid permits edges to target a subgraph ID. Those endpoints constrain
    // layout, but they are not ordinary nodes in the rendered diagram.
    nodes.retain(|node| !subgraphs.iter().any(|subgraph| subgraph.id == node.id));

    Ok(GraphDiagram {
        direction,
        nodes,
        edges,
        subgraphs,
    })
}

fn collect_line(
    line: GraphLine,
    nodes: &mut Vec<NodeDecl>,
    edges: &mut Vec<Edge>,
    subgraphs: &mut Vec<Subgraph>,
) {
    match line {
        GraphLine::Edge(edge, from_decl, to_decl) => {
            add_node(nodes, from_decl);
            add_node(nodes, to_decl);
            edges.push(edge);
        }
        GraphLine::Edges(items) => {
            for (edge, from_decl, to_decl) in items {
                add_node(nodes, from_decl);
                add_node(nodes, to_decl);
                edges.push(edge);
            }
        }
        GraphLine::Node(decl) => {
            add_node(nodes, decl);
        }
        GraphLine::SubgraphBlock(id, label, inner_lines) => {
            let mut sg_node_ids: Vec<String> = Vec::new();
            for inner in inner_lines {
                match &inner {
                    GraphLine::Edge(_, from_decl, to_decl) => {
                        add_subgraph_node_id(&mut sg_node_ids, subgraphs, from_decl);
                        add_subgraph_node_id(&mut sg_node_ids, subgraphs, to_decl);
                    }
                    GraphLine::Edges(items) => {
                        for (_, from_decl, to_decl) in items {
                            add_subgraph_node_id(&mut sg_node_ids, subgraphs, from_decl);
                            add_subgraph_node_id(&mut sg_node_ids, subgraphs, to_decl);
                        }
                    }
                    GraphLine::Node(decl) => {
                        add_subgraph_node_id(&mut sg_node_ids, subgraphs, decl);
                    }
                    GraphLine::SubgraphBlock(_, _, _) => {}
                }
                collect_line(inner, nodes, edges, subgraphs);
            }
            subgraphs.push(Subgraph {
                id,
                label,
                node_ids: sg_node_ids,
            });
        }
    }
}

fn add_subgraph_node_id(
    subgraph_node_ids: &mut Vec<String>,
    subgraphs: &[Subgraph],
    decl: &NodeDecl,
) {
    let belongs_to_current = subgraph_node_ids.contains(&decl.id);
    let owned_by_earlier_subgraph = subgraphs
        .iter()
        .any(|subgraph| subgraph.node_ids.contains(&decl.id));
    if !belongs_to_current && !owned_by_earlier_subgraph {
        subgraph_node_ids.push(decl.id.clone());
    }
}

fn add_node(nodes: &mut Vec<NodeDecl>, decl: NodeDecl) {
    if !nodes.iter().any(|n| n.id == decl.id) {
        nodes.push(decl);
    }
}

#[derive(Debug)]
enum GraphLine {
    Edge(Edge, NodeDecl, NodeDecl),
    Edges(Vec<(Edge, NodeDecl, NodeDecl)>),
    Node(NodeDecl),
    SubgraphBlock(String, String, Vec<GraphLine>),
}

fn graph_line(input: &mut &str) -> winnow::Result<Option<GraphLine>> {
    space0.parse_next(input)?;

    if input.is_empty() {
        return Err(winnow::error::ParserError::from_input(input));
    }

    let result = alt((
        blank_line.map(|_| None),
        directive_line.map(|_| None),
        subgraph_block.map(Some),
        edge_line.map(Some),
        dotted_labeled_edge_line.map(Some),
        alt_edge_line.map(Some),
        node_line.map(Some),
    ))
    .parse_next(input)?;

    Ok(result)
}

fn subgraph_block(input: &mut &str) -> winnow::Result<GraphLine> {
    "subgraph".parse_next(input)?;
    space1.parse_next(input)?;
    let header = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    let (id, label) = parse_subgraph_header(header.trim_end());
    opt(line_ending).parse_next(input)?;

    let mut inner_lines: Vec<GraphLine> = Vec::new();
    loop {
        space0.parse_next(input)?;
        if input.starts_with("end") {
            "end".parse_next(input)?;
            opt(line_ending).parse_next(input)?;
            break;
        }
        if input.is_empty() {
            break;
        }
        if let Some(line) = graph_line(input)? {
            inner_lines.push(line);
        }
    }

    Ok(GraphLine::SubgraphBlock(id, label, inner_lines))
}

fn parse_subgraph_header(header: &str) -> (String, String) {
    if let Some((id, title)) = header
        .strip_suffix(']')
        .and_then(|header| header.split_once('['))
    {
        return (
            id.trim_end().to_string(),
            title.trim_matches('"').to_string(),
        );
    }
    (header.replace(' ', "_").to_lowercase(), header.to_string())
}

fn blank_line(input: &mut &str) -> winnow::Result<()> {
    line_ending.void().parse_next(input)
}

fn directive_line(input: &mut &str) -> winnow::Result<()> {
    alt((
        "classDef",
        "linkStyle",
        "direction",
        "click",
        "style",
        "class",
        "link",
    ))
    .parse_next(input)?;
    space1.parse_next(input)?;
    let _ = take_while(0.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(())
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
    let original = *input;
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_').parse_next(input)?;

    loop {
        let checkpoint = *input;
        let Some(after_hyphen) = input.strip_prefix('-') else {
            break;
        };
        let segment_len = after_hyphen
            .char_indices()
            .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
            .map(|(index, _)| index)
            .unwrap_or(after_hyphen.len());
        if segment_len == 0 {
            *input = checkpoint;
            break;
        }
        *input = &after_hyphen[segment_len..];
    }

    let consumed = original.len() - input.len();
    Ok(&original[..consumed])
}

fn node_ref(input: &mut &str) -> winnow::Result<NodeDecl> {
    let id = identifier.parse_next(input)?;
    let shape_label = opt(shape_label).parse_next(input)?;
    let (shape, label) = shape_label.unwrap_or_else(|| (NodeShape::Box, id.to_string()));
    Ok(NodeDecl {
        id: id.to_string(),
        label,
        shape,
    })
}

fn shape_label(input: &mut &str) -> winnow::Result<(NodeShape, String)> {
    alt((
        circle_label.map(|l| (NodeShape::Circle, l)),
        stadium_label.map(|l| (NodeShape::Stadium, l)),
        subroutine_label.map(|l| (NodeShape::Subroutine, l)),
        cylinder_label.map(|l| (NodeShape::Cylinder, l)),
        hexagon_label.map(|l| (NodeShape::Hexagon, l)),
        round_label.map(|l| (NodeShape::Round, l)),
        diamond_label.map(|l| (NodeShape::Diamond, l)),
        bracketed_label.map(|l| (NodeShape::Box, l)),
    ))
    .parse_next(input)
}

fn delimited_label(
    input: &mut &str,
    mut opening: &str,
    mut closing: &str,
) -> winnow::Result<String> {
    opening.parse_next(input)?;
    let text = take_until(1.., closing).parse_next(input)?;
    closing.parse_next(input)?;
    Ok(text.trim_matches('"').to_string())
}

fn stadium_label(input: &mut &str) -> winnow::Result<String> {
    delimited_label(input, "([", "])")
}

fn subroutine_label(input: &mut &str) -> winnow::Result<String> {
    delimited_label(input, "[[", "]]")
}

fn cylinder_label(input: &mut &str) -> winnow::Result<String> {
    delimited_label(input, "[(", ")]")
}

fn hexagon_label(input: &mut &str) -> winnow::Result<String> {
    delimited_label(input, "{{", "}}")
}

fn quoted_inner(quote: char, closer: char) -> impl FnMut(&mut &str) -> winnow::Result<String> {
    move |input: &mut &str| {
        if input.starts_with(quote) {
            let _q: char = winnow::token::any.parse_next(input)?;
            let text = take_while(1.., move |c: char| c != quote).parse_next(input)?;
            let result = text.to_string();
            let _q2: char = winnow::token::any.parse_next(input)?;
            Ok(result)
        } else {
            let text = take_while(1.., move |c: char| c != closer).parse_next(input)?;
            Ok(text.to_string())
        }
    }
}

fn round_label(input: &mut &str) -> winnow::Result<String> {
    "(".parse_next(input)?;
    let text = quoted_inner('"', ')').parse_next(input)?;
    ")".parse_next(input)?;
    Ok(text)
}

fn diamond_label(input: &mut &str) -> winnow::Result<String> {
    "{".parse_next(input)?;
    let text = quoted_inner('"', '}').parse_next(input)?;
    "}".parse_next(input)?;
    Ok(text)
}

fn circle_label(input: &mut &str) -> winnow::Result<String> {
    "((".parse_next(input)?;
    let text = take_while(1.., |c: char| c != ')').parse_next(input)?;
    "))".parse_next(input)?;
    Ok(text.to_string())
}

fn bracketed_label(input: &mut &str) -> winnow::Result<String> {
    "[".parse_next(input)?;
    let text = quoted_inner('"', ']').parse_next(input)?;
    "]".parse_next(input)?;
    Ok(text)
}

fn edge_type(input: &mut &str) -> winnow::Result<EdgeType> {
    alt((
        "~~~".value(EdgeType::Invisible),
        "-.->".value(EdgeType::DottedArrow),
        "-.-".value(EdgeType::DottedLink),
        "==>".value(EdgeType::ThickArrow),
        "===".value(EdgeType::ThickLink),
        "-->".value(EdgeType::Arrow),
        "---".value(EdgeType::OpenLink),
    ))
    .parse_next(input)
}

fn edge_label(input: &mut &str) -> winnow::Result<String> {
    "|".parse_next(input)?;
    let text = take_while(1.., |c: char| c != '|').parse_next(input)?;
    "|".parse_next(input)?;
    let text = text
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text);
    Ok(text.to_string())
}

fn edge_line(input: &mut &str) -> winnow::Result<GraphLine> {
    let from = node_ref.parse_next(input)?;
    let mut segment_froms = vec![from];
    let mut items = Vec::new();
    loop {
        space0.parse_next(input)?;
        let Some(et) = opt(edge_type).parse_next(input)? else {
            break;
        };
        let label = opt(edge_label).parse_next(input)?;
        space0.parse_next(input)?;
        let first_to = node_ref.parse_next(input)?;
        let mut targets = vec![first_to];

        loop {
            space0.parse_next(input)?;
            if opt("&").parse_next(input)?.is_none() {
                break;
            }
            space0.parse_next(input)?;
            let target = node_ref.parse_next(input)?;
            targets.push(target);
        }
        for segment_from in &segment_froms {
            for target in &targets {
                items.push((
                    Edge {
                        from: segment_from.id.clone(),
                        to: target.id.clone(),
                        edge_type: et,
                        label: label.clone(),
                    },
                    segment_from.clone(),
                    target.clone(),
                ));
            }
        }
        segment_froms = targets;
    }
    if items.is_empty() {
        return Err(winnow::error::ParserError::from_input(input));
    }
    opt(line_ending).parse_next(input)?;

    if items.len() == 1 {
        let (edge, from, to) = items.pop().unwrap();
        Ok(GraphLine::Edge(edge, from, to))
    } else {
        Ok(GraphLine::Edges(items))
    }
}

fn dotted_labeled_edge_line(input: &mut &str) -> winnow::Result<GraphLine> {
    let from = node_ref.parse_next(input)?;
    space0.parse_next(input)?;
    "-. ".parse_next(input)?;
    let label = take_until(1.., " .->")
        .parse_next(input)?
        .trim()
        .to_string();
    " .->".parse_next(input)?;
    space0.parse_next(input)?;
    let to = node_ref.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(GraphLine::Edge(
        Edge {
            from: from.id.clone(),
            to: to.id.clone(),
            edge_type: EdgeType::DottedArrow,
            label: Some(label),
        },
        from,
        to,
    ))
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
    fn parse_hyphenated_node_ids() {
        let diagram =
            parse_graph("flowchart LR\n    alpha-step --> beta-step --> gamma-step\n").unwrap();

        assert_eq!(
            diagram
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha-step", "beta-step", "gamma-step"]
        );
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].from, "alpha-step");
        assert_eq!(diagram.edges[0].to, "beta-step");
        assert_eq!(diagram.edges[1].from, "beta-step");
        assert_eq!(diagram.edges[1].to, "gamma-step");
    }

    #[test]
    fn referenced_node_keeps_its_original_subgraph_membership() {
        let diagram = parse_graph(
            "flowchart LR\n\
             subgraph first\n\
               source --> shared\n\
             end\n\
             subgraph second\n\
               shared --> target\n\
             end\n",
        )
        .unwrap();

        assert_eq!(diagram.subgraphs[0].node_ids, vec!["source", "shared"]);
        assert_eq!(diagram.subgraphs[1].node_ids, vec!["target"]);
    }

    #[test]
    fn bare_node_can_become_owned_by_a_later_subgraph() {
        let diagram = parse_graph(
            "flowchart LR\n\
             outside --> shared\n\
             subgraph group\n\
               shared --> target\n\
             end\n",
        )
        .unwrap();

        assert_eq!(diagram.subgraphs[0].node_ids, vec!["shared", "target"]);
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
    fn parse_quoted_edge_label_preserves_text_and_breaks() {
        let input = "flowchart LR\n    A -->|\"検査対象<br>長い説明を含む経路\"| B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(
            diagram.edges[0].label.as_deref(),
            Some("検査対象<br>長い説明を含む経路")
        );
    }

    #[test]
    fn parse_edge_label_preserves_quotes_within_text() {
        for label in [
            "say \"yes\"",
            "\"unfinished",
            "unfinished\"",
            " spaced text ",
        ] {
            let input = format!("graph LR\n    A -->|{label}| B\n");
            let diagram = parse_graph(&input).unwrap();
            assert_eq!(diagram.edges[0].label.as_deref(), Some(label));
        }
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
        assert_eq!(diagram.edges[0].label, Some("hello world".to_string()));
    }

    #[test]
    fn parse_edge_dotted_arrow() {
        let mut input = "-.->rest";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::DottedArrow);
        assert_eq!(input, "rest");
    }

    #[test]
    fn parse_edge_dotted_link() {
        let mut input = "-.-rest";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::DottedLink);
        assert_eq!(input, "rest");
    }

    #[test]
    fn parse_edge_thick_arrow() {
        let mut input = "==>rest";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::ThickArrow);
        assert_eq!(input, "rest");
    }

    #[test]
    fn parse_edge_thick_link() {
        let mut input = "===rest";
        assert_eq!(edge_type(&mut input).unwrap(), EdgeType::ThickLink);
        assert_eq!(input, "rest");
    }

    #[test]
    fn parse_dotted_arrow_graph() {
        let input = "graph TD\n    A -.-> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::DottedArrow);
    }

    #[test]
    fn parse_thick_arrow_graph() {
        let input = "graph TD\n    A ==> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::ThickArrow);
    }

    #[test]
    fn parse_dotted_arrow_with_label() {
        let input = "graph TD\n    A -.->|yes| B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::DottedArrow);
        assert_eq!(diagram.edges[0].label, Some("yes".to_string()));
    }

    #[test]
    fn parse_thick_arrow_with_label() {
        let input = "graph TD\n    A ==>|yes| B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::ThickArrow);
        assert_eq!(diagram.edges[0].label, Some("yes".to_string()));
    }

    #[test]
    fn parse_node_ref_round() {
        let mut input = "A(Round)";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "Round");
        assert_eq!(n.shape, NodeShape::Round);
    }

    #[test]
    fn parse_node_ref_diamond() {
        let mut input = "A{Diamond}";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "Diamond");
        assert_eq!(n.shape, NodeShape::Diamond);
    }

    #[test]
    fn parse_node_ref_circle() {
        let mut input = "A((Circle))";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "Circle");
        assert_eq!(n.shape, NodeShape::Circle);
    }

    #[test]
    fn parse_node_ref_box_shape() {
        let mut input = "A[Box]";
        let n = node_ref(&mut input).unwrap();
        assert_eq!(n.id, "A");
        assert_eq!(n.label, "Box");
        assert_eq!(n.shape, NodeShape::Box);
    }

    #[test]
    fn parse_additional_node_shapes() {
        let cases = [
            ("A([Stadium])", NodeShape::Stadium),
            ("A[[Subroutine]]", NodeShape::Subroutine),
            ("A[(Database)]", NodeShape::Cylinder),
            ("A{{Hexagon}}", NodeShape::Hexagon),
        ];
        for (mut input, expected) in cases {
            let node = node_ref(&mut input).unwrap();
            assert_eq!(node.shape, expected, "input: {input}");
        }
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

    #[test]
    fn parse_subgraph_basic() {
        let input = "graph TD\n    subgraph Backend\n        A --> B\n    end\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].label, "Backend");
        assert_eq!(diagram.subgraphs[0].node_ids, vec!["A", "B"]);
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn parse_subgraph_with_labeled_nodes() {
        let input = "graph TD\n    subgraph Backend\n        A[API] --> B[DB]\n    end\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].label, "Backend");
        assert_eq!(diagram.nodes[0].label, "API");
        assert_eq!(diagram.nodes[1].label, "DB");
    }

    #[test]
    fn parse_subgraph_with_standalone_node() {
        let input = "graph TD\n    subgraph Group\n        A\n    end\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].node_ids, vec!["A"]);
        assert_eq!(diagram.nodes.len(), 1);
    }

    #[test]
    fn parse_quoted_bracket_label() {
        let input = "graph TD\n    A[\"[NOTE] Hello World\"] --> B\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.nodes[0].label, "[NOTE] Hello World");
        assert_eq!(diagram.nodes[0].shape, NodeShape::Box);
    }

    #[test]
    fn parse_quoted_round_label() {
        let input = "graph TD\n    A(\"(inner) text\")\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes[0].label, "(inner) text");
        assert_eq!(diagram.nodes[0].shape, NodeShape::Round);
    }

    #[test]
    fn parse_quoted_diamond_label() {
        let input = "graph TD\n    A{\"choice {A}\"}\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes[0].label, "choice {A}");
        assert_eq!(diagram.nodes[0].shape, NodeShape::Diamond);
    }

    #[test]
    fn parse_subgraph_mixed_with_outer_nodes() {
        let input =
            "graph TD\n    C\n    subgraph Backend\n        A --> B\n    end\n    C --> A\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].node_ids, vec!["A", "B"]);
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
    }

    #[test]
    fn parse_chained_edges() {
        let diagram = parse_graph("graph LR\n    A --> B --> C\n").unwrap();
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(
            (&diagram.edges[0].from, &diagram.edges[0].to),
            (&"A".into(), &"B".into())
        );
        assert_eq!(
            (&diagram.edges[1].from, &diagram.edges[1].to),
            (&"B".into(), &"C".into())
        );
    }

    #[test]
    fn parse_chained_edges_after_fan_out() {
        let diagram = parse_graph("graph LR\n    A --> B & C --> D\n").unwrap();
        let pairs: Vec<_> = diagram
            .edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        assert_eq!(pairs, vec![("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")]);
    }

    #[test]
    fn parse_dotted_edge_label_syntax() {
        let diagram = parse_graph("graph LR\n    A -. retry .-> B\n").unwrap();
        assert_eq!(diagram.edges[0].edge_type, EdgeType::DottedArrow);
        assert_eq!(diagram.edges[0].label.as_deref(), Some("retry"));
    }

    #[test]
    fn parse_subgraph_id_and_title() {
        let diagram =
            parse_graph("graph TD\n    subgraph api [Public API]\n        A\n    end\n").unwrap();
        assert_eq!(diagram.subgraphs[0].id, "api");
        assert_eq!(diagram.subgraphs[0].label, "Public API");
    }

    #[test]
    fn ignore_non_rendering_directives() {
        let input = "graph TD\n    click A href \"https://example.com\"\n    link A \"https://example.com\"\n    subgraph S\n        direction LR\n        A --> B\n    end\n";
        let diagram = parse_graph(input).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
    }
}
