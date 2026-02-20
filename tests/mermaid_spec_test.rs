use pretty_assertions::assert_eq;

// =============================================================================
// Implemented features — these tests MUST pass
// =============================================================================

#[test]
fn spec_participant_implicit() {
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
fn spec_participant_explicit_alias() {
    let input = "\
sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hello
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Alice"), "should display alias Alice");
    assert!(output.contains("Bob"), "should display alias Bob");
    assert!(!output.contains("│ A │"), "should not display raw id A");
    assert!(output.contains("Hello"));
}

#[test]
fn spec_arrow_all_types() {
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
fn spec_activation_shorthand() {
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
fn spec_activation_explicit() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    activate Bob
    Bob-->>Alice: Hi!
    deactivate Bob
";
    let output = ma::render(input).unwrap();

    let lines: Vec<&str> = output.lines().collect();
    assert!(!lines[3].contains('┃'), "Bob not active during Hello");
    assert!(lines[6].contains('┃'), "Bob active during Hi!");
}

#[test]
fn spec_note_right_of() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note right of Bob: Got it!
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
    │         │ ┌─────────┐
    │         │ │ Got it! │
    │         │ └─────────┘
    │ Hi!     │
    │< ─ ─ ─ ─│
    │         │
┌───┴───┐  ┌──┴──┐
│ Alice │  │ Bob │
└───────┘  └─────┘";
    assert_eq!(output, expected);
}

#[test]
fn spec_note_over_two() {
    let input = "\
sequenceDiagram
    Alice->>Bob: Hello
    Note over Alice,Bob: Shared
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Shared"));
    let lines: Vec<&str> = output.lines().collect();
    let note_line = lines.iter().find(|l| l.contains("Shared")).unwrap();
    assert!(
        note_line.contains("│ Shared"),
        "note text should be in a box: {note_line}"
    );
}

#[test]
fn spec_comment_ignored() {
    let input = "\
sequenceDiagram
    %% This comment should not appear
    Alice->>Bob: Hello
";
    let output = ma::render(input).unwrap();

    assert!(!output.contains("comment"), "comments must not appear in output");
    assert!(output.contains("Hello"));
}

// =============================================================================
// Unimplemented features — #[ignore] until implemented
// =============================================================================

// --- loop ---

#[test]
fn spec_loop() {
    let input = "\
sequenceDiagram
    loop Check
        A->>B: Ping
    end
";
    let output = ma::render(input).unwrap();

    // Frame structure
    assert!(output.contains("loop Check"), "frame label visible");
    assert!(output.contains("Ping"), "message inside loop visible");
    assert!(output.contains('┼'), "lifeline-frame intersection");

    // Frame characters
    let lines: Vec<&str> = output.lines().collect();
    let frame_top = lines.iter().find(|l| l.contains("loop Check")).unwrap();
    assert!(frame_top.contains('┌'), "frame top-left corner");
    assert!(frame_top.contains('┐'), "frame top-right corner");

    let frame_bottom = lines[3..].iter().find(|l| l.contains('└') && l.contains('┘')).unwrap();
    assert!(frame_bottom.contains('┼'), "frame bottom has lifeline intersection");
}

#[test]
fn spec_loop_with_surrounding_messages() {
    let input = "\
sequenceDiagram
    A->>B: Hello
    loop Check
        A->>B: Ping
    end
    B-->>A: Bye
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Hello"), "message before loop");
    assert!(output.contains("loop Check"), "loop label");
    assert!(output.contains("Ping"), "message inside loop");
    assert!(output.contains("Bye"), "message after loop");

    // Verify ordering: Hello appears before loop, Ping inside, Bye after
    let hello_pos = output.find("Hello").unwrap();
    let loop_pos = output.find("loop Check").unwrap();
    let ping_pos = output.find("Ping").unwrap();
    let bye_pos = output.find("Bye").unwrap();
    assert!(hello_pos < loop_pos, "Hello before loop");
    assert!(loop_pos < ping_pos, "loop label before Ping");
    assert!(ping_pos < bye_pos, "Ping before Bye");

    // Frame borders visible on message rows inside loop
    let lines: Vec<&str> = output.lines().collect();
    let ping_line = lines.iter().find(|l| l.contains("Ping")).unwrap();
    assert!(
        ping_line.starts_with("│"),
        "frame left border on message row: {ping_line}"
    );
}

// --- alt / else ---

#[test]
fn spec_alt_else() {
    let input = "\
sequenceDiagram
    alt Happy
        A->>B: Yes
    else Sad
        A->>B: No
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("alt Happy"), "alt label visible");
    assert!(output.contains("else Sad"), "else label visible");
    assert!(output.contains("Yes"), "first branch message");
    assert!(output.contains("No"), "second branch message");

    // Frame structure
    let lines: Vec<&str> = output.lines().collect();

    let alt_line = lines.iter().find(|l| l.contains("alt Happy")).unwrap();
    assert!(alt_line.contains('┌'), "alt frame top-left");
    assert!(alt_line.contains('┐'), "alt frame top-right");

    let else_line = lines.iter().find(|l| l.contains("else Sad")).unwrap();
    assert!(else_line.contains('├'), "else divider left");
    assert!(else_line.contains('┤'), "else divider right");

    let body = &lines[3..lines.len() - 3];
    let frame_bottom = body
        .iter()
        .rev()
        .find(|l| l.contains('└') && l.contains('┘'))
        .unwrap();
    assert!(frame_bottom.contains('┼'), "frame bottom lifeline intersection");
}

// --- opt ---

#[test]
fn spec_opt() {
    let input = "\
sequenceDiagram
    opt Maybe
        A->>B: Perhaps
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("opt Maybe"), "opt label visible");
    assert!(output.contains("Perhaps"), "message inside opt");

    let lines: Vec<&str> = output.lines().collect();
    let opt_line = lines.iter().find(|l| l.contains("opt Maybe")).unwrap();
    assert!(opt_line.contains('┌'), "opt frame top-left");
    assert!(opt_line.contains('┐'), "opt frame top-right");
}

// --- actor ---

#[test]
#[ignore = "actor keyword not yet implemented"]
fn spec_actor_as_participant() {
    let input = "\
sequenceDiagram
    actor A as Alice
    actor B as Bob
    A->>B: Hello
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Alice"), "actor displayed as participant");
    assert!(output.contains("Bob"));
    assert!(output.contains("Hello"));
}

// --- autonumber ---

#[test]
#[ignore = "autonumber not yet implemented"]
fn spec_autonumber() {
    let input = "\
sequenceDiagram
    autonumber
    Alice->>Bob: Hello
    Bob-->>Alice: Hi!
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("1."), "first message numbered");
    assert!(output.contains("2."), "second message numbered");
}

// --- nested blocks ---

#[test]
fn spec_nested_loop() {
    let input = "\
sequenceDiagram
    loop Outer
        A->>B: Ping
        loop Inner
            B-->>A: Pong
        end
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("loop Outer"), "outer loop label");
    assert!(output.contains("loop Inner"), "inner loop label");
    assert!(output.contains("Ping"));
    assert!(output.contains("Pong"));
}

// --- par ---

#[test]
fn spec_par() {
    let input = "\
sequenceDiagram
    par Task1
        A->>B: Do X
    and Task2
        A->>B: Do Y
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("par Task1"), "par label");
    assert!(output.contains("Task2"), "and label");
    assert!(output.contains("Do X"));
    assert!(output.contains("Do Y"));
}

// --- critical ---

#[test]
fn spec_critical() {
    let input = "\
sequenceDiagram
    critical Setup
        A->>B: Init
    option Fallback
        A->>B: Retry
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("critical Setup"), "critical label");
    assert!(output.contains("Fallback"), "option label");
    assert!(output.contains("Init"));
    assert!(output.contains("Retry"));
}

// --- break ---

#[test]
fn spec_break() {
    let input = "\
sequenceDiagram
    A->>B: Hello
    break Error
        B-->>A: Fail
    end
";
    let output = ma::render(input).unwrap();

    assert!(output.contains("break Error"), "break label");
    assert!(output.contains("Fail"));
}
