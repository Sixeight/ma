#[test]
fn spec_class_diagram_renders_types_members_and_relationships() {
    let input = r#"classDiagram
        direction LR
        class PullRequest {
            +String title
            +merge()
        }
        class Review {
            +String status
            +approve()
        }
        PullRequest "1" *-- "many" Review : contains
    "#;

    let output = ma::render(input).unwrap();
    assert!(output.contains("PullRequest"), "{output}");
    assert!(output.contains("+String title"), "{output}");
    assert!(output.contains("+merge()"), "{output}");
    assert!(output.contains("Review"), "{output}");
    assert!(output.contains("contains"), "{output}");
    assert!(output.contains("composition"), "{output}");
}

#[test]
fn spec_class_diagram_supports_colon_members_and_inheritance() {
    let input = r#"classDiagram
        class Base
        Base : +render()
        class Markdown
        Base <|-- Markdown : extends
    "#;

    let output = ma::render(input).unwrap();
    assert!(output.contains("+render()"), "{output}");
    assert!(output.contains("extends"), "{output}");
    let markdown = output.find("Markdown").unwrap();
    let base = output.find("Base").unwrap();
    assert!(
        markdown < base,
        "inheritance arrow should run from child to base: {output}"
    );
}

#[test]
fn spec_class_diagram_rejects_namespace() {
    let error = ma::render("classDiagram\n    namespace Domain {\n    }\n").unwrap_err();
    assert!(error.contains("namespace"), "{error}");
}

#[test]
fn spec_class_diagram_supports_reverse_composition() {
    let output = ma::render("classDiagram\n    Part --* Whole : belongs to\n").unwrap();
    assert!(output.contains("composition: belongs to"), "{output}");
    let part = output.find("Part").unwrap();
    let whole = output.find("Whole").unwrap();
    assert!(
        whole < part,
        "composition points from Whole to Part: {output}"
    );
}

#[test]
fn spec_class_diagram_rejects_bidirectional_relations_instead_of_misrendering() {
    let error = ma::render("classDiagram\n    A <|--|> B\n").unwrap_err();
    assert!(error.contains("bidirectional"), "{error}");
}

#[test]
fn spec_class_diagram_supports_inline_and_separate_annotations() {
    let input = "classDiagram\n    class Shape <<interface>>\n    <<service>> Renderer\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("<<interface>>"), "{output}");
    assert!(output.contains("<<service>>"), "{output}");
    assert!(output.contains("Shape"), "{output}");
    assert!(output.contains("Renderer"), "{output}");
}

#[test]
fn spec_class_relationship_label_may_contain_another_relation_token() {
    let output = ma::render("classDiagram\n    A -- B : displays --> literally\n").unwrap();
    assert!(output.contains("displays --> literally"), "{output}");
}

#[test]
fn spec_class_rejects_empty_annotation() {
    let error = ma::render("classDiagram\n    <<>> Empty\n").unwrap_err();
    assert!(error.contains("empty class annotation"), "{error}");
}

#[test]
fn spec_class_relationship_tokens_keep_direction_and_edge_style() {
    use ma::graph_ast::EdgeType;

    let diagram = ma::class_parser::parse_class(
        "classDiagram\n    Parent <|-- Child\n    Client ..> Service\n    Part --o Whole\n",
    )
    .unwrap();
    assert_eq!(diagram.edges[0].from, "Child");
    assert_eq!(diagram.edges[0].to, "Parent");
    assert_eq!(diagram.edges[0].edge_type, EdgeType::Arrow);
    assert_eq!(diagram.edges[1].from, "Client");
    assert_eq!(diagram.edges[1].to, "Service");
    assert_eq!(diagram.edges[1].edge_type, EdgeType::DottedArrow);
    assert_eq!(diagram.edges[2].from, "Whole");
    assert_eq!(diagram.edges[2].to, "Part");
    assert_eq!(diagram.edges[2].edge_type, EdgeType::OpenLink);
}

#[test]
fn spec_class_alias_annotation_and_implicit_relationship_node_compose() {
    let input = "classDiagram\n    Service --> Port : uses\n    class Service[\"Application Service\"] <<service>>\n    Service : +run()\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("Application Service"), "{output}");
    assert!(output.contains("<<service>>"), "{output}");
    assert!(output.contains("+run()"), "{output}");
    assert!(output.contains("uses"), "{output}");
}

#[test]
fn spec_class_rejects_unclosed_block() {
    let error = ma::render("classDiagram\n    class Open {\n        +field\n").unwrap_err();
    assert!(error.contains("unclosed class block"), "{error}");
}

#[test]
fn spec_class_init_directive_dispatches_to_class_parser() {
    let input = "%%{init: {'theme': 'neutral'}}%%\nclassDiagram\n    A --> B\n";
    let output = ma::render(input).unwrap();
    assert!(output.contains("A") && output.contains("B"), "{output}");
}
