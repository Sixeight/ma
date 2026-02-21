#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMany,
    ZeroOrMany,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErDiagram {
    pub entities: Vec<String>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub left_card: Cardinality,
    pub right_card: Cardinality,
    pub label: String,
}
