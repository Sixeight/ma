use pretty_assertions::assert_eq;

#[test]
fn spec_er_single_relationship() {
    let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌──────────┐          ┌───────┐
│ CUSTOMER │──places──│ ORDER │
└──────────┘          └───────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_er_chain() {
    let input = "\
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("CUSTOMER"));
    assert!(output.contains("ORDER"));
    assert!(output.contains("LINE-ITEM"));
    assert!(output.contains("places"));
    assert!(output.contains("contains"));
}

#[test]
fn spec_er_entity_with_hyphen() {
    let input = "erDiagram\n    ORDER ||--|{ LINE-ITEM : contains\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("LINE-ITEM"));
}

#[test]
fn spec_er_label_with_spaces() {
    let input = "erDiagram\n    CUSTOMER }o--|| ADDRESS : billing address\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("billing address"));
}

#[test]
fn spec_er_empty_diagram() {
    let input = "erDiagram\n";
    let result = ma::render(input);
    assert!(result.is_err());
}

#[test]
fn spec_er_does_not_break_graph() {
    let input = "graph TD\n    A --> B\n";
    let result = ma::render(input);
    assert!(result.is_ok());
}

#[test]
fn spec_er_does_not_break_sequence() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello\n";
    let result = ma::render(input);
    assert!(result.is_ok());
}
