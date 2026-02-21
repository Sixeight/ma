pub mod ast;
pub mod display_width;
pub mod er_ast;
pub mod er_layout;
pub mod er_parser;
pub mod er_renderer;
pub mod graph_ast;
pub mod graph_layout;
pub mod graph_parser;
pub mod graph_renderer;
pub mod layout;
pub mod parser;
pub mod renderer;

pub fn render(input: &str) -> Result<String, String> {
    render_with_options(input, None)
}

pub fn render_with_options(input: &str, max_width: Option<usize>) -> Result<String, String> {
    let trimmed = input.trim_start();
    if trimmed.starts_with("graph") || trimmed.starts_with("flowchart") {
        let diagram = graph_parser::parse_graph(input)?;
        let computed = match max_width {
            Some(w) => graph_layout::compute_with_max_width(&diagram, w)?,
            None => graph_layout::compute(&diagram)?,
        };
        Ok(graph_renderer::render(&computed))
    } else if trimmed.starts_with("erDiagram") {
        let diagram = er_parser::parse_er(input)?;
        let computed = match max_width {
            Some(w) => er_layout::compute_with_max_width(&diagram, w)?,
            None => er_layout::compute(&diagram)?,
        };
        Ok(er_renderer::render(&computed))
    } else if trimmed.starts_with("sequenceDiagram") {
        let diagram = parser::parse_diagram(input)?;
        let computed = match max_width {
            Some(w) => layout::compute_with_max_width(&diagram, w)?,
            None => layout::compute(&diagram)?,
        };
        Ok(renderer::render(&computed))
    } else {
        let first_word = trimmed.split_whitespace().next().unwrap_or("(empty)");
        Err(format!("unknown diagram type: {first_word}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_unknown_diagram_type_returns_error() {
        let err = render("classDiagram\n  Foo\n").unwrap_err();
        assert!(
            err.contains("unknown diagram type"),
            "error should mention unknown diagram type, got: {err}"
        );
        assert!(err.contains("classDiagram"), "error should include the type, got: {err}");
    }

    #[test]
    fn render_empty_input_returns_error() {
        let err = render("").unwrap_err();
        assert!(err.contains("unknown diagram type"), "got: {err}");
    }

    #[test]
    fn render_sequence_diagram_works() {
        let output = render("sequenceDiagram\n    Alice->>Bob: Hello\n").unwrap();
        assert!(output.contains("Alice"));
    }

    #[test]
    fn render_graph_diagram_works() {
        let output = render("graph TD\n    A --> B\n").unwrap();
        assert!(output.contains("A"));
    }

    #[test]
    fn render_er_diagram_works() {
        let output = render("erDiagram\n    A ||--o{ B : has\n").unwrap();
        assert!(output.contains("A"));
    }
}
