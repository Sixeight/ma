#[test]
fn spec_state_diagram_renders_pr_workflow() {
    let input = r#"stateDiagram-v2
        direction LR
        [*] --> Draft
        state "In review" as Review
        Draft --> Review : open PR
        Review --> Merged : approve
        Review --> Draft : request changes
        Merged --> [*]
    "#;

    let output = ma::render(input).unwrap();
    assert!(output.contains("Draft"), "{output}");
    assert!(output.contains("In review"), "{output}");
    assert!(output.contains("Merged"), "{output}");
    assert!(output.contains("open PR"), "{output}");
    assert!(output.contains("request changes"), "{output}");
    assert!(
        output.contains('●'),
        "start marker should be visible: {output}"
    );
    assert!(
        output.contains('◉'),
        "end marker should be visible: {output}"
    );
}

#[test]
fn spec_state_diagram_supports_inline_descriptions_and_lr_direction() {
    let input = "stateDiagram-v2\n    direction LR\n    idle : Waiting for CI\n    idle --> done\n";
    let output = ma::render(input).unwrap();

    assert!(output.contains("Waiting for CI"), "{output}");
    assert!(
        output.contains('>'),
        "LR transition should point right: {output}"
    );
}

#[test]
fn spec_state_diagram_rejects_unsupported_composite_state() {
    let error =
        ma::render("stateDiagram-v2\n    state Parent {\n        Child\n    }\n").unwrap_err();
    assert!(error.contains("composite state"), "{error}");
}

#[test]
fn spec_state_inline_description_may_contain_transition_token() {
    let output = ma::render("stateDiagram-v2\n    Ready : waiting --> running\n").unwrap();
    assert!(output.contains("waiting --> running"), "{output}");
}

#[test]
fn spec_state_alias_preserves_as_inside_quoted_label() {
    let input =
        "stateDiagram-v2\n    state \"Waiting as requested\" as Waiting\n    Waiting --> Done\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("Waiting as requested"), "{output}");
}

#[test]
fn spec_state_rejects_alias_without_as_identifier() {
    let error = ma::render("stateDiagram-v2\n    state \"Missing identifier\"\n").unwrap_err();
    assert!(error.contains("invalid state declaration"), "{error}");
}

#[test]
fn spec_state_td_transition_does_not_truncate_wide_unicode_label() {
    let label = "承認待ちのトランジションラベル";
    let input = format!("stateDiagram-v2\n    [*] --> Review : {label}\n");
    let output = ma::render(&input).unwrap();
    assert!(output.contains(label), "{output}");
}

#[test]
fn spec_state_init_directive_dispatches_to_state_parser() {
    let input = "%%{init: {'theme': 'neutral'}}%%\nstateDiagram-v2\n    A --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("A") && output.contains("B"), "{output}");
}
