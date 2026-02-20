pub mod ast;
pub mod layout;
pub mod parser;
pub mod renderer;

pub fn render(input: &str) -> Result<String, String> {
    let diagram = parser::parse_diagram(input)?;
    let layout = layout::compute(&diagram)?;
    Ok(renderer::render(&layout))
}
