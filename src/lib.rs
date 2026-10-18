pub mod ast;
mod canvas;
pub mod class_parser;
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
pub mod state_parser;

pub const MAX_SOURCE_BYTES: usize = 1_048_576;
const MAX_CANVAS_CELLS: usize = 1_000_000;
const MAX_OUTPUT_BYTES: usize = 2_000_000;

pub fn render(input: &str) -> Result<String, String> {
    render_with_options(input, None)
}

pub fn render_with_options(input: &str, max_width: Option<usize>) -> Result<String, String> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(format!("source exceeds {MAX_SOURCE_BYTES}-byte limit"));
    }
    let input = strip_init_directives(input);
    let trimmed = input.trim_start();
    let output = if trimmed.starts_with("graph") || trimmed.starts_with("flowchart") {
        render_graph(graph_parser::parse_graph(input)?, max_width)
    } else if trimmed.starts_with("erDiagram") {
        let diagram = er_parser::parse_er(input)?;
        let computed = match max_width {
            Some(w) => er_layout::compute_with_max_width(&diagram, w)?,
            None => er_layout::compute(&diagram)?,
        };
        check_canvas_size(computed.width, computed.height)?;
        Ok(er_renderer::render(&computed))
    } else if trimmed.starts_with("sequenceDiagram") {
        let diagram = parser::parse_diagram(input)?;
        let computed = match max_width {
            Some(w) => layout::compute_with_max_width(&diagram, w)?,
            None => layout::compute(&diagram)?,
        };
        let (width, height) = renderer::dimensions(&computed);
        check_canvas_size(width, height)?;
        Ok(renderer::render(&computed))
    } else if trimmed.starts_with("stateDiagram-v2") || trimmed.starts_with("stateDiagram") {
        render_graph(state_parser::parse_state(input)?, max_width)
    } else if trimmed.starts_with("classDiagram") {
        render_graph(class_parser::parse_class(input)?, max_width)
    } else {
        let first_word = trimmed.split_whitespace().next().unwrap_or("(empty)");
        Err(format!("unknown diagram type: {first_word}"))
    }?;
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(format!("output exceeds {MAX_OUTPUT_BYTES}-byte limit"));
    }
    Ok(output)
}

fn render_graph(
    diagram: graph_ast::GraphDiagram,
    max_width: Option<usize>,
) -> Result<String, String> {
    let computed = match max_width {
        Some(width) => graph_layout::compute_with_max_width(&diagram, width)?,
        None => graph_layout::compute(&diagram)?,
    };
    check_canvas_size(computed.width, computed.height)?;
    Ok(graph_renderer::render(&computed))
}

fn check_canvas_size(width: usize, height: usize) -> Result<(), String> {
    if width
        .checked_mul(height)
        .is_none_or(|cells| cells > MAX_CANVAS_CELLS)
    {
        return Err(format!("canvas exceeds {MAX_CANVAS_CELLS}-cell limit"));
    }
    Ok(())
}

fn strip_init_directives(mut input: &str) -> &str {
    loop {
        let trimmed = input.trim_start();
        if !trimmed.starts_with("%%{init") {
            return trimmed;
        }
        let Some(end) = trimmed.find("}%%") else {
            return trimmed;
        };
        input = &trimmed[end + 3..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_unknown_diagram_type_returns_error() {
        let err = render("gantt\n  title Work\n").unwrap_err();
        assert!(
            err.contains("unknown diagram type"),
            "error should mention unknown diagram type, got: {err}"
        );
        assert!(
            err.contains("gantt"),
            "error should include the type, got: {err}"
        );
    }

    #[test]
    fn render_empty_input_returns_error() {
        let err = render("").unwrap_err();
        assert!(err.contains("unknown diagram type"), "got: {err}");
    }

    #[test]
    fn render_rejects_oversized_source() {
        let input = format!("graph TD\nA --> B\n%%{}", "x".repeat(1_048_576));
        let Err(err) = render(&input) else {
            panic!("oversized source was accepted");
        };
        assert!(err.contains("source") && err.contains("limit"), "{err}");
    }

    #[test]
    fn render_rejects_oversized_canvas() {
        let input = format!("graph TD\nA[{}]\n", "x".repeat(350_000));
        let Err(err) = render(&input) else {
            panic!("oversized canvas was accepted");
        };
        assert!(err.contains("canvas") && err.contains("limit"), "{err}");
    }

    #[test]
    fn render_rejects_oversized_output() {
        let input = format!("graph TD\nA[{}]\n", "x".repeat(300_000));
        let Err(err) = render(&input) else {
            panic!("oversized output was accepted");
        };
        assert!(err.contains("output") && err.contains("limit"), "{err}");
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
    fn render_preserves_wide_subgraph_titles_and_frame_width() {
        let output =
            render("flowchart LR\nsubgraph before[\"変更前\"]\nA[setup] --> B[build-go]\nend\n")
                .unwrap();
        let lines: Vec<_> = output.lines().collect();
        assert!(lines[0].contains("変更前"), "{output}");
        assert_eq!(
            display_width::display_width(lines[0]),
            display_width::display_width(lines.last().unwrap()),
            "{output}"
        );
    }

    #[test]
    fn render_er_diagram_works() {
        let output = render("erDiagram\n    A ||--o{ B : has\n").unwrap();
        assert!(output.contains("A"));
    }

    #[test]
    fn render_ignores_init_directive_before_diagram() {
        let output =
            render("%%{init: {'theme': 'neutral'}}%%\nsequenceDiagram\n    A->>B: Hi\n").unwrap();
        assert!(output.contains("Hi"));
    }
}
