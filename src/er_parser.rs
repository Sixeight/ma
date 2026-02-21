use winnow::prelude::*;
use winnow::ascii::{line_ending, space0, space1};
use winnow::combinator::{alt, opt, repeat};
use winnow::token::take_while;

use crate::er_ast::*;

pub fn parse_er(input: &str) -> Result<ErDiagram, String> {
    let mut input = input;
    er_diagram(&mut input).map_err(|e| format!("{e}"))
}

fn er_diagram(input: &mut &str) -> winnow::Result<ErDiagram> {
    space0.parse_next(input)?;
    "erDiagram".parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let lines: Vec<Option<Relationship>> = repeat(0.., er_line).parse_next(input)?;

    let mut entities: Vec<String> = Vec::new();
    let mut relationships: Vec<Relationship> = Vec::new();
    for rel in lines.into_iter().flatten() {
        add_entity(&mut entities, &rel.from);
        add_entity(&mut entities, &rel.to);
        relationships.push(rel);
    }

    Ok(ErDiagram {
        entities,
        relationships,
    })
}

fn er_line(input: &mut &str) -> winnow::Result<Option<Relationship>> {
    alt((
        relationship_line.map(Some),
        blank_line.map(|_| None),
    ))
    .parse_next(input)
}

fn blank_line(input: &mut &str) -> winnow::Result<()> {
    space0.parse_next(input)?;
    line_ending.parse_next(input)?;
    Ok(())
}

fn add_entity(entities: &mut Vec<String>, name: &str) {
    if !entities.iter().any(|e| e == name) {
        entities.push(name.to_string());
    }
}

fn er_identifier<'s>(input: &mut &'s str) -> winnow::Result<&'s str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-').parse_next(input)
}

fn relationship_line(input: &mut &str) -> winnow::Result<Relationship> {
    space0.parse_next(input)?;
    let from = er_identifier.parse_next(input)?;
    space1.parse_next(input)?;
    let (left_card, right_card) = cardinality.parse_next(input)?;
    space1.parse_next(input)?;
    let to = er_identifier.parse_next(input)?;
    space0.parse_next(input)?;
    ":".parse_next(input)?;
    space0.parse_next(input)?;
    let label: &str =
        take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    Ok(Relationship {
        from: from.to_string(),
        to: to.to_string(),
        left_card,
        right_card,
        label: label.trim_end().to_string(),
    })
}

fn cardinality(input: &mut &str) -> winnow::Result<(Cardinality, Cardinality)> {
    let left_str: &str =
        take_while(1.., |c: char| c == '|' || c == 'o' || c == '{' || c == '}')
            .parse_next(input)?;
    "--".parse_next(input)?;
    let right_str: &str =
        take_while(1.., |c: char| c == '|' || c == 'o' || c == '{' || c == '}')
            .parse_next(input)?;
    let left = parse_left_cardinality(left_str);
    let right = parse_right_cardinality(right_str);
    Ok((left, right))
}

fn parse_left_cardinality(s: &str) -> Cardinality {
    match s {
        "||" => Cardinality::ExactlyOne,
        "o|" => Cardinality::ZeroOrOne,
        "}|" => Cardinality::OneOrMany,
        "}o" => Cardinality::ZeroOrMany,
        _ => Cardinality::ExactlyOne,
    }
}

fn parse_right_cardinality(s: &str) -> Cardinality {
    match s {
        "||" => Cardinality::ExactlyOne,
        "|o" => Cardinality::ZeroOrOne,
        "|{" => Cardinality::OneOrMany,
        "o{" => Cardinality::ZeroOrMany,
        _ => Cardinality::ExactlyOne,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_er_identifier_simple() {
        let mut input = "CUSTOMER rest";
        let result = er_identifier(&mut input).unwrap();
        assert_eq!(result, "CUSTOMER");
    }

    #[test]
    fn parse_er_identifier_with_hyphen() {
        let mut input = "LINE-ITEM rest";
        let result = er_identifier(&mut input).unwrap();
        assert_eq!(result, "LINE-ITEM");
    }

    #[test]
    fn parse_cardinality_one_to_many() {
        let mut input = "||--o{ rest";
        let (left, right) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ExactlyOne);
        assert_eq!(right, Cardinality::ZeroOrMany);
    }

    #[test]
    fn parse_cardinality_one_to_one() {
        let mut input = "||--|| rest";
        let (left, right) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ExactlyOne);
        assert_eq!(right, Cardinality::ExactlyOne);
    }

    #[test]
    fn parse_cardinality_many_to_many() {
        let mut input = "}o--o{ rest";
        let (left, right) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ZeroOrMany);
        assert_eq!(right, Cardinality::ZeroOrMany);
    }

    #[test]
    fn parse_cardinality_zero_or_one() {
        let mut input = "o|--|o rest";
        let (left, right) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ZeroOrOne);
        assert_eq!(right, Cardinality::ZeroOrOne);
    }

    #[test]
    fn parse_cardinality_one_or_many() {
        let mut input = "}|--|{ rest";
        let (left, right) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::OneOrMany);
        assert_eq!(right, Cardinality::OneOrMany);
    }

    #[test]
    fn parse_relationship_basic() {
        let mut input = "CUSTOMER ||--o{ ORDER : places\n";
        let rel = relationship_line(&mut input).unwrap();
        assert_eq!(rel.from, "CUSTOMER");
        assert_eq!(rel.to, "ORDER");
        assert_eq!(rel.label, "places");
    }

    #[test]
    fn parse_relationship_label_with_spaces() {
        let mut input = "CUSTOMER }o--|| ADDRESS : billing address\n";
        let rel = relationship_line(&mut input).unwrap();
        assert_eq!(rel.from, "CUSTOMER");
        assert_eq!(rel.to, "ADDRESS");
        assert_eq!(rel.label, "billing address");
    }

    #[test]
    fn parse_er_diagram_single() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(diagram.entities, vec!["CUSTOMER", "ORDER"]);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].label, "places");
    }

    #[test]
    fn parse_er_diagram_chain() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE-ITEM : contains\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(
            diagram.entities,
            vec!["CUSTOMER", "ORDER", "LINE-ITEM"]
        );
        assert_eq!(diagram.relationships.len(), 2);
    }

    #[test]
    fn parse_er_diagram_entity_dedup() {
        let input = "erDiagram\n    A ||--|| B : r1\n    B ||--|| C : r2\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(diagram.entities, vec!["A", "B", "C"]);
    }

    #[test]
    fn parse_er_diagram_blank_lines() {
        let input = "erDiagram\n\n    A ||--|| B : r1\n\n    B ||--|| C : r2\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(diagram.relationships.len(), 2);
    }
}
