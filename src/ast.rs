#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    ParticipantDecl(ParticipantDecl),
    Message(Message),
    Activate(String),
    Deactivate(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantDecl {
    pub id: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub arrow: Arrow,
    pub text: String,
    pub activate_target: bool,
    pub deactivate_source: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow {
    pub line_style: LineStyle,
    pub head: ArrowHead,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    Solid,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowHead {
    None,
    Arrowhead,
    Cross,
    Open,
}
