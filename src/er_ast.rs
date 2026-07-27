#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMany,
    ZeroOrMany,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityAttribute {
    pub attr_type: String,
    pub name: String,
    pub key: Option<String>,
    pub comment: Option<String>,
}

impl EntityAttribute {
    pub fn display_text(&self) -> String {
        let mut text = format!("{} {}", self.attr_type, self.name);
        if let Some(key) = &self.key {
            text.push(' ');
            text.push_str(key);
        }
        if let Some(comment) = &self.comment {
            text.push_str(" \"");
            text.push_str(comment);
            text.push('"');
        }
        text
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub name: String,
    pub alias: Option<String>,
    pub attributes: Vec<EntityAttribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErDiagram {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub left_card: Cardinality,
    pub right_card: Cardinality,
    pub line_style: RelationshipLineStyle,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelationshipLineStyle {
    Identifying,
    NonIdentifying,
}
