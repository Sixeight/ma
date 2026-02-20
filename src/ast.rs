#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    ParticipantDecl(ParticipantDecl),
    Message(Message),
    Note(Note),
    Activate(String),
    Deactivate(String),
    Loop(LoopBlock),
    Alt(AltBlock),
    Opt(LoopBlock),
    Break(LoopBlock),
    Par(AltBlock),
    Critical(AltBlock),
    AutoNumber,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopBlock {
    pub label: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AltBlock {
    pub label: String,
    pub body: Vec<Statement>,
    pub else_branches: Vec<ElseBranch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseBranch {
    pub label: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub placement: NotePlacement,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotePlacement {
    RightOf(String),
    LeftOf(String),
    Over(String),
    OverTwo(String, String),
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
