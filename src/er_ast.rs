#[derive(Debug, Clone, PartialEq)]
pub struct ErDiagram {
    pub entities: Vec<String>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub label: String,
}
