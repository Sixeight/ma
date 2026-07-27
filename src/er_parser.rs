use winnow::ascii::{line_ending, space0, space1, till_line_ending};
use winnow::combinator::{alt, eof, opt, repeat};
use winnow::prelude::*;
use winnow::token::take_while;

use crate::er_ast::*;

pub fn parse_er(input: &str) -> Result<ErDiagram, String> {
    let mut input = input;
    er_diagram(&mut input).map_err(|_| {
        let context = input.lines().next().unwrap_or("").trim();
        let context_display = if context.len() > 40 {
            format!("{}...", &context[..40])
        } else {
            context.to_string()
        };
        format!("syntax error in ER diagram: unexpected `{context_display}`")
    })
}

fn er_diagram(input: &mut &str) -> winnow::Result<ErDiagram> {
    space0.parse_next(input)?;
    "erDiagram".parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let lines: Vec<Option<ErLine>> = repeat(0.., er_line).parse_next(input)?;
    space0.parse_next(input)?;
    eof.parse_next(input)?;

    let mut entities: Vec<Entity> = Vec::new();
    let mut relationships: Vec<Relationship> = Vec::new();
    for line in lines.into_iter().flatten() {
        match line {
            ErLine::Relationship(parsed) => {
                add_entity(&mut entities, &parsed.relationship.from, parsed.from_alias);
                add_entity(&mut entities, &parsed.relationship.to, parsed.to_alias);
                relationships.push(parsed.relationship);
            }
            ErLine::EntityBlock(name, alias, attrs) => {
                if let Some(e) = entities.iter_mut().find(|e| e.name == name) {
                    e.alias = alias;
                    e.attributes = attrs;
                } else {
                    entities.push(Entity {
                        name,
                        alias,
                        attributes: attrs,
                    });
                }
            }
        }
    }

    Ok(ErDiagram {
        entities,
        relationships,
    })
}

#[derive(Debug)]
enum ErLine {
    Relationship(ParsedRelationship),
    EntityBlock(String, Option<String>, Vec<EntityAttribute>),
}

#[derive(Debug)]
struct ParsedRelationship {
    relationship: Relationship,
    from_alias: Option<String>,
    to_alias: Option<String>,
}

fn er_line(input: &mut &str) -> winnow::Result<Option<ErLine>> {
    alt((
        entity_block.map(|(name, alias, attrs)| Some(ErLine::EntityBlock(name, alias, attrs))),
        relationship_line.map(|r| Some(ErLine::Relationship(r))),
        blank_line.map(|_| None),
    ))
    .parse_next(input)
}

fn blank_line(input: &mut &str) -> winnow::Result<()> {
    space0.parse_next(input)?;
    line_ending.parse_next(input)?;
    Ok(())
}

fn add_entity(entities: &mut Vec<Entity>, name: &str, alias: Option<String>) {
    if let Some(entity) = entities.iter_mut().find(|e| e.name == name) {
        if alias.is_some() {
            entity.alias = alias;
        }
    } else {
        entities.push(Entity {
            name: name.to_string(),
            alias,
            attributes: Vec::new(),
        });
    }
}

fn entity_block(
    input: &mut &str,
) -> winnow::Result<(String, Option<String>, Vec<EntityAttribute>)> {
    space0.parse_next(input)?;
    let (name, alias) = entity_ref.parse_next(input)?;
    space0.parse_next(input)?;
    "{".parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let mut attrs = Vec::new();
    loop {
        space0.parse_next(input)?;
        if input.starts_with('}') {
            "}".parse_next(input)?;
            opt(line_ending).parse_next(input)?;
            break;
        }
        if input.is_empty() {
            return Err(winnow::error::ParserError::from_input(input));
        }
        if let Ok(()) = blank_line(input) {
            continue;
        }
        let attr = entity_attribute.parse_next(input)?;
        attrs.push(attr);
    }

    Ok((name.to_string(), alias, attrs))
}

fn entity_attribute(input: &mut &str) -> winnow::Result<EntityAttribute> {
    space0.parse_next(input)?;
    let attr_type = er_identifier.parse_next(input)?;
    space1.parse_next(input)?;
    let name = er_identifier.parse_next(input)?;
    let tail = till_line_ending.parse_next(input)?;
    if tail.contains('"') && !tail.trim_end().ends_with('"') {
        return Err(winnow::error::ParserError::from_input(input));
    }
    opt(line_ending).parse_next(input)?;
    let (key, comment) = parse_attribute_tail(tail);

    Ok(EntityAttribute {
        attr_type: attr_type.to_string(),
        name: name.to_string(),
        key,
        comment,
    })
}

fn parse_attribute_tail(tail: &str) -> (Option<String>, Option<String>) {
    let tail = tail.trim();
    if let Some(quote_start) = tail.find('"') {
        let key = tail[..quote_start].trim();
        let comment = tail[quote_start + 1..].strip_suffix('"').unwrap();
        (
            (!key.is_empty()).then(|| key.to_string()),
            Some(comment.to_string()),
        )
    } else {
        ((!tail.is_empty()).then(|| tail.to_string()), None)
    }
}

fn er_identifier<'s>(input: &mut &'s str) -> winnow::Result<&'s str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-').parse_next(input)
}

fn entity_ref<'s>(input: &mut &'s str) -> winnow::Result<(&'s str, Option<String>)> {
    let name = er_identifier.parse_next(input)?;
    let alias = opt(entity_alias).parse_next(input)?;
    Ok((name, alias))
}

fn entity_alias(input: &mut &str) -> winnow::Result<String> {
    "[".parse_next(input)?;
    let quoted = opt("\"").parse_next(input)?.is_some();
    let closer = if quoted { '"' } else { ']' };
    let alias = take_while(1.., |c: char| c != closer).parse_next(input)?;
    if quoted {
        "\"".parse_next(input)?;
    }
    "]".parse_next(input)?;
    Ok(alias.to_string())
}

fn relationship_line(input: &mut &str) -> winnow::Result<ParsedRelationship> {
    space0.parse_next(input)?;
    let (from, from_alias) = entity_ref.parse_next(input)?;
    space1.parse_next(input)?;
    let (left_card, right_card, line_style) = cardinality.parse_next(input)?;
    space1.parse_next(input)?;
    let (to, to_alias) = entity_ref.parse_next(input)?;
    space0.parse_next(input)?;
    ":".parse_next(input)?;
    space0.parse_next(input)?;
    let label: &str = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    Ok(ParsedRelationship {
        from_alias,
        to_alias,
        relationship: Relationship {
            from: from.to_string(),
            to: to.to_string(),
            left_card,
            right_card,
            line_style,
            label: label.trim_end().to_string(),
        },
    })
}

fn cardinality(
    input: &mut &str,
) -> winnow::Result<(Cardinality, Cardinality, RelationshipLineStyle)> {
    let left_str: &str = take_while(1.., |c: char| c == '|' || c == 'o' || c == '{' || c == '}')
        .parse_next(input)?;
    let line_style = alt((
        "--".value(RelationshipLineStyle::Identifying),
        "..".value(RelationshipLineStyle::NonIdentifying),
    ))
    .parse_next(input)?;
    let right_str: &str = take_while(1.., |c: char| c == '|' || c == 'o' || c == '{' || c == '}')
        .parse_next(input)?;
    let left = parse_left_cardinality(left_str);
    let right = parse_right_cardinality(right_str);
    Ok((left, right, line_style))
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
        let (left, right, _) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ExactlyOne);
        assert_eq!(right, Cardinality::ZeroOrMany);
    }

    #[test]
    fn parse_cardinality_one_to_one() {
        let mut input = "||--|| rest";
        let (left, right, _) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ExactlyOne);
        assert_eq!(right, Cardinality::ExactlyOne);
    }

    #[test]
    fn parse_cardinality_many_to_many() {
        let mut input = "}o--o{ rest";
        let (left, right, _) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ZeroOrMany);
        assert_eq!(right, Cardinality::ZeroOrMany);
    }

    #[test]
    fn parse_cardinality_zero_or_one() {
        let mut input = "o|--|o rest";
        let (left, right, _) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::ZeroOrOne);
        assert_eq!(right, Cardinality::ZeroOrOne);
    }

    #[test]
    fn parse_cardinality_one_or_many() {
        let mut input = "}|--|{ rest";
        let (left, right, _) = cardinality(&mut input).unwrap();
        assert_eq!(input, " rest");
        assert_eq!(left, Cardinality::OneOrMany);
        assert_eq!(right, Cardinality::OneOrMany);
    }

    #[test]
    fn parse_relationship_basic() {
        let mut input = "CUSTOMER ||--o{ ORDER : places\n";
        let rel = relationship_line(&mut input).unwrap().relationship;
        assert_eq!(rel.from, "CUSTOMER");
        assert_eq!(rel.to, "ORDER");
        assert_eq!(rel.label, "places");
    }

    #[test]
    fn parse_relationship_label_with_spaces() {
        let mut input = "CUSTOMER }o--|| ADDRESS : billing address\n";
        let rel = relationship_line(&mut input).unwrap().relationship;
        assert_eq!(rel.from, "CUSTOMER");
        assert_eq!(rel.to, "ADDRESS");
        assert_eq!(rel.label, "billing address");
    }

    #[test]
    fn parse_non_identifying_relationship() {
        let mut input = "CUSTOMER ||..o{ ORDER : places\n";
        let rel = relationship_line(&mut input).unwrap().relationship;
        assert_eq!(rel.line_style, RelationshipLineStyle::NonIdentifying);
    }

    #[test]
    fn parse_entity_alias() {
        let diagram = parse_er("erDiagram\n    CUSTOMER[\"Customer Account\"] {\n    }\n").unwrap();
        assert_eq!(diagram.entities[0].name, "CUSTOMER");
        assert_eq!(
            diagram.entities[0].alias.as_deref(),
            Some("Customer Account")
        );
    }

    #[test]
    fn parse_attribute_comment() {
        let diagram =
            parse_er("erDiagram\n    CUSTOMER {\n        string name \"Full legal name\"\n    }\n")
                .unwrap();
        assert_eq!(diagram.entities[0].attributes[0].key, None);
        assert_eq!(
            diagram.entities[0].attributes[0].comment.as_deref(),
            Some("Full legal name")
        );
    }

    #[test]
    fn reject_unterminated_attribute_comment() {
        let input = "erDiagram\n    CUSTOMER {\n        string name \"unterminated\n    }\n";
        assert!(parse_er(input).is_err());
    }

    fn entity_names(diagram: &ErDiagram) -> Vec<&str> {
        diagram.entities.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn parse_er_diagram_single() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(entity_names(&diagram), vec!["CUSTOMER", "ORDER"]);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].label, "places");
    }

    #[test]
    fn parse_er_diagram_chain() {
        let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE-ITEM : contains\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(
            entity_names(&diagram),
            vec!["CUSTOMER", "ORDER", "LINE-ITEM"]
        );
        assert_eq!(diagram.relationships.len(), 2);
    }

    #[test]
    fn parse_er_diagram_entity_dedup() {
        let input = "erDiagram\n    A ||--|| B : r1\n    B ||--|| C : r2\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(entity_names(&diagram), vec!["A", "B", "C"]);
    }

    #[test]
    fn parse_er_diagram_blank_lines() {
        let input = "erDiagram\n\n    A ||--|| B : r1\n\n    B ||--|| C : r2\n";
        let diagram = parse_er(input).unwrap();
        assert_eq!(diagram.relationships.len(), 2);
    }
}
