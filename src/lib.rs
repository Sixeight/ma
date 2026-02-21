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
    let trimmed = input.trim_start();
    if trimmed.starts_with("graph") || trimmed.starts_with("flowchart") {
        let diagram = graph_parser::parse_graph(input)?;
        let layout = graph_layout::compute(&diagram)?;
        Ok(graph_renderer::render(&layout))
    } else if trimmed.starts_with("erDiagram") {
        let diagram = er_parser::parse_er(input)?;
        let layout = er_layout::compute(&diagram)?;
        Ok(er_renderer::render(&layout))
    } else {
        let diagram = parser::parse_diagram(input)?;
        let layout = layout::compute(&diagram)?;
        Ok(renderer::render(&layout))
    }
}
