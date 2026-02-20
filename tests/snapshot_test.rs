use pretty_assertions::assert_eq;

#[test]
fn snapshot_two_participants() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Bob-->>Alice: Hi!
";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───────┐  ┌─────┐
│ Alice │  │ Bob │
└───┬───┘  └──┬──┘
    │ Hello   │
    │────────>│
    │         │
    │ Hi!     │
    │< ─ ─ ─ ─│
    │         │
┌───┴───┐  ┌──┴──┐
│ Alice │  │ Bob │
└───────┘  └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn snapshot_three_participants() {
    let input = "\
sequenceDiagram
    participant A as Alice
    participant B as Bob
    participant C as Charlie
    A->>B: Hello Bob!
    B->>C: Hey Charlie
    C-->>A: Hi everyone!
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Alice"));
    assert!(output.contains("Bob"));
    assert!(output.contains("Charlie"));
    assert!(output.contains("Hello Bob!"));
    assert!(output.contains("Hey Charlie"));
    assert!(output.contains("Hi everyone!"));

    let alice_count = output.matches("Alice").count();
    assert_eq!(alice_count, 2, "Alice appears in top and bottom boxes");
}

#[test]
fn snapshot_all_arrow_types() {
    let input = "\
sequenceDiagram
    A->B: solid none
    A->>B: solid arrowhead
    A-xB: solid cross
    A-)B: solid open
    B-->A: dotted none
    B-->>A: dotted arrowhead
    B--xA: dotted cross
    B--)A: dotted open
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("solid none"));
    assert!(output.contains("solid arrowhead"));
    assert!(output.contains("solid cross"));
    assert!(output.contains("solid open"));
    assert!(output.contains("dotted none"));
    assert!(output.contains("dotted arrowhead"));
    assert!(output.contains("dotted cross"));
    assert!(output.contains("dotted open"));
}

#[test]
fn snapshot_activation_shorthand() {
    let input = "\
sequenceDiagram
    Alice->>+Bob: Hello
    Bob-->>-Alice: Hi!
";
    let output = ma::render(input).unwrap();
    let expected = "\
┌───────┐  ┌─────┐
│ Alice │  │ Bob │
└───┬───┘  └──┬──┘
    │ Hello   ┃
    │────────>┃
    │         ┃
    │ Hi!     ┃
    │< ─ ─ ─ ─┃
    │         ┃
┌───┴───┐  ┌──┴──┐
│ Alice │  │ Bob │
└───────┘  └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn snapshot_activation_explicit() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    activate Bob
    Bob-->>Alice: Hi!
    deactivate Bob
";
    let output = ma::render(input).unwrap();

    assert!(output.contains('┃'), "active lifeline should use heavy vertical");

    let lines: Vec<&str> = output.lines().collect();
    // First message (Hello) - Bob not yet active
    assert!(lines[3].contains("Hello"));
    assert!(!lines[3].contains('┃'), "Bob not active during Hello");
    // Second message (Hi!) - Bob active
    assert!(lines[6].contains("Hi!"));
    assert!(lines[6].contains('┃'), "Bob active during Hi!");
}
