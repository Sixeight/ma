use pretty_assertions::assert_eq;

#[test]
fn spec_er_single_relationship() {
    let input = "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n";
    let output = ma::render(input).unwrap();
    let expected = "\
┌──────────┐              ┌───────┐
│ CUSTOMER │||──places──o{│ ORDER │
└──────────┘              └───────┘";
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
    assert!(output.contains("}o"), "should contain left ZeroOrMany symbol");
    assert!(output.contains("||"), "should contain right ExactlyOne symbol");
}

#[test]
fn spec_er_cardinality_symbols_all_variants() {
    let input = "erDiagram\n    A o|--|o B : rel\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("o|"), "should contain left ZeroOrOne symbol");
    assert!(output.contains("|o"), "should contain right ZeroOrOne symbol");
}

#[test]
fn spec_er_cardinality_one_or_many() {
    let input = "erDiagram\n    A }|--|{ B : rel\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("}|"), "should contain left OneOrMany symbol");
    assert!(output.contains("|{"), "should contain right OneOrMany symbol");
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

// =============================================================================
// Entity Attributes
// =============================================================================

#[test]
fn spec_er_entity_with_attributes() {
    let input = "\
erDiagram
    CUSTOMER {
        string name
        int age
    }
    CUSTOMER ||--o{ ORDER : places
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("CUSTOMER"), "entity name visible");
    assert!(output.contains("name"), "attribute name visible");
    assert!(output.contains("age"), "second attribute visible");
    assert!(output.contains("string"), "attribute type visible");
    assert!(output.contains("int"), "second attribute type visible");
    assert!(output.contains("places"), "relationship label visible");
}

#[test]
fn spec_er_entity_attributes_box_structure() {
    let input = "\
erDiagram
    CUSTOMER {
        string name
        int age
    }
";
    let output = ma::render(input).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    // Entity name on first content row
    assert!(lines[1].contains("CUSTOMER"), "entity name in header");

    // Separator line between name and attributes
    let separator = lines.iter().find(|l| l.contains('├') && l.contains('┤'));
    assert!(separator.is_some(), "separator between name and attributes");

    // Attributes below separator
    let name_line = lines.iter().find(|l| l.contains("string") && l.contains("name"));
    assert!(name_line.is_some(), "attribute 'string name' visible");
    let age_line = lines.iter().find(|l| l.contains("int") && l.contains("age"));
    assert!(age_line.is_some(), "attribute 'int age' visible");
}

#[test]
fn spec_er_entity_attribute_with_key() {
    let input = "\
erDiagram
    CUSTOMER {
        int id PK
        string name
    }
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("PK"), "PK marker visible");
    assert!(output.contains("id"), "key attribute name visible");
}

#[test]
fn spec_er_entity_no_attributes_unchanged() {
    let with_attrs = ma::render("erDiagram\n    A ||--|| B : rel\n").unwrap();
    // Entities without attribute blocks should render the same as before
    assert!(with_attrs.contains("│ A │"), "simple entity unchanged");
    assert!(with_attrs.contains("│ B │"), "simple entity unchanged");
}

#[test]
fn spec_er_mixed_entities() {
    let input = "\
erDiagram
    CUSTOMER {
        int id PK
        string name
    }
    ORDER ||--|{ LINE-ITEM : contains
    CUSTOMER ||--o{ ORDER : places
";
    let output = ma::render(input).unwrap();
    assert!(output.contains("CUSTOMER"), "entity with attrs");
    assert!(output.contains("ORDER"), "entity without attrs");
    assert!(output.contains("LINE-ITEM"), "entity without attrs");
    assert!(output.contains("id"), "attribute visible");
    assert!(output.contains("places"), "relationship label visible");
}
