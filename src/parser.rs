use winnow::prelude::*;
use winnow::ascii::{line_ending, space0, space1, till_line_ending};
use winnow::combinator::{alt, opt, preceded, repeat};
use winnow::token::take_while;

use crate::ast::*;

pub fn parse_diagram(input: &str) -> Result<Diagram, String> {
    let mut input = input;
    diagram(&mut input).map_err(|e| format!("{e}"))
}

fn diagram(input: &mut &str) -> winnow::Result<Diagram> {
    space0.parse_next(input)?;
    "sequenceDiagram".parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    let statements: Vec<Option<Statement>> = repeat(0.., statement).parse_next(input)?;
    let statements = statements.into_iter().flatten().collect();

    Ok(Diagram { statements })
}

fn statement(input: &mut &str) -> winnow::Result<Option<Statement>> {
    space0.parse_next(input)?;

    if input.is_empty() {
        return Err(winnow::error::ParserError::from_input(input));
    }

    let result = alt((
        comment_line.map(|_| None),
        blank_line.map(|_| None),
        participant_decl.map(|p| Some(Statement::ParticipantDecl(p))),
        note_stmt.map(|n| Some(Statement::Note(n))),
        activate_stmt.map(|id| Some(Statement::Activate(id))),
        deactivate_stmt.map(|id| Some(Statement::Deactivate(id))),
        message.map(|m| Some(Statement::Message(m))),
    ))
    .parse_next(input)?;

    Ok(result)
}

fn comment_line(input: &mut &str) -> winnow::Result<()> {
    "%%".parse_next(input)?;
    till_line_ending.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(())
}

fn blank_line(input: &mut &str) -> winnow::Result<()> {
    line_ending.void().parse_next(input)
}

fn activate_stmt<'s>(input: &mut &'s str) -> winnow::Result<String> {
    "activate".parse_next(input)?;
    space1.parse_next(input)?;
    let id = identifier.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(id.to_string())
}

fn deactivate_stmt<'s>(input: &mut &'s str) -> winnow::Result<String> {
    "deactivate".parse_next(input)?;
    space1.parse_next(input)?;
    let id = identifier.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(id.to_string())
}

fn participant_decl(input: &mut &str) -> winnow::Result<ParticipantDecl> {
    "participant".parse_next(input)?;
    space1.parse_next(input)?;
    let id = identifier.parse_next(input)?;

    let alias = opt(preceded((space1, "as", space1), till_line_ending)).parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    Ok(ParticipantDecl {
        id: id.to_string(),
        alias: alias.map(|s: &str| s.trim().to_string()),
    })
}

fn note_stmt(input: &mut &str) -> winnow::Result<Note> {
    "Note".parse_next(input)?;
    space1.parse_next(input)?;

    let placement = alt((
        ("right of", space1, identifier).map(|(_, _, id): (&str, &str, &str)| {
            NotePlacement::RightOf(id.to_string())
        }),
        ("left of", space1, identifier).map(|(_, _, id): (&str, &str, &str)| {
            NotePlacement::LeftOf(id.to_string())
        }),
        ("over", space1, identifier, ",", space0, identifier).map(
            |(_, _, a, _, _, b): (&str, &str, &str, &str, &str, &str)| {
                NotePlacement::OverTwo(a.to_string(), b.to_string())
            },
        ),
        ("over", space1, identifier).map(|(_, _, id): (&str, &str, &str)| {
            NotePlacement::Over(id.to_string())
        }),
    ))
    .parse_next(input)?;

    space0.parse_next(input)?;
    ":".parse_next(input)?;
    space0.parse_next(input)?;
    let text = till_line_ending.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    Ok(Note {
        placement,
        text: text.trim().to_string(),
    })
}

fn message(input: &mut &str) -> winnow::Result<Message> {
    let from = identifier.parse_next(input)?;
    space0.parse_next(input)?;
    let arr = arrow.parse_next(input)?;

    let modifier = opt(alt(("+".value('+'), "-".value('-')))).parse_next(input)?;
    space0.parse_next(input)?;
    let to = identifier.parse_next(input)?;
    space0.parse_next(input)?;
    ":".parse_next(input)?;
    space0.parse_next(input)?;
    let text = till_line_ending.parse_next(input)?;
    opt(line_ending).parse_next(input)?;

    Ok(Message {
        from: from.to_string(),
        to: to.to_string(),
        arrow: arr,
        text: text.trim().to_string(),
        activate_target: modifier == Some('+'),
        deactivate_source: modifier == Some('-'),
    })
}

fn arrow(input: &mut &str) -> winnow::Result<Arrow> {
    let line_style = alt((
        "--".value(LineStyle::Dotted),
        "-".value(LineStyle::Solid),
    ))
    .parse_next(input)?;

    let head = alt((
        ">>".value(ArrowHead::Arrowhead),
        ">".value(ArrowHead::None),
        "x".value(ArrowHead::Cross),
        ")".value(ArrowHead::Open),
    ))
    .parse_next(input)?;

    Ok(Arrow { line_style, head })
}

fn identifier<'s>(input: &mut &'s str) -> winnow::Result<&'s str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_').parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // --- identifier ---

    #[test]
    fn parse_identifier_simple() {
        let mut input = "Alice";
        assert_eq!(identifier(&mut input).unwrap(), "Alice");
        assert_eq!(input, "");
    }

    #[test]
    fn parse_identifier_stops_at_arrow() {
        let mut input = "Alice->>Bob";
        assert_eq!(identifier(&mut input).unwrap(), "Alice");
        assert_eq!(input, "->>Bob");
    }

    #[test]
    fn parse_identifier_with_underscore() {
        let mut input = "my_actor rest";
        assert_eq!(identifier(&mut input).unwrap(), "my_actor");
        assert_eq!(input, " rest");
    }

    // --- arrow ---

    #[test]
    fn parse_arrow_solid_none() {
        let mut input = "->Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Solid);
        assert_eq!(a.head, ArrowHead::None);
    }

    #[test]
    fn parse_arrow_solid_arrowhead() {
        let mut input = "->>Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Solid);
        assert_eq!(a.head, ArrowHead::Arrowhead);
    }

    #[test]
    fn parse_arrow_dotted_none() {
        let mut input = "-->Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Dotted);
        assert_eq!(a.head, ArrowHead::None);
    }

    #[test]
    fn parse_arrow_dotted_arrowhead() {
        let mut input = "-->>Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Dotted);
        assert_eq!(a.head, ArrowHead::Arrowhead);
    }

    #[test]
    fn parse_arrow_solid_cross() {
        let mut input = "-xBob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Solid);
        assert_eq!(a.head, ArrowHead::Cross);
    }

    #[test]
    fn parse_arrow_dotted_cross() {
        let mut input = "--xBob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Dotted);
        assert_eq!(a.head, ArrowHead::Cross);
    }

    #[test]
    fn parse_arrow_solid_open() {
        let mut input = "-)Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Solid);
        assert_eq!(a.head, ArrowHead::Open);
    }

    #[test]
    fn parse_arrow_dotted_open() {
        let mut input = "--)Bob";
        let a = arrow(&mut input).unwrap();
        assert_eq!(a.line_style, LineStyle::Dotted);
        assert_eq!(a.head, ArrowHead::Open);
    }

    // --- message ---

    #[test]
    fn parse_message_basic() {
        let mut input = "Alice->>Bob: Hello";
        let msg = message(&mut input).unwrap();
        assert_eq!(msg.from, "Alice");
        assert_eq!(msg.to, "Bob");
        assert_eq!(msg.arrow.line_style, LineStyle::Solid);
        assert_eq!(msg.arrow.head, ArrowHead::Arrowhead);
        assert_eq!(msg.text, "Hello");
    }

    #[test]
    fn parse_message_dotted_response() {
        let mut input = "Bob-->>Alice: Hi there!";
        let msg = message(&mut input).unwrap();
        assert_eq!(msg.from, "Bob");
        assert_eq!(msg.to, "Alice");
        assert_eq!(msg.arrow.line_style, LineStyle::Dotted);
        assert_eq!(msg.arrow.head, ArrowHead::Arrowhead);
        assert_eq!(msg.text, "Hi there!");
    }

    #[test]
    fn parse_message_with_spaces_around_colon() {
        let mut input = "Alice ->> Bob : Hello World";
        let msg = message(&mut input).unwrap();
        assert_eq!(msg.from, "Alice");
        assert_eq!(msg.to, "Bob");
        assert_eq!(msg.text, "Hello World");
    }

    // --- participant_decl ---

    #[test]
    fn parse_participant_without_alias() {
        let mut input = "participant Alice";
        let p = participant_decl(&mut input).unwrap();
        assert_eq!(p.id, "Alice");
        assert_eq!(p.alias, None);
    }

    #[test]
    fn parse_participant_with_alias() {
        let mut input = "participant A as Alice";
        let p = participant_decl(&mut input).unwrap();
        assert_eq!(p.id, "A");
        assert_eq!(p.alias, Some("Alice".to_string()));
    }

    // --- diagram ---

    #[test]
    fn parse_minimal_diagram() {
        let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi!\n";
        let diagram = parse_diagram(input).unwrap();
        assert_eq!(diagram.statements.len(), 2);
        match &diagram.statements[0] {
            Statement::Message(m) => {
                assert_eq!(m.from, "Alice");
                assert_eq!(m.to, "Bob");
                assert_eq!(m.text, "Hello");
            }
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn parse_diagram_with_participants() {
        let input = "\
sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hello
";
        let diagram = parse_diagram(input).unwrap();
        assert_eq!(diagram.statements.len(), 3);
        match &diagram.statements[0] {
            Statement::ParticipantDecl(p) => {
                assert_eq!(p.id, "A");
                assert_eq!(p.alias, Some("Alice".to_string()));
            }
            _ => panic!("expected ParticipantDecl"),
        }
    }

    #[test]
    fn parse_diagram_with_comments_and_blank_lines() {
        let input = "\
sequenceDiagram
    %% This is a comment
    Alice->>Bob: Hello

    Bob-->>Alice: Hi!
";
        let diagram = parse_diagram(input).unwrap();
        assert_eq!(diagram.statements.len(), 2);
    }

    // --- activate/deactivate ---

    #[test]
    fn parse_activate_statement() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    activate Bob
    Bob-->>Alice: Hi!
    deactivate Bob
";
        let diagram = parse_diagram(input).unwrap();
        assert_eq!(diagram.statements.len(), 4);
        assert_eq!(diagram.statements[1], Statement::Activate("Bob".to_string()));
        assert_eq!(diagram.statements[3], Statement::Deactivate("Bob".to_string()));
    }

    #[test]
    fn parse_message_with_activate_shorthand() {
        let mut input = "Alice->>+Bob: Hello";
        let msg = message(&mut input).unwrap();
        assert_eq!(msg.from, "Alice");
        assert_eq!(msg.to, "Bob");
        assert!(msg.activate_target);
        assert!(!msg.deactivate_source);
    }

    #[test]
    fn parse_message_with_deactivate_shorthand() {
        let mut input = "Bob-->>-Alice: Hi!";
        let msg = message(&mut input).unwrap();
        assert_eq!(msg.from, "Bob");
        assert_eq!(msg.to, "Alice");
        assert!(!msg.activate_target);
        assert!(msg.deactivate_source);
    }

    #[test]
    fn parse_message_without_modifiers() {
        let mut input = "Alice->>Bob: Hello";
        let msg = message(&mut input).unwrap();
        assert!(!msg.activate_target);
        assert!(!msg.deactivate_source);
    }

    // --- note ---

    #[test]
    fn parse_note_right_of() {
        let mut input = "Note right of Alice: This is a note";
        let n = note_stmt(&mut input).unwrap();
        assert_eq!(n.placement, NotePlacement::RightOf("Alice".to_string()));
        assert_eq!(n.text, "This is a note");
    }

    #[test]
    fn parse_note_left_of() {
        let mut input = "Note left of Bob: Left note";
        let n = note_stmt(&mut input).unwrap();
        assert_eq!(n.placement, NotePlacement::LeftOf("Bob".to_string()));
        assert_eq!(n.text, "Left note");
    }

    #[test]
    fn parse_note_over_single() {
        let mut input = "Note over Alice: Centered";
        let n = note_stmt(&mut input).unwrap();
        assert_eq!(n.placement, NotePlacement::Over("Alice".to_string()));
        assert_eq!(n.text, "Centered");
    }

    #[test]
    fn parse_note_over_two() {
        let mut input = "Note over Alice,Bob: Spanning note";
        let n = note_stmt(&mut input).unwrap();
        assert_eq!(
            n.placement,
            NotePlacement::OverTwo("Alice".to_string(), "Bob".to_string())
        );
        assert_eq!(n.text, "Spanning note");
    }

    #[test]
    fn parse_diagram_with_note() {
        let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note right of Bob: Got it!
    Bob-->>Alice: Hi!
";
        let diagram = parse_diagram(input).unwrap();
        assert_eq!(diagram.statements.len(), 3);
        match &diagram.statements[1] {
            Statement::Note(n) => {
                assert_eq!(n.placement, NotePlacement::RightOf("Bob".to_string()));
                assert_eq!(n.text, "Got it!");
            }
            other => panic!("expected Note, got {other:?}"),
        }
    }
}
